use egui::{self, Color32};

use crate::canvas::CanvasState;
use crate::i18n::{t, Lang};
use crate::svg_doc::SvgDoc;

/// Show the left panel: list of paths.
pub fn show_path_list(ui: &mut egui::Ui, doc: &mut SvgDoc, state: &mut CanvasState, lang: &Lang) {
    ui.heading(t("paths.heading", lang));
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for i in 0..doc.paths.len() {
            let is_selected = state.selected_paths.contains(&i);

            let (fill_preview, stroke_preview, label_text) = {
                let path = &doc.paths[i];
                let fill = path.fill_color.unwrap_or(Color32::TRANSPARENT);
                let stroke = path.stroke_color.unwrap_or(Color32::TRANSPARENT);
                let label = if path.id.is_empty() {
                    format!("{} {}", t("paths.path_n", lang), i + 1)
                } else {
                    path.id.clone()
                };
                (fill, stroke, label)
            };

            let bg_color = if is_selected {
                Some(ui.visuals().selection.bg_fill)
            } else {
                None
            };

            let frame = egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(8, 4))
                .corner_radius(4)
                .fill(bg_color.unwrap_or(Color32::TRANSPARENT));

            let response = ui
                .allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), 36.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        frame.show(ui, |ui| {
                            // Color preview squares
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(14.0, 14.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 2.0, fill_preview);
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0, Color32::from_gray(100)),
                                egui::StrokeKind::Middle,
                            );

                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(14.0, 14.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 2.0, stroke_preview);
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0, Color32::from_gray(100)),
                                egui::StrokeKind::Middle,
                            );

                            ui.label(&label_text);
                        });
                    },
                )
                .response;

            let click_response = ui.interact(
                response.rect,
                egui::Id::new("path_list").with(i),
                egui::Sense::click(),
            );

            if click_response.clicked() {
                if ui.input(|i| i.modifiers.ctrl) {
                    if is_selected {
                        state.selected_paths.retain(|&x| x != i);
                    } else {
                        state.selected_paths.push(i);
                    }
                } else {
                    state.selected_paths.clear();
                    state.selected_paths.push(i);
                }
            }
        }
    });
}

