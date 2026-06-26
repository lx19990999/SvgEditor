use egui::{Color32, Pos2, Rect, Sense, Stroke, TextureHandle, Vec2};

use crate::svg_doc::{EditablePath, PathCmd, SvgDoc};

/// Canvas state: zoom, pan, cached texture.
#[derive(Clone, Debug)]
pub struct CanvasState {
    /// Zoom factor (1.0 = 100%).
    pub zoom: f32,
    /// Pan offset in screen coordinates.
    pub pan: Vec2,
    /// Currently selected path index.
    pub selected_path: Option<usize>,
    /// Currently dragged point index.
    pub dragging: Option<DragState>,
    /// Drawing mode: collecting points for a new path.
    pub drawing_mode: bool,
    /// Points collected so far while drawing (in SVG coordinates).
    pub drawing_points: Vec<Pos2>,
    /// Whether the current path is closed.
    pub drawing_closed: bool,
    /// Finalized drawing ready to be added as a new path (consumed by app.rs).
    pub finalized_drawing: Option<(Vec<Pos2>, bool)>,
}

#[derive(Clone, Debug)]
pub struct DragState {
    pub path_idx: usize,
    pub cmd_idx: usize,
    pub point_role: u8,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            selected_path: None,
            dragging: None,
            drawing_mode: false,
            drawing_points: Vec::new(),
            drawing_closed: false,
            finalized_drawing: None,
        }
    }
}

/// Render the SVG document on the egui canvas using resvg for pixel-perfect rendering.
pub fn show_canvas(
    ui: &mut egui::Ui,
    doc: &SvgDoc,
    state: &mut CanvasState,
    texture: &mut Option<TextureHandle>,
) {
    let available = ui.available_size();
    let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());

    // Dark background around the SVG
    painter.rect_filled(response.rect, 0.0, Color32::from_gray(40));

    // Calculate the transform: center the SVG in the canvas
    let canvas_rect = response.rect;
    let svg_size = Vec2::new(doc.width, doc.height);
    let fit_scale = (canvas_rect.width() / svg_size.x)
        .min(canvas_rect.height() / svg_size.y)
        * 0.9;

    let total_scale = fit_scale * state.zoom;
    let svg_screen_size = svg_size * total_scale;
    let offset = canvas_rect.center() - svg_screen_size / 2.0 + state.pan;

    // SVG bounding rect on screen
    let bg_rect = Rect::from_min_size(
        Pos2::new(offset.x, offset.y),
        svg_screen_size,
    );

    // Generate or use cached texture
    let tex = texture.get_or_insert_with(|| {
        let color_image = render_svg_to_image(doc, 1024);
        ui.ctx()
            .load_texture("svg_preview", color_image, Default::default())
    });

    // Draw the SVG texture
    painter.image(
        tex.id(),
        bg_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );

    // Draw control points for selected path (in SVG coordinates → screen)
    let svg_to_screen = |p: Pos2| -> Pos2 {
        Pos2::new(
            offset.x + p.x * total_scale,
            offset.y + p.y * total_scale,
        )
    };

    if let Some(idx) = state.selected_path {
        if let Some(path) = doc.paths.get(idx) {
            let (cx, cy) = path_center_f(path);
            let w = bbox_w(path);
            let h = bbox_h(path);
            // Pivot in original coordinates (same as SVG export computes)
            let pcx = cx + (path.pivot_x - 0.5) * w;
            let pcy = cy + (path.pivot_y - 0.5) * h;
            let tx = path.translate_x;
            let ty = path.translate_y;
            let sx = path.scale_x;
            let sy = path.scale_y;
            let rot = path.rotation.to_radians();
            let cos_r = rot.cos();
            let sin_r = rot.sin();

            // SVG transform order: scale(origin) → rotate(pivot) → translate
            // rotate(angle, cx, cy) uses the ORIGINAL pivot (not scaled)
            let xform_svg = move |p: Pos2| -> Pos2 {
                // 1. Scale around origin
                let sxp = p.x * sx;
                let syp = p.y * sy;
                // 2. Rotate around ORIGINAL pivot (pcx, pcy)
                let dx = sxp - pcx;
                let dy = syp - pcy;
                let rx = dx * cos_r - dy * sin_r + pcx;
                let ry = dx * sin_r + dy * cos_r + pcy;
                // 3. Translate
                Pos2::new(rx + tx, ry + ty)
            };

            let full_map = move |p: Pos2| -> Pos2 { svg_to_screen(xform_svg(p)) };
            draw_control_points(ui, path, idx, full_map, state);
        }
    }

    // Handle zoom (scroll wheel) — only when mouse is over the canvas
    let scroll = if response.hovered() {
        ui.input(|i| i.smooth_scroll_delta())
    } else {
        Vec2::ZERO
    };
    if scroll.y != 0.0 {
        let old_zoom = state.zoom;
        state.zoom *= 1.0 + scroll.y * 0.002;
        state.zoom = state.zoom.clamp(0.05, 100.0);

        if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
            let zoom_ratio = state.zoom / old_zoom;
            state.pan = (state.pan + mouse_pos.to_vec2() - canvas_rect.center().to_vec2())
                * zoom_ratio
                - (mouse_pos.to_vec2() - canvas_rect.center().to_vec2());
        }
    }

    // Handle pan
    let is_panning = ui.input(|i| i.pointer.middle_down())
        || (ui.input(|i| i.key_down(egui::Key::Space)) && response.dragged());

    if is_panning {
        state.pan += response.drag_delta();
    }

    // Drawing mode: collect points for a new path
    if state.drawing_mode {
        handle_drawing(ui, &painter, response, doc, state, svg_to_screen, canvas_rect, total_scale, offset);
    }
}

