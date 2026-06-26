use egui::{self, Vec2};

use crate::canvas::{self, CanvasState};
use crate::config::{AppConfig, ThemePreference};
use crate::history::History;
use crate::i18n::{self, Lang};
use crate::panels;
use crate::path_editor;
use crate::svg_doc::SvgDoc;

/// The main application state.
pub struct SvgEditorApp {
    doc: Option<SvgDoc>,
    canvas_state: CanvasState,
    status_msg: String,
    error_msg: Option<String>,
    config: AppConfig,
    lang: Lang,
    dpi_initialized: bool,
    texture: Option<egui::TextureHandle>,
    history: Option<History>,
    was_dragging: bool,
}

impl SvgEditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();

        // Apply theme from config
        cc.egui_ctx.set_theme(config.theme.to_egui());

        // Determine language
        let lang = Lang::from_code(&config.language);

        // Apply DPI if explicitly set in config
        if config.dpi > 0.0 {
            cc.egui_ctx.set_pixels_per_point(config.dpi);
        }
        // If dpi == 0, we'll auto-detect on first frame

        let mut canvas_state = CanvasState::default();

        // Load system font list for text input
        canvas_state.font_list = load_system_font_families();

        Self {
            doc: None,
            canvas_state,
            status_msg: i18n::t("status.ready", &lang).to_string(),
            error_msg: None,
            config,
            lang,
            dpi_initialized: false,
            texture: None,
            history: None,
            was_dragging: false,
        }
    }

    fn load_file(&mut self, path: &std::path::Path) {
        match std::fs::read(path) {
            Ok(bytes) => match SvgDoc::from_bytes(&bytes, Some(path.to_path_buf())) {
                Ok(doc) => {
                    self.status_msg = format!(
                        "{}: {} ({} {})",
                        i18n::t("status.loaded", &self.lang),
                        path.display(),
                        doc.paths.len(),
                        i18n::t("paths.heading", &self.lang).to_lowercase(),
                    );
                    let saved_font_list = std::mem::take(&mut self.canvas_state.font_list);
                    self.canvas_state = CanvasState::default();
                    self.canvas_state.font_list = saved_font_list;
                    self.history = Some(History::new(doc.clone()));
                    self.doc = Some(doc);
                    self.texture = None;
                    self.error_msg = None;
                }
                Err(e) => {
                    self.error_msg = Some(format!("{}: {}", i18n::t("status.parse_error", &self.lang), e));
                    self.status_msg = i18n::t("status.failed_load", &self.lang).to_string();
                }
            },
            Err(e) => {
                self.error_msg = Some(format!("{}: {}", i18n::t("status.read_error", &self.lang), e));
                self.status_msg = i18n::t("status.failed_read", &self.lang).to_string();
            }
        }
    }

    fn save_file_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG", &["svg"])
            .save_file()
        {
            if let Some(ref doc) = self.doc {
                match doc.save(&path) {
                    Ok(()) => {
                        self.status_msg = format!("{}: {}", i18n::t("status.saved", &self.lang), path.display());
                        self.error_msg = None;
                    }
                    Err(e) => {
                        self.error_msg = Some(format!("{}: {}", i18n::t("status.save_error", &self.lang), e));
                    }
                }
            }
        }
    }

    fn save_file_direct(&mut self) {
        if let Some(ref doc) = self.doc {
            if let Some(ref path) = doc.file_path {
                match doc.save(path) {
                    Ok(()) => {
                        self.status_msg = format!("{}: {}", i18n::t("status.saved", &self.lang), path.display());
                        self.error_msg = None;
                    }
                    Err(e) => {
                        self.error_msg = Some(format!("{}: {}", i18n::t("status.save_error", &self.lang), e));
                    }
                }
            } else {
                self.save_file_as();
            }
        }
    }

    fn export_as(&mut self, format: &str) {
        let (filter_name, ext) = match format {
            "png" => ("PNG", "png"),
            "jpg" => ("JPG", "jpg"),
            _ => return,
        };

        // Default filename: timestamp + extension
        let default_name = chrono::Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();

        if let Some(path) = rfd::FileDialog::new()
            .add_filter(filter_name, &[ext])
            .set_file_name(format!("{}.{}", default_name, ext))
            .save_file()
        {
            if let Some(ref doc) = self.doc {
                let result = match format {
                    "png" => doc.export_png(&path),
                    "jpg" => doc.export_jpg(&path),
                    _ => return,
                };
                match result {
                    Ok(()) => {
                        self.status_msg =
                            format!("{}: {}", i18n::t("status.saved", &self.lang), path.display());
                        self.error_msg = None;
                    }
                    Err(e) => {
                        self.error_msg =
                            Some(format!("{}: {}", i18n::t("status.save_error", &self.lang), e));
                    }
                }
            }
        }
    }

    /// Save current doc state to history.
    fn save_history(&mut self) {
        if let (Some(ref doc), Some(ref mut hist)) = (&self.doc, &mut self.history) {
            hist.push(doc.clone());
        }
    }

    /// Perform undo.
    fn undo(&mut self) {
        if let Some(ref mut hist) = self.history {
            if let Some(prev) = hist.undo() {
                self.doc = Some(prev.clone());
                self.texture = None;
                self.status_msg = i18n::t("status.undo", &self.lang).to_string();
            }
        }
    }

    /// Perform redo.
    fn redo(&mut self) {
        if let Some(ref mut hist) = self.history {
            if let Some(next) = hist.redo() {
                self.doc = Some(next.clone());
                self.texture = None;
                self.status_msg = i18n::t("status.redo", &self.lang).to_string();
            }
        }
    }

    fn handle_drag(&mut self, ui: &egui::Ui) {
        let has_cp_drag = self.canvas_state.dragging.is_some();
        let has_multi_select = self.canvas_state.selected_paths.len() > 1;
        let pointer_down = ui.input(|i| i.pointer.primary_down());
        let is_dragging = has_cp_drag || (has_multi_select && pointer_down);

        // Save history when drag starts
        if is_dragging && !self.was_dragging {
            self.save_history();
        }
        self.was_dragging = is_dragging;

        if is_dragging {
            let delta = ui.input(|i| i.pointer.delta());

            if let Some(ref mut doc) = self.doc {
                if delta.length_sq() > 0.0 {
                    let canvas_rect = ui.max_rect();

                    if has_cp_drag {
                        // Dragging a specific control point
                        path_editor::apply_drag(doc, &self.canvas_state, canvas_rect, delta);
                    } else if has_multi_select {
                        // Multi-select drag: translate all selected paths
                        let zoom_factor = {
                            let svg_size = egui::vec2(doc.width, doc.height);
                            let fit_scale = (canvas_rect.width() / svg_size.x)
                                .min(canvas_rect.height() / svg_size.y)
                                * 0.9;
                            fit_scale * self.canvas_state.zoom
                        };
                        let svg_delta = egui::vec2(delta.x / zoom_factor, delta.y / zoom_factor);
                        for &idx in &self.canvas_state.selected_paths {
                            if let Some(path) = doc.paths.get_mut(idx) {
                                path.translate_x += svg_delta.x;
                                path.translate_y += svg_delta.y;
                            }
                        }
                    }
                    self.texture = None;
                }
            }

            if ui.input(|i| i.pointer.any_released()) {
                self.canvas_state.dragging = None;
            }
        }
    }

    /// First-frame DPI auto-detection when no config DPI is set.
    fn init_dpi_if_needed(&mut self, ctx: &egui::Context) {
        if !self.dpi_initialized && self.config.dpi == 0.0 {
            let detected = AppConfig::auto_detect_dpi(ctx);
            self.config.dpi = detected;
            ctx.set_pixels_per_point(detected);
            self.config.save();
            log::info!("Auto-detected DPI: {}", detected);
        }
        self.dpi_initialized = true;
    }
}