/// Show the right panel: properties for canvas and selected path.
/// Returns `true` if the document was modified and needs re-rendering.
pub fn show_properties(ui: &mut egui::Ui, doc: &mut SvgDoc, state: &mut CanvasState, lang: &Lang) -> bool {
    let mut changed = false;

    ui.heading(t("props.heading", lang));
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Canvas section
        ui.collapsing(t("props.canvas", lang), |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{}:", t("props.width", lang)));
                if ui.add(egui::DragValue::new(&mut doc.width).speed(1.0).range(1.0..=10000.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(format!("{}:", t("props.height", lang)));
                if ui.add(egui::DragValue::new(&mut doc.height).speed(1.0).range(1.0..=10000.0)).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label(format!("{}:", t("props.background", lang)));
                changed |= hex_color_input(ui, &mut doc.bg_color, "bg_hex", 0);
                if ui.small_button("✖").on_hover_text(t("props.clear_bg", lang)).clicked() {
                    doc.bg_color = Color32::TRANSPARENT;
                    changed = true;
                }
            });
        });

        ui.separator();

        // Selected path section
        if let Some(&idx) = state.selected_paths.last() {
            if idx < doc.paths.len() {
                let path_name = {
                    let path = &doc.paths[idx];
                    if path.id.is_empty() {
                        format!("{} {}", t("paths.path_n", lang), idx + 1)
                    } else {
                        path.id.clone()
                    }
                };

                let mut delete_requested = false;

                ui.collapsing(&path_name, |ui| {
                    let path = &mut doc.paths[idx];

                    // Fill color
                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", t("props.fill", lang)));
                        let mut has_fill = path.fill_color.is_some();
                        if ui.checkbox(&mut has_fill, "").changed() {
                            if has_fill {
                                path.fill_color = Some(Color32::BLACK);
                            } else {
                                path.fill_color = None;
                            }
                            changed = true;
                        }
                        if let Some(ref mut fill) = path.fill_color {
                            changed |= hex_color_input(ui, fill, "fill_hex", idx);
                        }
                    });

                    // Stroke color
                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", t("props.stroke", lang)));
                        let mut has_stroke = path.stroke_color.is_some();
                        if ui.checkbox(&mut has_stroke, "").changed() {
                            if has_stroke {
                                path.stroke_color = Some(Color32::BLACK);
                                if path.stroke_width == 0.0 {
                                    path.stroke_width = 1.0;
                                }
                            } else {
                                path.stroke_color = None;
                            }
                            changed = true;
                        }
                        if let Some(ref mut stroke) = path.stroke_color {
                            changed |= hex_color_input(ui, stroke, "stroke_hex", idx);
                        }
                    });

                    // Stroke width
                    if path.stroke_color.is_some() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", t("props.stroke_width", lang)));
                            if ui.add(
                                egui::DragValue::new(&mut path.stroke_width)
                                    .speed(0.1)
                                    .range(0.1..=100.0),
                            ).changed() {
                                changed = true;
                            }
                        });
                    }

                    ui.separator();

                    // Transform controls
                    ui.collapsing(t("props.transform", lang), |ui| {
                        // Position (translate)
                        ui.horizontal(|ui| {
                            ui.label(format!("{} X:", t("props.translate", lang)));
                            if ui.add(egui::DragValue::new(&mut path.translate_x).speed(1.0)).changed() {
                                changed = true;
                            }
                            ui.label("Y:");
                            if ui.add(egui::DragValue::new(&mut path.translate_y).speed(1.0)).changed() {
                                changed = true;
                            }
                        });

                        // Scale
                        ui.horizontal(|ui| {
                            ui.label(format!("{} X:", t("props.scale", lang)));
                            let r = ui.add(egui::DragValue::new(&mut path.scale_x).speed(0.01).range(0.01..=100.0));
                            if r.changed() {
                                if path.scale_locked {
                                    path.scale_y = path.scale_x;
                                }
                                changed = true;
                            }

                            // Lock aspect ratio toggle
                            let lock_icon = if path.scale_locked { "🔗" } else { "🔓" };
                            let lock_text = if path.scale_locked {
                                t("props.locked", lang)
                            } else {
                                t("props.unlocked", lang)
                            };
                            if ui.small_button(lock_icon).on_hover_text(lock_text).clicked() {
                                path.scale_locked = !path.scale_locked;
                                if path.scale_locked {
                                    path.scale_y = path.scale_x;
                                    changed = true;
                                }
                            }

                            ui.label("Y:");
                            let r = ui.add(egui::DragValue::new(&mut path.scale_y).speed(0.01).range(0.01..=100.0));
                            if r.changed() {
                                if path.scale_locked {
                                    path.scale_x = path.scale_y;
                                }
                                changed = true;
                            }
                        });

                        // Rotation
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", t("props.rotation", lang)));
                            if ui.add(egui::DragValue::new(&mut path.rotation).speed(1.0).suffix("°")).changed() {
                                changed = true;
                            }
                        });

                        // Pivot
                        ui.horizontal(|ui| {
                            ui.label(format!("{} X:", t("props.pivot", lang)));
                            if ui.add(egui::DragValue::new(&mut path.pivot_x).speed(0.01).range(0.0..=1.0)).changed() {
                                changed = true;
                            }
                            ui.label("Y:");
                            if ui.add(egui::DragValue::new(&mut path.pivot_y).speed(0.01).range(0.0..=1.0)).changed() {
                                changed = true;
                            }
                        });
                    });

                    ui.separator();

                    ui.label(format!("{}: {}", t("props.commands", lang), path.commands.len()));

                    // Path source code editor
                    ui.collapsing(t("props.source", lang), |ui| {
                        let mut d_string = path.to_d_string();
                        let te = egui::TextEdit::multiline(&mut d_string)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(6);
                        if ui.add(te).changed() {
                            if let Some(new_cmds) = crate::svg_doc::EditablePath::parse_d_string(&d_string) {
                                if !new_cmds.is_empty() {
                                    path.commands = new_cmds;
                                    changed = true;
                                }
                            }
                        }
                    });

                    if ui.button(t("props.delete_path", lang)).clicked() {
                        delete_requested = true;
                    }
                });

                if delete_requested {
                    doc.paths.remove(idx);
                    state.selected_paths.clear();
                    changed = true;
                }
            }
        } else {
            ui.label(t("props.no_selection", lang));
            ui.label(t("props.click_to_select", lang));
        }

        ui.separator();

        // Zoom controls
        ui.collapsing(t("props.view", lang), |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{}:", t("props.zoom", lang)));
                let zoom_pct = state.zoom * 100.0;
                let mut zoom = zoom_pct;
                ui.add(egui::DragValue::new(&mut zoom).speed(1.0).range(5.0..=10000.0).suffix("%"));
                state.zoom = zoom / 100.0;
            });

            if ui.button(t("props.reset_view", lang)).clicked() {
                state.zoom = 1.0;
                state.pan = egui::Vec2::ZERO;
            }
        });
    });

    changed
}

