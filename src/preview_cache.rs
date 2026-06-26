use std::time::{Duration, Instant};

use egui::TextureHandle;

use crate::canvas;
use crate::svg_doc::SvgDoc;

const TEXTURE_THROTTLE: Duration = Duration::from_millis(80);

/// Cached SVG preview texture with stale flag and update throttling.
#[derive(Default)]
pub struct SvgPreviewCache {
    texture: Option<TextureHandle>,
    stale: bool,
    last_update: Option<Instant>,
}

impl SvgPreviewCache {
    pub fn invalidate(&mut self) {
        self.stale = true;
    }

    pub fn clear(&mut self) {
        self.texture = None;
        self.stale = true;
        self.last_update = None;
    }

    pub fn texture(&self) -> Option<&TextureHandle> {
        self.texture.as_ref()
    }

    pub fn is_stale(&self) -> bool {
        self.stale || self.texture.is_none()
    }

    /// Regenerate the preview texture when stale, throttling rapid updates.
    pub fn ensure_fresh(&mut self, ctx: &egui::Context, doc: &SvgDoc) {
        if !self.stale && self.texture.is_some() {
            return;
        }

        let now = Instant::now();
        if self.texture.is_some() {
            if let Some(last) = self.last_update {
                if now.duration_since(last) < TEXTURE_THROTTLE {
                    ctx.request_repaint_after(TEXTURE_THROTTLE - now.duration_since(last));
                    return;
                }
            }
        }

        let color_image = canvas::render_svg_to_image(doc, 1024);
        let handle = ctx.load_texture(
            "svg_preview",
            color_image,
            egui::TextureOptions::LINEAR,
        );
        self.texture = Some(handle);
        self.stale = false;
        self.last_update = Some(now);
    }
}