/// Handle drawing mode: collect points, show preview, finalize on double-click/Enter.
fn handle_drawing(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    response: egui::Response,
    doc: &SvgDoc,
    state: &mut CanvasState,
    svg_to_screen: impl Fn(Pos2) -> Pos2 + Copy,
    _canvas_rect: Rect,
    total_scale: f32,
    offset: Pos2,
) {
    // Cancel with Escape
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.drawing_mode = false;
        state.drawing_points.clear();
        state.drawing_closed = false;
        return;
    }

    // Get mouse position in SVG coordinates
    let mouse_svg = ui.input(|i| i.pointer.latest_pos()).map(|mouse_pos| {
        Pos2::new(
            (mouse_pos.x - offset.x) / total_scale,
            (mouse_pos.y - offset.y) / total_scale,
        )
    });

    // Handle click to add point
    if response.clicked() {
        if let Some(svg_pos) = mouse_svg {
            // Check if clicking near the first point to close
            if state.drawing_points.len() >= 3 {
                if let Some(first) = state.drawing_points.first() {
                    let dist = (*first - svg_pos).length();
                    if dist < 10.0 / total_scale {
                        state.drawing_closed = true;
                        finalize_drawing(doc, state);
                        return;
                    }
                }
            }
            state.drawing_points.push(svg_pos);
        }
    }

    // Double-click to finish (open path)
    if response.double_clicked() && !state.drawing_points.is_empty() {
        finalize_drawing(doc, state);
        return;
    }

    // Enter to finish
    if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !state.drawing_points.is_empty() {
        finalize_drawing(doc, state);
        return;
    }

    // Draw preview
    let preview_color = Color32::from_rgba_premultiplied(0, 150, 255, 180);
    let point_color = Color32::from_rgb(0, 150, 255);
    let first_color = Color32::from_rgb(255, 100, 0);

    // Draw lines between collected points
    if state.drawing_points.len() >= 2 {
        let screen_points: Vec<Pos2> = state.drawing_points.iter().map(|p| svg_to_screen(*p)).collect();
        painter.add(egui::Shape::line(screen_points.clone(), Stroke::new(2.0, preview_color)));
        // Line to mouse
        if let Some(svg_pos) = mouse_svg {
            let last_screen = svg_to_screen(*state.drawing_points.last().unwrap());
            let mouse_screen = svg_to_screen(svg_pos);
            painter.line_segment([last_screen, mouse_screen], Stroke::new(1.0, preview_color));
        }
    } else if state.drawing_points.len() == 1 {
        // Line from first point to mouse
        if let Some(svg_pos) = mouse_svg {
            let first_screen = svg_to_screen(state.drawing_points[0]);
            let mouse_screen = svg_to_screen(svg_pos);
            painter.line_segment([first_screen, mouse_screen], Stroke::new(1.0, preview_color));
        }
    }

    // Draw point handles
    for (i, p) in state.drawing_points.iter().enumerate() {
        let sp = svg_to_screen(*p);
        let color = if i == 0 { first_color } else { point_color };
        painter.circle_filled(sp, 4.0, color);
        painter.circle_stroke(sp, 4.0, Stroke::new(1.5, Color32::WHITE));
    }
}