/// Hex color input with color picker and swatch preview.
/// Uses widget memory to preserve user edits across frames.
/// Returns true if color was changed.
fn hex_color_input(ui: &mut egui::Ui, color: &mut Color32, tag: &str, idx: usize) -> bool {
    let mut changed = false;

    // Color picker button (click to open full color picker)
    if egui::color_picker::color_edit_button_srgba(ui, color, egui::color_picker::Alpha::Opaque).changed() {
        changed = true;
    }

    let id = ui.make_persistent_id((tag, idx));
    let color_hex = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());

    // Track the last known color to detect external changes (from color picker)
    let last_color_id = ui.make_persistent_id((tag, "last_color", idx));
    let last_color = ui.memory_mut(|m| {
        m.data.get_temp::<String>(last_color_id).unwrap_or_default()
    });

    // Get editing text from memory, or initialize from color
    let mut hex = ui.memory_mut(|m| {
        m.data.get_temp::<String>(id).unwrap_or_else(|| color_hex.clone())
    });

    // If color changed externally (picker), update hex to match
    if last_color != color_hex {
        hex = color_hex.clone();
    }

    let resp = ui.add(
        egui::TextEdit::singleline(&mut hex)
            .desired_width(72.0)
            .font(egui::TextStyle::Monospace),
    );

    // Save editing text and current color to memory
    ui.memory_mut(|m| {
        m.data.insert_temp(id, hex.clone());
        m.data.insert_temp(last_color_id, color_hex.clone());
    });

    // Apply on Enter
    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        if let Some(c) = parse_hex_color(&hex) {
            *color = c;
            changed = true;
        }
        ui.memory_mut(|m| m.surrender_focus(id));
    }

    // Apply on focus loss
    if resp.lost_focus() {
        if let Some(c) = parse_hex_color(&hex) {
            *color = c;
            changed = true;
        }
    }

    changed
}

/// Parse a hex color string like "#ff0000" or "ff0000" or "#f00" into Color32.
fn parse_hex_color(s: &str) -> Option<Color32> {
    let s = s.trim().strip_prefix('#').unwrap_or(s.trim());
    let s = match s.len() {
        3 => {
            // Short form: "f00" -> "ff0000"
            let mut expanded = String::with_capacity(6);
            for c in s.chars() {
                expanded.push(c);
                expanded.push(c);
            }
            expanded
        }
        6 => s.to_string(),
        8 => s.to_string(),
        _ => return None,
    };

    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else if s.len() == 8 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        let a = u8::from_str_radix(&s[6..8], 16).ok()?;
        Some(Color32::from_rgba_unmultiplied(r, g, b, a))
    } else {
        None
    }
}
