use egui::Color32;

/// Represents a single SVG path command in absolute coordinates.
#[derive(Clone, Debug)]
pub enum PathCmd {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    CurveTo(f32, f32, f32, f32, f32, f32), // cubic bezier: cp1x,cp1y, cp2x,cp2y, x,y
    QuadTo(f32, f32, f32, f32),            // quadratic bezier: cpx,cpy, x,y
    Close,
}

/// An editable SVG path with styling.
#[derive(Clone, Debug)]
pub struct EditablePath {
    pub id: String,
    pub fill_color: Option<Color32>,
    pub stroke_color: Option<Color32>,
    pub stroke_width: f32,
    pub commands: Vec<PathCmd>,
    // Transform properties
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub scale_locked: bool, // lock aspect ratio
    pub rotation: f32, // degrees
    pub pivot_x: f32,  // rotation center X (relative to path center, 0-1)
    pub pivot_y: f32,  // rotation center Y (relative to path center, 0-1)
}

impl EditablePath {
    /// Convert path commands to SVG `d` attribute string.
    pub fn to_d_string(&self) -> String {
        let mut d = String::new();
        for cmd in &self.commands {
            match cmd {
                PathCmd::MoveTo(x, y) => d.push_str(&format!("M {} {} ", x, y)),
                PathCmd::LineTo(x, y) => d.push_str(&format!("L {} {} ", x, y)),
                PathCmd::CurveTo(c1x, c1y, c2x, c2y, x, y) => {
                    d.push_str(&format!("C {} {} {} {} {} {} ", c1x, c1y, c2x, c2y, x, y));
                }
                PathCmd::QuadTo(cx, cy, x, y) => {
                    d.push_str(&format!("Q {} {} {} {} ", cx, cy, x, y));
                }
                PathCmd::Close => d.push('Z'),
            }
        }
        d
    }

    /// Parse an SVG `d` attribute string into path commands.
    pub fn parse_d_string(s: &str) -> Option<Vec<PathCmd>> {
        let mut cmds = Vec::new();
        let mut chars = s.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == ',' {
                chars.next();
                continue;
            }

            // Read the command letter (or implicit repeat)
            let cmd = if c.is_alphabetic() {
                chars.next();
                c
            } else {
                // Implicit repeat of last command (but not M after M becomes L)
                match cmds.last() {
                    Some(PathCmd::MoveTo(_, _)) | Some(PathCmd::LineTo(_, _)) => 'L',
                    Some(PathCmd::CurveTo(_, _, _, _, _, _)) => 'C',
                    Some(PathCmd::QuadTo(_, _, _, _)) => 'Q',
                    _ => '\0',
                }
            };

            if cmd == '\0' {
                chars.next();
                continue;
            }

            match cmd {
                'M' | 'm' => {
                    let x = parse_num(&mut chars)?;
                    let y = parse_num(&mut chars)?;
                    cmds.push(PathCmd::MoveTo(x, y));
                    // Subsequent pairs after M are implicit L
                    while let Some(&next) = chars.peek() {
                        if next.is_alphabetic() { break; }
                        let x2 = parse_num(&mut chars)?;
                        let y2 = parse_num(&mut chars)?;
                        cmds.push(PathCmd::LineTo(x2, y2));
                    }
                }
                'L' | 'l' => {
                    while let Some(&next) = chars.peek() {
                        if next.is_alphabetic() { break; }
                        let x = parse_num(&mut chars)?;
                        let y = parse_num(&mut chars)?;
                        cmds.push(PathCmd::LineTo(x, y));
                    }
                }
                'C' | 'c' => {
                    while let Some(&next) = chars.peek() {
                        if next.is_alphabetic() { break; }
                        let c1x = parse_num(&mut chars)?;
                        let c1y = parse_num(&mut chars)?;
                        let c2x = parse_num(&mut chars)?;
                        let c2y = parse_num(&mut chars)?;
                        let x = parse_num(&mut chars)?;
                        let y = parse_num(&mut chars)?;
                        cmds.push(PathCmd::CurveTo(c1x, c1y, c2x, c2y, x, y));
                    }
                }
                'Q' | 'q' => {
                    while let Some(&next) = chars.peek() {
                        if next.is_alphabetic() { break; }
                        let cx = parse_num(&mut chars)?;
                        let cy = parse_num(&mut chars)?;
                        let x = parse_num(&mut chars)?;
                        let y = parse_num(&mut chars)?;
                        cmds.push(PathCmd::QuadTo(cx, cy, x, y));
                    }
                }
                'Z' | 'z' => {
                    cmds.push(PathCmd::Close);
                }
                _ => {
                    // Skip unknown command
                    chars.next();
                }
            }
        }