/// Finalize the drawing: store points for app.rs to consume.
fn finalize_drawing(_doc: &SvgDoc, state: &mut CanvasState) {
    state.finalized_drawing = Some((std::mem::take(&mut state.drawing_points), state.drawing_closed));
    state.drawing_mode = false;
    state.drawing_closed = false;
}

/// Render the SVG document to a ColorImage using resvg.
/// The image has a checkerboard background with the SVG drawn on top.
fn render_svg_to_image(doc: &SvgDoc, max_size: u32) -> egui::ColorImage {
    let w = (doc.width as u32).max(1).min(max_size);
    let h = (doc.height as u32).max(1).min(max_size);

    // Create pixmap with transparent background
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).unwrap();

    // Draw checkerboard pattern
    let tile = 8u32;
    let white = resvg::tiny_skia::ColorU8::from_rgba(255, 255, 255, 255).premultiply();
    let gray = resvg::tiny_skia::ColorU8::from_rgba(204, 204, 204, 255).premultiply();
    let pixels = pixmap.pixels_mut();
    for y in 0..h {
        for x in 0..w {
            let color = if (x / tile + y / tile).is_multiple_of(2) {
                white
            } else {
                gray
            };
            let idx = (y * w + x) as usize;
            pixels[idx] = color;
        }
    }

    // Generate SVG string from our model and render with resvg
    let svg_string = doc.to_svg_string();
    if let Ok(tree) =
        resvg::usvg::Tree::from_data(svg_string.as_bytes(), &resvg::usvg::Options::default())
    {
        let tree_size = tree.size();
        let scale_x = w as f32 / tree_size.width();
        let scale_y = h as f32 / tree_size.height();
        let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
    }

    // Convert pixmap to egui ColorImage
    let pixels: Vec<Color32> = pixmap
        .pixels()
        .iter()
        .map(|p| {
            let a = p.alpha();
            if a == 0 {
                Color32::TRANSPARENT
            } else {
                let r = (p.red() as u16 * 255 / a as u16) as u8;
                let g = (p.green() as u16 * 255 / a as u16) as u8;
                let b = (p.blue() as u16 * 255 / a as u16) as u8;
                Color32::from_rgba_unmultiplied(r, g, b, a)
            }
        })
        .collect();

    egui::ColorImage {
        size: [w as usize, h as usize],
        source_size: egui::Vec2::new(w as f32, h as f32),
        pixels,
    }
}

/// Draw control points for a selected path.
fn draw_control_points(
    ui: &mut egui::Ui,
    path: &EditablePath,
    path_idx: usize,
    svg_to_screen: impl Fn(Pos2) -> Pos2 + Copy,
    state: &mut CanvasState,
) {
    let point_radius = 5.0;
    let control_color = Color32::from_rgb(0, 150, 255);
    let endpoint_color = Color32::from_rgb(255, 100, 0);

    let mut last_pos = Pos2::ZERO;

    for (cmd_idx, cmd) in path.commands.iter().enumerate() {
        match cmd {
            PathCmd::MoveTo(x, y) => {
                let sp = svg_to_screen(Pos2::new(*x, *y));
                let resp = draw_handle(ui, sp, point_radius, endpoint_color);
                if resp.dragged() {
                    state.dragging = Some(DragState { path_idx, cmd_idx, point_role: 0 });
                }
                last_pos = Pos2::new(*x, *y);
            }
            PathCmd::LineTo(x, y) => {
                let sp = svg_to_screen(Pos2::new(*x, *y));
                let resp = draw_handle(ui, sp, point_radius, endpoint_color);
                if resp.dragged() {
                    state.dragging = Some(DragState { path_idx, cmd_idx, point_role: 0 });
                }
                last_pos = Pos2::new(*x, *y);
            }
            PathCmd::CurveTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                let cp1 = svg_to_screen(Pos2::new(*cp1x, *cp1y));
                let last_s = svg_to_screen(last_pos);
                ui.painter().line_segment([last_s, cp1], Stroke::new(1.0, Color32::from_gray(150)));
                let resp = draw_handle(ui, cp1, point_radius - 1.0, control_color);
                if resp.dragged() {
                    state.dragging = Some(DragState { path_idx, cmd_idx, point_role: 0 });
                }

                let cp2 = svg_to_screen(Pos2::new(*cp2x, *cp2y));
                let end = svg_to_screen(Pos2::new(*x, *y));
                ui.painter().line_segment([cp2, end], Stroke::new(1.0, Color32::from_gray(150)));
                let resp = draw_handle(ui, cp2, point_radius - 1.0, control_color);
                if resp.dragged() {
                    state.dragging = Some(DragState { path_idx, cmd_idx, point_role: 1 });
                }

                let resp = draw_handle(ui, end, point_radius, endpoint_color);
                if resp.dragged() {
                    state.dragging = Some(DragState { path_idx, cmd_idx, point_role: 2 });
                }
                last_pos = Pos2::new(*x, *y);
            }
            PathCmd::QuadTo(cpx, cpy, x, y) => {
                let cp = svg_to_screen(Pos2::new(*cpx, *cpy));
                let last_s = svg_to_screen(last_pos);
                let end = svg_to_screen(Pos2::new(*x, *y));
                ui.painter().line_segment([last_s, cp], Stroke::new(1.0, Color32::from_gray(150)));
                ui.painter().line_segment([cp, end], Stroke::new(1.0, Color32::from_gray(150)));
                let resp = draw_handle(ui, cp, point_radius - 1.0, control_color);
                if resp.dragged() {
                    state.dragging = Some(DragState { path_idx, cmd_idx, point_role: 0 });
                }
                let resp = draw_handle(ui, end, point_radius, endpoint_color);
                if resp.dragged() {
                    state.dragging = Some(DragState { path_idx, cmd_idx, point_role: 1 });
                }
                last_pos = Pos2::new(*x, *y);
            }
            PathCmd::Close => {}
        }
    }
}

