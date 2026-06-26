// Hide the Windows console window in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod canvas;
mod config;
mod history;
mod i18n;
mod panels;
mod path_editor;
pub mod svg_doc;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("SVG Editor"),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "SVG Editor",
        options,
        Box::new(|cc| {
            // Bundle Noto Sans CJK SC for Chinese text support
            cc.egui_ctx.add_font(egui::epaint::text::FontInsert::new(
                "NotoSansCJK",
                egui::FontData::from_static(include_bytes!(
                    "../fonts/NotoSansSC-Regular.ttf"
                )),
                vec![
                    egui::epaint::text::InsertFontFamily {
                        family: egui::FontFamily::Proportional,
                        priority: egui::epaint::text::FontPriority::Lowest,
                    },
                    egui::epaint::text::InsertFontFamily {
                        family: egui::FontFamily::Monospace,
                        priority: egui::epaint::text::FontPriority::Lowest,
                    },
                ],
            ));

            Ok(Box::new(app::SvgEditorApp::new(cc)))
        }),
    )
}