        Some(cmds)
    }
}

/// Parse a float number from the char iterator, skipping whitespace/commas.
fn parse_num(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<f32> {
    // Skip whitespace and commas
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == ',' {
            chars.next();
        } else {
            break;
        }
    }

    let mut num_str = String::new();

    // Optional sign
    if let Some(&'-') = chars.peek() {
        num_str.push('-');
        chars.next();
    } else if let Some(&'+') = chars.peek() {
        chars.next();
    }

    // Digits before decimal
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }

    // Decimal point and digits after
    if let Some(&'.') = chars.peek() {
        num_str.push('.');
        chars.next();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                chars.next();
            } else {
                break;
            }
        }
    }

    if num_str.is_empty() || num_str == "-" || num_str == "+" {
        return None;
    }

    num_str.parse::<f32>().ok()
}

/// The full editable SVG document.
#[derive(Clone, Debug)]
pub struct SvgDoc {
    pub width: f32,
    pub height: f32,
    pub bg_color: Color32,
    pub paths: Vec<EditablePath>,
    /// Original file path, if any.
    pub file_path: Option<std::path::PathBuf>,
}

impl SvgDoc {
    /// Parse an SVG file from bytes.
    pub fn from_bytes(bytes: &[u8], file_path: Option<std::path::PathBuf>) -> Result<Self, String> {
        let mut opts = resvg::usvg::Options::default();
        // Set resources_dir for resolving relative paths
        if let Some(ref path) = file_path {
            if let Some(parent) = path.parent() {
                opts.resources_dir = Some(parent.to_path_buf());
            }
        }

        let tree = resvg::usvg::Tree::from_data(bytes, &opts).map_err(|e| e.to_string())?;

        let size = tree.size();

        // Use viewBox dimensions as canvas size for full-resolution rendering.
        let (vb_w, vb_h) = parse_viewbox(bytes).unwrap_or((size.width(), size.height()));

        // usvg's abs_transform includes viewBox→size scaling. We need paths in
        // viewBox space, so compute the inverse scale to convert back.
        let to_vb_x = vb_w / size.width();
        let to_vb_y = vb_h / size.height();

        let mut doc = SvgDoc {
            width: vb_w,
            height: vb_h,
            bg_color: Color32::TRANSPARENT,
            paths: Vec::new(),
            file_path,
        };

        doc.extract_paths(tree.root(), to_vb_x, to_vb_y);

        Ok(doc)
    }

    /// Recursively extract paths from the SVG tree.
    fn extract_paths(&mut self, group: &resvg::usvg::Group, scale_x: f32, scale_y: f32) {
        for node in group.children() {
            match node {
                resvg::usvg::Node::Path(ref path) => {
                    self.add_path(path, scale_x, scale_y);
                }
                resvg::usvg::Node::Group(ref g) => {
                    self.extract_paths(g, scale_x, scale_y);
                }
                _ => {} // Skip images and text for now
            }
        }
    }