/// Draw a draggable handle point.
fn draw_handle(ui: &mut egui::Ui, pos: Pos2, radius: f32, color: Color32) -> egui::Response {
    let size = egui::vec2(radius * 4.0, radius * 4.0);
    let rect = egui::Rect::from_center_size(pos, size);
    let response = ui.allocate_rect(rect, Sense::drag());

    let painter = ui.painter();
    let fill_color = if response.hovered() || response.dragged() {
        Color32::WHITE
    } else {
        color
    };
    painter.circle_filled(pos, radius, fill_color);
    painter.circle_stroke(pos, radius, Stroke::new(1.5, color));

    response
}

/// Compute center of path's raw bounding box.
fn path_center_f(path: &EditablePath) -> (f32, f32) {
    let (min_x, max_x, min_y, max_y) = path_bounds_raw(path);
    ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0)
}

/// Compute width of path's raw bounding box.
fn bbox_w(path: &EditablePath) -> f32 {
    let (min_x, max_x, _, _) = path_bounds_raw(path);
    max_x - min_x
}

/// Compute height of path's raw bounding box.
fn bbox_h(path: &EditablePath) -> f32 {
    let (_, _, min_y, max_y) = path_bounds_raw(path);
    max_y - min_y
}

/// Compute raw bounding box (min_x, max_x, min_y, max_y).
fn path_bounds_raw(path: &EditablePath) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for cmd in &path.commands {
        let (x, y) = match cmd {
            PathCmd::MoveTo(x, y) | PathCmd::LineTo(x, y) => (*x, *y),
            PathCmd::CurveTo(_, _, _, _, x, y) => (*x, *y),
            PathCmd::QuadTo(_, _, x, y) => (*x, *y),
            PathCmd::Close => continue,
        };
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    if min_x > max_x {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        (min_x, max_x, min_y, max_y)
    }
}

/// Convert screen coordinates back to SVG coordinates.
pub fn screen_to_svg(
    screen_pos: Pos2,
    canvas_rect: Rect,
    doc: &SvgDoc,
    state: &CanvasState,
) -> Pos2 {
    let svg_size = Vec2::new(doc.width, doc.height);
    let fit_scale = (canvas_rect.width() / svg_size.x)
        .min(canvas_rect.height() / svg_size.y)
        * 0.9;
    let total_scale = fit_scale * state.zoom;
    let svg_screen_size = svg_size * total_scale;
    let offset = canvas_rect.center() - svg_screen_size / 2.0 + state.pan;

    Pos2::new(
        (screen_pos.x - offset.x) / total_scale,
        (screen_pos.y - offset.y) / total_scale,
    )
}