impl eframe::App for SvgEditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.request_repaint();

        // First-frame initialization
        self.init_dpi_if_needed(&ctx);

        // Handle keyboard shortcuts: Ctrl+Z = undo, Ctrl+Y / Ctrl+Shift+Z = redo
        let undo_pressed = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
        let redo_pressed = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
                || i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )
        });
        if undo_pressed {
            self.undo();
        }
        if redo_pressed {
            self.redo();
        }

        // Arrow keys to move selected paths
        if !self.canvas_state.selected_paths.is_empty() && self.doc.is_some() {
            let step = if ui.input(|i| i.modifiers.shift) { 10.0 } else { 1.0 };
            let mut moved = false;
            let mut dx = 0.0f32;
            let mut dy = 0.0f32;
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)) {
                dx -= step;
                moved = true;
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)) {
                dx += step;
                moved = true;
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                dy -= step;
                moved = true;
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                dy += step;
                moved = true;
            }
            if moved {
                self.save_history();
                let doc = self.doc.as_mut().unwrap();
                for &idx in &self.canvas_state.selected_paths {
                    if let Some(path) = doc.paths.get_mut(idx) {
                        path.translate_x += dx;
                        path.translate_y += dy;
                    }
                }
                self.texture = None;
            }
        }

        // Handle drag operations
        self.handle_drag(ui);

        let lang = self.lang;

        // Top menu bar
        egui::Panel::top("menu_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(i18n::t("menu.file", &lang), |ui| {
                    if ui.button(i18n::t("menu.open", &lang)).clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .pick_file()
                        {
                            self.load_file(&path);
                        }
                    }

                    let has_doc = self.doc.is_some();
                    if ui
                        .add_enabled(has_doc, egui::Button::new(i18n::t("menu.save_as", &lang)))
                        .clicked()
                    {
                        ui.close();
                        self.save_file_as();
                    }

                    if ui
                        .add_enabled(has_doc, egui::Button::new(i18n::t("menu.export_png", &lang)))
                        .clicked()
                    {
                        ui.close();
                        self.export_as("png");
                    }

                    if ui
                        .add_enabled(has_doc, egui::Button::new(i18n::t("menu.export_jpg", &lang)))
                        .clicked()
                    {
                        ui.close();
                        self.export_as("jpg");
                    }

                    ui.separator();

                    if ui.button(i18n::t("menu.quit", &lang)).clicked() {
                        ui.close();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        // Toolbar
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                // File buttons
                if ui.button(format!("📂 {}", i18n::t("toolbar.open", &lang))).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SVG", &["svg"])
                        .pick_file()
                    {
                        self.load_file(&path);
                    }
                }

                let has_doc = self.doc.is_some();
                if ui
                    .add_enabled(has_doc, egui::Button::new(format!("💾 {}", i18n::t("toolbar.save", &lang))))
                    .clicked()
                {
                    self.save_file_direct();
                }

                ui.separator();

                // Undo/Redo
                let can_undo = self.history.as_ref().is_some_and(|h| h.can_undo());
                let can_redo = self.history.as_ref().is_some_and(|h| h.can_redo());
                if ui
                    .add_enabled(can_undo, egui::Button::new(format!("↩ {}", i18n::t("toolbar.undo", &lang))))
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(can_redo, egui::Button::new(format!("↪ {}", i18n::t("toolbar.redo", &lang))))
                    .clicked()
                {
                    self.redo();
                }

                ui.separator();

                // Zoom controls
                if has_doc {
                    if ui.button(format!("🔍+ {}", i18n::t("toolbar.zoom_in", &lang))).clicked() {
                        self.canvas_state.zoom *= 1.25;
                    }
                    if ui.button(format!("🔍- {}", i18n::t("toolbar.zoom_out", &lang))).clicked() {
                        self.canvas_state.zoom *= 0.8;
                    }
                    if ui.button(i18n::t("toolbar.fit", &lang)).clicked() {
                        self.canvas_state.zoom = 1.0;
                        self.canvas_state.pan = Vec2::ZERO;
                    }
                    ui.label(format!("{}%", (self.canvas_state.zoom * 100.0) as i32));

                    ui.separator();

                    // Drawing mode toggle
                    let draw_label = if self.canvas_state.drawing_mode {
                        format!("✏ {} [{}]", i18n::t("toolbar.drawing", &lang), i18n::t("toolbar.drawing_hint", &lang))
                    } else {
                        format!("✏ {}", i18n::t("toolbar.draw", &lang))
                    };
                    if ui.selectable_label(self.canvas_state.drawing_mode, draw_label).clicked() {
                        self.canvas_state.drawing_mode = !self.canvas_state.drawing_mode;
                        self.canvas_state.drawing_points.clear();
                        self.canvas_state.drawing_closed = false;
                        self.canvas_state.text_mode = false;
                    }

                    // Text mode toggle
                    let text_label = if self.canvas_state.text_mode {
                        format!("T {} [{}]", i18n::t("toolbar.texting", &lang), i18n::t("toolbar.text_hint", &lang))
                    } else {
                        format!("T {}", i18n::t("toolbar.text", &lang))
                    };
                    if ui.selectable_label(self.canvas_state.text_mode, text_label).clicked() {
                        self.canvas_state.text_mode = !self.canvas_state.text_mode;
                        self.canvas_state.text_position = None;
                        self.canvas_state.text_input.clear();
                        self.canvas_state.drawing_mode = false;
                    }
                }

                // DPI controls
                ui.label(format!("{}:", i18n::t("toolbar.dpi", &lang)));
                if ui.button("-").clicked() {
                    self.config.dpi = (self.config.dpi - 0.5).max(0.5);
                    ctx.set_pixels_per_point(self.config.dpi);
                    self.config.save();
                }
                ui.label(format!("{:.1}", self.config.dpi));
                if ui.button("+").clicked() {
                    self.config.dpi = (self.config.dpi + 0.5).min(4.0);
                    ctx.set_pixels_per_point(self.config.dpi);
                    self.config.save();
                }

                ui.separator();

                // Language selector
                ui.label(format!("{}:", i18n::t("toolbar.language", &lang)));
                egui::ComboBox::from_id_salt("lang_select")
            .selected_text(self.lang.display_name())
            .show_ui(ui, |ui| {
                for &l in Lang::all() {
                    if ui
                        .selectable_value(&mut self.lang, l, l.display_name())
                        .clicked()
                    {
                        self.config.language = l.code().to_string();
                        self.config.save();
                        // Update status message to new language
                        self.status_msg = i18n::t("status.ready", &self.lang).to_string();
                    }
                }
            });

                ui.separator();

                // Theme selector
                ui.label(format!("{}:", i18n::t("toolbar.theme", &lang)));
                let theme_label = match self.config.theme {
                    ThemePreference::System => i18n::t("theme.system", &lang),
                    ThemePreference::Dark => i18n::t("theme.dark", &lang),
                    ThemePreference::Light => i18n::t("theme.light", &lang),
                };
                egui::ComboBox::from_id_salt("theme_select")
            .selected_text(theme_label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(
                        self.config.theme == ThemePreference::System,
                        i18n::t("theme.system", &lang),
                    )
                    .clicked()
                {
                    self.config.theme = ThemePreference::System;
                    ctx.set_theme(ThemePreference::System.to_egui());
                    self.config.save();
                }
                if ui
                    .selectable_label(
                        self.config.theme == ThemePreference::Dark,
                        i18n::t("theme.dark", &lang),
                    )
                    .clicked()
                {
                    self.config.theme = ThemePreference::Dark;
                    ctx.set_theme(ThemePreference::Dark.to_egui());
                    self.config.save();
                }
                if ui
                    .selectable_label(
                        self.config.theme == ThemePreference::Light,
                        i18n::t("theme.light", &lang),
                    )
                    .clicked()
                {
                    self.config.theme = ThemePreference::Light;
                    ctx.set_theme(ThemePreference::Light.to_egui());
                    self.config.save();
                }
            });
            });
        });

        // Status bar
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            if let Some(ref err) = self.error_msg {
                ui.colored_label(egui::Color32::RED, err);
            } else {
                ui.label(&self.status_msg);
            }
        });

        // Left panel: path list
        if self.doc.is_some() {
            egui::Panel::left("path_list")
                .default_size(200.0)
                .show(ui, |ui| {
                    if let Some(ref mut doc) = self.doc {
                        panels::show_path_list(ui, doc, &mut self.canvas_state, &self.lang);
                    }
                });

            // Right panel: properties
            egui::Panel::right("properties")
                .default_size(250.0)
                .show(ui, |ui| {
                    if let Some(ref mut doc) = self.doc {
                        // Save pre-change state before showing properties
                        let pre_change = doc.clone();
                        if panels::show_properties(ui, doc, &mut self.canvas_state, &self.lang) {
                            // Push the pre-change state to history
                            if let Some(ref mut hist) = self.history {
                                hist.push(pre_change);
                            }
                            self.texture = None;
                        }
                    }
                });
        }

        // Central canvas (must be last)
        egui::CentralPanel::default().show(ui, |ui| {
            // Consume finalized drawing first (before borrowing doc)
            let finalized = self.canvas_state.finalized_drawing.take();
            if let Some((points, closed)) = finalized {
                if !points.is_empty() && self.doc.is_some() {
                    let commands: Vec<crate::svg_doc::PathCmd> = points
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            if i == 0 {
                                crate::svg_doc::PathCmd::MoveTo(p.x, p.y)
                            } else {
                                crate::svg_doc::PathCmd::LineTo(p.x, p.y)
                            }
                        })
                        .chain(if closed { Some(crate::svg_doc::PathCmd::Close) } else { None })
                        .collect();

                    self.save_history();
                    let doc = self.doc.as_mut().unwrap();
                    doc.paths.push(crate::svg_doc::EditablePath {
                        id: String::new(),
                        fill_color: Some(egui::Color32::from_rgb(0, 0, 0)),
                        stroke_color: None,
                        stroke_width: 0.0,
                        commands,
                        translate_x: 0.0,
                        translate_y: 0.0,
                        scale_x: 1.0,
                        scale_y: 1.0,
                        scale_locked: true,
                        rotation: 0.0,
                        pivot_x: 0.5,
                        pivot_y: 0.5,
                    });
                    self.canvas_state.selected_paths.clear();
                    self.canvas_state.selected_paths.push(doc.paths.len() - 1);
                    self.texture = None;
                    self.status_msg = format!("{}: {}",
                        i18n::t("status.new_path", &self.lang),
                        i18n::t("toolbar.draw", &self.lang));
                }
            }

            // Consume finalized text and convert to paths
            let finalized_text = self.canvas_state.finalized_text.take();
            if let Some((text, pos, font_size, font_family, bold, italic)) = finalized_text {
                if !text.is_empty() && self.doc.is_some() {
                    let new_paths = crate::svg_doc::text_to_paths(&text, pos.x, pos.y, font_size, &font_family, bold, italic);
                    if !new_paths.is_empty() {
                        self.save_history();
                        let doc = self.doc.as_mut().unwrap();
                        let count = new_paths.len();
                        doc.paths.extend(new_paths);
                        self.canvas_state.selected_paths.clear();
                        self.canvas_state.selected_paths.push(doc.paths.len() - 1);
                        self.texture = None;
                        self.status_msg = format!("{}: {} ({} {})",
                            i18n::t("status.new_path", &self.lang),
                            text,
                            count,
                            i18n::t("paths.heading", &self.lang).to_lowercase());
                    }
                }
            }

            // Text input UI overlay
            if self.canvas_state.text_mode {
                egui::Window::new(i18n::t("toolbar.text", &self.lang))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
                    .show(ui.ctx(), |ui| {
                        ui.label(i18n::t("toolbar.text_hint", &self.lang));
                        ui.horizontal(|ui| {
                            ui.label(i18n::t("props.text_content", &self.lang));
                            let r = ui.text_edit_singleline(&mut self.canvas_state.text_input);
                            if r.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                // Enter pressed in text field - finalize
                                if let Some(pos) = self.canvas_state.text_position {
                                    if !self.canvas_state.text_input.is_empty() {
                                        let text = std::mem::take(&mut self.canvas_state.text_input);
                                        self.canvas_state.finalized_text =
                                            Some((text, pos, self.canvas_state.text_font_size, self.canvas_state.text_font_family.clone(), self.canvas_state.text_bold, self.canvas_state.text_italic));
                                        self.canvas_state.text_mode = false;
                                        self.canvas_state.text_position = None;
                                    }
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(i18n::t("props.text_size", &self.lang));
                            ui.add(
                                egui::DragValue::new(&mut self.canvas_state.text_font_size)
                                    .speed(1.0)
                                    .range(8.0..=f32::MAX),
                            );
                        });
                        // Font family selector
                        ui.horizontal(|ui| {
                            ui.label(i18n::t("props.text_font", &self.lang));
                            let font_list: Vec<String> = self.canvas_state.font_list.clone();
                            egui::ComboBox::from_id_salt("font_select")
                                .selected_text(&self.canvas_state.text_font_family)
                                .width(200.0)
                                .show_ui(ui, |ui| {
                                    ui.label(format!("({} {})", font_list.len(), i18n::t("props.text_font_count", &self.lang)));
                                    ui.separator();
                                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                        for family in &font_list {
                                            if ui
                                                .selectable_label(
                                                    self.canvas_state.text_font_family == *family,
                                                    family,
                                                )
                                                .clicked()
                                            {
                                                self.canvas_state.text_font_family = family.clone();
                                            }
                                        }
                                    });
                                });
                        });
                        // Bold / Italic toggles
                        ui.horizontal(|ui| {
                            ui.label(i18n::t("props.text_style", &self.lang));
                            let bold_btn = egui::Button::new("B").selected(self.canvas_state.text_bold);
                            if ui.add(bold_btn).clicked() {
                                self.canvas_state.text_bold = !self.canvas_state.text_bold;
                            }
                            let italic_btn = egui::Button::new("I").selected(self.canvas_state.text_italic);
                            if ui.add(italic_btn).clicked() {
                                self.canvas_state.text_italic = !self.canvas_state.text_italic;
                            }
                        });
                        if ui.button(i18n::t("menu.quit", &self.lang)).clicked() {
                            self.canvas_state.text_mode = false;
                            self.canvas_state.text_position = None;
                            self.canvas_state.text_input.clear();
                        }
                    });
            }

            if let Some(ref doc) = self.doc {
                let old_selected = self.canvas_state.selected_paths.clone();

                canvas::show_canvas(ui, doc, &mut self.canvas_state, &mut self.texture);

                // Handle click-to-select on canvas
                if ui.input(|i| i.pointer.any_released()) && self.canvas_state.dragging.is_none() {
                    if let Some(click_pos) = ui.input(|i| i.pointer.latest_pos()) {
                        let canvas_rect = ui.max_rect();
                        let tolerance = 10.0;
                        if let Some(clicked_idx) =
                            path_editor::hit_test(doc, &self.canvas_state, canvas_rect, click_pos, tolerance)
                        {
                            if ui.input(|i| i.modifiers.ctrl) {
                                // Ctrl+click: toggle in multi-selection
                                if let Some(pos) = self.canvas_state.selected_paths.iter().position(|&x| x == clicked_idx) {
                                    self.canvas_state.selected_paths.remove(pos);
                                } else {
                                    self.canvas_state.selected_paths.push(clicked_idx);
                                }
                            } else {
                                self.canvas_state.selected_paths.clear();
                                self.canvas_state.selected_paths.push(clicked_idx);
                            }
                        }
                    }
                }

                // Status update on selection change
                if self.canvas_state.selected_paths != old_selected {
                    if let Some(&idx) = self.canvas_state.selected_paths.last() {
                        if let Some(path) = doc.paths.get(idx) {
                            let name = if path.id.is_empty() {
                                format!("{} {}", i18n::t("paths.path_n", &self.lang), idx + 1)
                            } else {
                                path.id.clone()
                            };
                            let count = self.canvas_state.selected_paths.len();
                            if count > 1 {
                                self.status_msg = format!("{}: {} (+{})", i18n::t("status.selected", &self.lang), name, count - 1);
                            } else {
                                self.status_msg = format!("{}: {}", i18n::t("status.selected", &self.lang), name);
                            }
                        }
                    }
                }
            } else {
                // Welcome screen
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.heading(i18n::t("welcome.title", &self.lang));
                    ui.add_space(16.0);
                    ui.label(i18n::t("welcome.no_file", &self.lang));
                    ui.add_space(8.0);
                    if ui.button(i18n::t("welcome.open_svg", &self.lang)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SVG", &["svg"])
                            .pick_file()
                        {
                            self.load_file(&path);
                        }
                    }
                    ui.add_space(16.0);
                    ui.label(i18n::t("welcome.drag_drop", &self.lang));
                });

                // Handle file drag & drop
                let dropped_files = ui.input(|i| i.raw.dropped_files.clone());
                for file in dropped_files {
                    if let Some(ref path) = file.path {
                        self.load_file(path);
                    }
                }
            }
        });
    }
}

/// Load system font family names using fontdb.
fn load_system_font_families() -> Vec<String> {
    let mut fontdb = resvg::usvg::fontdb::Database::new();
    let count_before = fontdb.len();
    fontdb.load_system_fonts();
    let count_after = fontdb.len();
    log::info!("fontdb: before={}, after={}", count_before, count_after);

    let mut families = std::collections::BTreeSet::new();
    for face in fontdb.faces() {
        for (name, _lang) in &face.families {
            families.insert(name.clone());
        }
    }
    let result: Vec<String> = families.into_iter().collect();
    log::info!("Font families found: {}", result.len());
    if result.is_empty() {
        log::warn!("No font families found! fontdb has {} faces", count_after);
    }
    result
}