    /// Extract a single usvg::Path into our editable model.
    fn add_path(&mut self, path: &resvg::usvg::Path, scale_x: f32, scale_y: f32) {
        let id = path.id().to_string();

        // Extract fill color
        let fill_color = path.fill().and_then(|f| match f.paint() {
            resvg::usvg::Paint::Color(c) => {
                let opacity = f.opacity();
                Some(Color32::from_rgba_unmultiplied(
                    c.red,
                    c.green,
                    c.blue,
                    (opacity.get() * 255.0) as u8,
                ))
            }
            _ => None, // Gradients/patterns not supported in editor yet
        });

        // Extract stroke color
        let stroke_color = path.stroke().and_then(|s| match s.paint() {
            resvg::usvg::Paint::Color(c) => {
                let opacity = s.opacity();
                Some(Color32::from_rgba_unmultiplied(
                    c.red,
                    c.green,
                    c.blue,
                    (opacity.get() * 255.0) as u8,
                ))
            }
            _ => None,
        });

        let stroke_width = path
            .stroke()
            .map(|s| s.width().get())
            .unwrap_or(0.0);

        // Apply the path's absolute transform (includes parent group transforms)
        // to get coordinates in viewBox/canvas space.
        let ts = path.abs_transform();
        let transformed = path.data().clone().transform(ts);
        let path_data = transformed.as_ref().unwrap_or(path.data());

        // Extract path commands, scaling from size space to viewBox space
        let commands = path_data
            .segments()
            .map(|seg| match seg {
                resvg::tiny_skia::PathSegment::MoveTo(p) => {
                    PathCmd::MoveTo(p.x * scale_x, p.y * scale_y)
                }
                resvg::tiny_skia::PathSegment::LineTo(p) => {
                    PathCmd::LineTo(p.x * scale_x, p.y * scale_y)
                }
                resvg::tiny_skia::PathSegment::QuadTo(p0, p1) => {
                    PathCmd::QuadTo(
                        p0.x * scale_x, p0.y * scale_y,
                        p1.x * scale_x, p1.y * scale_y,
                    )
                }
                resvg::tiny_skia::PathSegment::CubicTo(p0, p1, p2) => {
                    PathCmd::CurveTo(
                        p0.x * scale_x, p0.y * scale_y,
                        p1.x * scale_x, p1.y * scale_y,
                        p2.x * scale_x, p2.y * scale_y,
                    )
                }
                resvg::tiny_skia::PathSegment::Close => PathCmd::Close,
            })
            .collect();

        self.paths.push(EditablePath {
            id,
            fill_color,
            stroke_color,
            stroke_width,
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
    }

    /// Export the document as an SVG string.
    pub fn to_svg_string(&self) -> String {
        let mut svg = String::new();
        svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
            self.width, self.height, self.width, self.height
        ));

        // Background rect (only if not transparent)
        if self.bg_color.a() > 0 {
            let bg_hex = color32_to_hex(self.bg_color);
            svg.push_str(&format!(
                "  <rect width=\"100%\" height=\"100%\" fill=\"{}\"/>\n",
                bg_hex
            ));
        }

        for path in &self.paths {
            // Wrap in <g> if path has transforms
            let has_transform = path.translate_x != 0.0
                || path.translate_y != 0.0
                || path.scale_x != 1.0
                || path.scale_y != 1.0
                || path.rotation != 0.0;

            if has_transform {
                // Compute pivot point from path bounding box
                let (cx, cy) = path_center(path);
                let pcx = cx + (path.pivot_x - 0.5) * bbox_width(path);
                let pcy = cy + (path.pivot_y - 0.5) * bbox_height(path);

                svg.push_str(&format!(
                    "  <g transform=\"translate({}, {}) rotate({}, {}, {}) scale({}, {})\">\n",
                    path.translate_x, path.translate_y,
                    path.rotation, pcx, pcy,
                    path.scale_x, path.scale_y,
                ));
            }

            svg.push_str(&format!("  <path id=\"{}\"", path.id));

            // Fill
            if let Some(fill) = path.fill_color {
                svg.push_str(&format!(" fill=\"{}\"", color32_to_hex(fill)));
            } else {
                svg.push_str(" fill=\"none\"");
            }

            // Stroke
            if let Some(stroke) = path.stroke_color {
                svg.push_str(&format!(
                    " stroke=\"{}\" stroke-width=\"{}\"",
                    color32_to_hex(stroke),
                    path.stroke_width
                ));
            }

            // Path data
            svg.push_str(" d=\"");
            for cmd in &path.commands {
                match cmd {
                    PathCmd::MoveTo(x, y) => svg.push_str(&format!("M {} {} ", x, y)),
                    PathCmd::LineTo(x, y) => svg.push_str(&format!("L {} {} ", x, y)),
                    PathCmd::CurveTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                        svg.push_str(&format!(
                            "C {} {} {} {} {} {} ",
                            cp1x, cp1y, cp2x, cp2y, x, y
                        ))
                    }
                    PathCmd::QuadTo(cpx, cpy, x, y) => {
                        svg.push_str(&format!("Q {} {} {} {} ", cpx, cpy, x, y))
                    }
                    PathCmd::Close => svg.push_str("Z "),
                }
            }
            svg.push_str("\"\n  />\n");

            if has_transform {
                svg.push_str("  </g>\n");
            }
        }

        svg.push_str("</svg>\n");
        svg
    }

    /// Save the SVG to a file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_svg_string())
    }

    /// Export as PNG to a file.
    pub fn export_png(&self, path: &std::path::Path) -> Result<(), String> {
        let pixmap = self.render_to_pixmap()?;
        let png_data = pixmap.encode_png().map_err(|e| e.to_string())?;
        std::fs::write(path, png_data).map_err(|e| e.to_string())
    }

    /// Export as JPG to a file.
    pub fn export_jpg(&self, path: &std::path::Path) -> Result<(), String> {
        let pixmap = self.render_to_pixmap()?;
        let w = pixmap.width();
        let h = pixmap.height();
        let rgba = pixmap.data();

        // Convert premultiplied RGBA to unpremultiplied RGB
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        for chunk in rgba.chunks(4) {
            let a = chunk[3] as f32 / 255.0;
            let r = if a > 0.0 { (chunk[0] as f32 / a) as u8 } else { 0 };
            let g = if a > 0.0 { (chunk[1] as f32 / a) as u8 } else { 0 };
            let b = if a > 0.0 { (chunk[2] as f32 / a) as u8 } else { 0 };
            rgb.push(r);
            rgb.push(g);
            rgb.push(b);
        }

        image::save_buffer(path, &rgb, w, h, image::ColorType::Rgb8)
            .map_err(|e| e.to_string())
    }

    /// Render the SVG to a tiny_skia Pixmap at the document's dimensions.
    fn render_to_pixmap(&self) -> Result<resvg::tiny_skia::Pixmap, String> {
        let w = (self.width as u32).max(1);
        let h = (self.height as u32).max(1);
        let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)
            .ok_or("Failed to create pixmap")?;

        // Fill background if not transparent
        if self.bg_color.a() > 0 {
            let c = resvg::tiny_skia::ColorU8::from_rgba(
                self.bg_color.r(),
                self.bg_color.g(),
                self.bg_color.b(),
                self.bg_color.a(),
            )
            .premultiply();
            pixmap.pixels_mut().fill(c);
        }

        // Render SVG
        let svg_string = self.to_svg_string();
        let tree = resvg::usvg::Tree::from_data(
            svg_string.as_bytes(),
            &resvg::usvg::Options::default(),
        )
        .map_err(|e| e.to_string())?;

        let tree_size = tree.size();
        let scale_x = w as f32 / tree_size.width();
        let scale_y = h as f32 / tree_size.height();
        let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        Ok(pixmap)
    }
}

/// Get the center of a path's bounding box.
fn path_center(path: &EditablePath) -> (f32, f32) {
    let (min_x, max_x, min_y, max_y) = path_bounds(path);
    ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0)
}

/// Get the width of a path's bounding box.
fn bbox_width(path: &EditablePath) -> f32 {
    let (min_x, max_x, _, _) = path_bounds(path);
    max_x - min_x
}

/// Get the height of a path's bounding box.
fn bbox_height(path: &EditablePath) -> f32 {
    let (_, _, min_y, max_y) = path_bounds(path);
    max_y - min_y
}

/// Compute bounding box (min_x, max_x, min_y, max_y) of path commands.
fn path_bounds(path: &EditablePath) -> (f32, f32, f32, f32) {
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

/// Convert a Color32 to a hex color string.
fn color32_to_hex(c: Color32) -> String {
    if c.a() == 255 {
        format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            c.r(),
            c.g(),
            c.b(),
            c.a()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewbox_parsing() {
        let bytes = std::fs::read("/home/flutter/source/SvgEditor/设备借还.svg").unwrap();
        let vb = parse_viewbox(&bytes);
        println!("viewBox: {:?}", vb);
        assert_eq!(vb, Some((1024.0, 1024.0)));
    }

    #[test]
    fn test_svg_doc_extraction() {
        let bytes = std::fs::read("/home/flutter/source/SvgEditor/设备借还.svg").unwrap();
        let doc = SvgDoc::from_bytes(&bytes, None).unwrap();
        println!("Doc: {}x{} paths={}", doc.width, doc.height, doc.paths.len());
        for (i, p) in doc.paths.iter().enumerate() {
            println!("  Path {}: id='{}' fill={:?} cmds={}", i, p.id, p.fill_color, p.commands.len());
            for (j, cmd) in p.commands.iter().enumerate().take(5) {
                println!("    cmd[{}]: {:?}", j, cmd);
            }
        }
        assert!(!doc.paths.is_empty(), "Should have extracted paths");
    }

    #[test]
    fn test_text_to_paths() {
        let paths = text_to_paths("Hello", 100.0, 100.0, 72.0, "sans-serif");
        println!("text_to_paths returned {} paths", paths.len());
        assert!(!paths.is_empty(), "Should have converted text to paths");
    }

    #[test]
    fn test_han_svg_with_group_transform() {
        let bytes = std::fs::read("/home/flutter/source/SvgEditor/han.svg").unwrap();
        let doc = SvgDoc::from_bytes(&bytes, None).unwrap();
        println!("Han SVG: {}x{} paths={}", doc.width, doc.height, doc.paths.len());
        for (i, p) in doc.paths.iter().enumerate().take(3) {
            let min_x = p.commands.iter().map(|c| match c {
                PathCmd::MoveTo(x, _) | PathCmd::LineTo(x, _) => *x,
                PathCmd::CurveTo(_, _, _, _, x, _) => *x,
                PathCmd::QuadTo(_, _, x, _) => *x,
                PathCmd::Close => f32::MAX,
            }).fold(f32::MAX, f32::min);
            let max_x = p.commands.iter().map(|c| match c {
                PathCmd::MoveTo(x, _) | PathCmd::LineTo(x, _) => *x,
                PathCmd::CurveTo(_, _, _, _, x, _) => *x,
                PathCmd::QuadTo(_, _, x, _) => *x,
                PathCmd::Close => f32::MIN,
            }).fold(f32::MIN, f32::max);
            let min_y = p.commands.iter().map(|c| match c {
                PathCmd::MoveTo(_, y) | PathCmd::LineTo(_, y) => *y,
                PathCmd::CurveTo(_, _, _, _, _, y) => *y,
                PathCmd::QuadTo(_, _, _, y) => *y,
                PathCmd::Close => f32::MAX,
            }).fold(f32::MAX, f32::min);
            let max_y = p.commands.iter().map(|c| match c {
                PathCmd::MoveTo(_, y) | PathCmd::LineTo(_, y) => *y,
                PathCmd::CurveTo(_, _, _, _, _, y) => *y,
                PathCmd::QuadTo(_, _, _, y) => *y,
                PathCmd::Close => f32::MIN,
            }).fold(f32::MIN, f32::max);
            println!("  Path {}: cmds={} x=[{:.1}, {:.1}] y=[{:.1}, {:.1}]",
                i, p.commands.len(), min_x, max_x, min_y, max_y);
        }
        assert!(!doc.paths.is_empty(), "Should have extracted paths from han.svg");
    }
}

/// Convert text to path commands using resvg's text-to-path conversion.
/// Returns a list of EditablePath, one per glyph or text run.
pub fn text_to_paths(text: &str, x: f32, y: f32, font_size: f32, font_family: &str) -> Vec<EditablePath> {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
            <text x="{}" y="{}" font-size="{}" font-family="{}" fill="black">{}</text>
        </svg>"#,
        x + font_size * text.len() as f32,
        y + font_size * 2.0,
        x + font_size * text.len() as f32,
        y + font_size * 2.0,
        x,
        y + font_size * 0.8,
        font_size,
        font_family,
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    );

    let mut opts = resvg::usvg::Options::default();
    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    opts.fontdb = std::sync::Arc::new(fontdb);

    let tree = match resvg::usvg::Tree::from_data(svg.as_bytes(), &opts) {
        Ok(tree) => tree,
        Err(e) => {
            log::error!("text_to_paths: failed to parse SVG: {}", e);
            return Vec::new();
        }
    };

    // usvg keeps text as Text nodes. Use to_string() to convert text to paths,
    // then re-parse the result to get Path nodes.
    let write_opts = resvg::usvg::WriteOptions {
        preserve_text: false, // Convert text to paths
        ..Default::default()
    };
    let svg_out = tree.to_string(&write_opts);

    let tree2 = match resvg::usvg::Tree::from_data(svg_out.as_bytes(), &opts) {
        Ok(tree) => tree,
        Err(e) => {
            log::error!("text_to_paths: failed to re-parse SVG: {}", e);
            return Vec::new();
        }
    };

    let mut paths = Vec::new();
    extract_text_paths(tree2.root(), &mut paths);
    paths
}

/// Recursively extract paths from usvg tree (for text-to-path).
fn extract_text_paths(group: &resvg::usvg::Group, paths: &mut Vec<EditablePath>) {
    for node in group.children() {
        match node {
            resvg::usvg::Node::Path(path) => {
                let commands: Vec<PathCmd> = path
                    .data()
                    .segments()
                    .map(|seg| match seg {
                        resvg::tiny_skia::PathSegment::MoveTo(p) => PathCmd::MoveTo(p.x, p.y),
                        resvg::tiny_skia::PathSegment::LineTo(p) => PathCmd::LineTo(p.x, p.y),
                        resvg::tiny_skia::PathSegment::QuadTo(p0, p1) => {
                            PathCmd::QuadTo(p0.x, p0.y, p1.x, p1.y)
                        }
                        resvg::tiny_skia::PathSegment::CubicTo(p0, p1, p2) => {
                            PathCmd::CurveTo(p0.x, p0.y, p1.x, p1.y, p2.x, p2.y)
                        }
                        resvg::tiny_skia::PathSegment::Close => PathCmd::Close,
                    })
                    .collect();

                if !commands.is_empty() {
                    paths.push(EditablePath {
                        id: String::new(),
                        fill_color: Some(Color32::BLACK),
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
                }
            }
            resvg::usvg::Node::Group(g) => {
                extract_text_paths(g, paths);
            }
            _ => {}
        }
    }
}

/// Parse the viewBox from raw SVG bytes. Returns (width, height) of the viewBox.
fn parse_viewbox(bytes: &[u8]) -> Option<(f32, f32)> {
    let text = std::str::from_utf8(bytes).ok()?;
    // Find viewBox="x y w h" attribute (case-insensitive for the attribute name)
    let start = text.find("viewBox=\"").or_else(|| text.find("viewbox=\""))?;
    let after = &text[start + 9..]; // skip 'viewBox="'
    let end = after.find('"')?;
    let value = &after[..end];
    let parts: Vec<f32> = value
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() >= 4 {
        Some((parts[2], parts[3]))
    } else {
        None
    }
}
