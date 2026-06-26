use egui::Pos2;

use crate::canvas::{screen_to_svg, CanvasState, DragState};
use crate::svg_doc::{PathCmd, SvgDoc};

/// Apply dragging to the path commands.
/// Called each frame when a drag is in progress.
pub fn apply_drag(
    doc: &mut SvgDoc,
    state: &CanvasState,
    canvas_rect: egui::Rect,
    delta: egui::Vec2,
) {
    if let Some(ref drag) = state.dragging {
        // Calculate zoom factor first (borrows doc immutably)
        let zoom_factor = calculate_zoom_factor(doc, canvas_rect, state);
        let screen_delta = egui::vec2(delta.x / zoom_factor, delta.y / zoom_factor);

        // Then mutate the path
        if let Some(path) = doc.paths.get_mut(drag.path_idx) {
            // Reverse the path's own transform so control points stay
            // visually attached to the rendered path.
            let svg_delta = reverse_transform_delta(screen_delta, path.scale_x, path.scale_y, path.rotation);
            apply_delta_to_cmd(&mut path.commands, drag, svg_delta);
        }
    }
}

/// Handle click-to-select on the canvas.
/// Returns the index of the path that was clicked, if any.
pub fn hit_test(
    doc: &SvgDoc,
    state: &CanvasState,
    canvas_rect: egui::Rect,
    click_pos: Pos2,
    tolerance: f32,
) -> Option<usize> {
    let svg_pos = screen_to_svg(click_pos, canvas_rect, doc, state);
    let zoom_factor = calculate_zoom_factor(doc, canvas_rect, state);
    let svg_tolerance = tolerance / zoom_factor;

    // Test each path in reverse order (topmost first)
    for (i, path) in doc.paths.iter().enumerate().rev() {
        if path_hit_test(path, svg_pos, svg_tolerance) {
            return Some(i);
        }
    }

    None
}

/// Check if a point is near any segment of the path.
fn path_hit_test(
    path: &crate::svg_doc::EditablePath,
    point: Pos2,
    tolerance: f32,
) -> bool {
    let mut last_pos = Pos2::ZERO;

    for cmd in &path.commands {
        match cmd {
            PathCmd::MoveTo(x, y) => {
                last_pos = Pos2::new(*x, *y);
            }
            PathCmd::LineTo(x, y) => {
                let end = Pos2::new(*x, *y);
                if point_to_line_distance(point, last_pos, end) < tolerance {
                    return true;
                }
                last_pos = end;
            }
            PathCmd::CurveTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                let cp1 = Pos2::new(*cp1x, *cp1y);
                let cp2 = Pos2::new(*cp2x, *cp2y);
                let end = Pos2::new(*x, *y);
                if point_to_cubic_bezier_distance(point, last_pos, cp1, cp2, end) < tolerance {
                    return true;
                }
                last_pos = end;
            }
            PathCmd::QuadTo(cpx, cpy, x, y) => {
                let cp = Pos2::new(*cpx, *cpy);
                let end = Pos2::new(*x, *y);
                if point_to_quad_bezier_distance(point, last_pos, cp, end) < tolerance {
                    return true;
                }
                last_pos = end;
            }
            PathCmd::Close => {}
        }
    }

    // Also check fill: if the path is filled and the point is inside
    if path.fill_color.is_some() && point_in_path(path, point) {
        return true;
    }

    false
}

/// Simple point-in-polygon test (ray casting).
fn point_in_path(path: &crate::svg_doc::EditablePath, point: Pos2) -> bool {
    let mut inside = false;
    let mut last_pos = Pos2::ZERO;
    let mut first_pos = Pos2::ZERO;

    for cmd in &path.commands {
        match cmd {
            PathCmd::MoveTo(x, y) => {
                last_pos = Pos2::new(*x, *y);
                first_pos = last_pos;
            }
            PathCmd::LineTo(x, y) => {
                let end = Pos2::new(*x, *y);
                if ray_cast_test(point, last_pos, end) {
                    inside = !inside;
                }
                last_pos = end;
            }
            PathCmd::CurveTo(_, _, _, _, x, y) => {
                let end = Pos2::new(*x, *y);
                if ray_cast_test(point, last_pos, end) {
                    inside = !inside;
                }
                last_pos = end;
            }
            PathCmd::QuadTo(_, _, x, y) => {
                let end = Pos2::new(*x, *y);
                if ray_cast_test(point, last_pos, end) {
                    inside = !inside;
                }
                last_pos = end;
            }
            PathCmd::Close => {
                if ray_cast_test(point, last_pos, first_pos) {
                    inside = !inside;
                }
                last_pos = first_pos;
            }
        }
    }

    inside
}

/// Ray casting test: does a horizontal ray from `point` to +infinity intersect the segment?
fn ray_cast_test(point: Pos2, seg_start: Pos2, seg_end: Pos2) -> bool {
    let (y_min, y_max) = if seg_start.y < seg_end.y {
        (seg_start.y, seg_end.y)
    } else {
        (seg_end.y, seg_start.y)
    };

    if point.y < y_min || point.y >= y_max {
        return false;
    }

    let x_intersect = seg_start.x
        + (point.y - seg_start.y) / (seg_end.y - seg_start.y) * (seg_end.x - seg_start.x);

    x_intersect > point.x
}

/// Apply a delta to a specific control point in the path.
fn apply_delta_to_cmd(commands: &mut [PathCmd], drag: &DragState, delta: egui::Vec2) {
    if let Some(cmd) = commands.get_mut(drag.cmd_idx) {
        match cmd {
            PathCmd::MoveTo(ref mut x, ref mut y) => {
                *x += delta.x;
                *y += delta.y;
            }
            PathCmd::LineTo(ref mut x, ref mut y) => {
                *x += delta.x;
                *y += delta.y;
            }
            PathCmd::CurveTo(
                ref mut cp1x,
                ref mut cp1y,
                ref mut cp2x,
                ref mut cp2y,
                ref mut x,
                ref mut y,
            ) => match drag.point_role {
                0 => {
                    *cp1x += delta.x;
                    *cp1y += delta.y;
                }
                1 => {
                    *cp2x += delta.x;
                    *cp2y += delta.y;
                }
                2 => {
                    *x += delta.x;
                    *y += delta.y;
                }
                _ => {}
            },
            PathCmd::QuadTo(ref mut cpx, ref mut cpy, ref mut x, ref mut y) => {
                match drag.point_role {
                    0 => {
                        *cpx += delta.x;
                        *cpy += delta.y;
                    }
                    1 => {
                        *x += delta.x;
                        *y += delta.y;
                    }
                    _ => {}
                }
            }
            PathCmd::Close => {}
        }
    }
}

/// Calculate the total zoom factor (SVG to screen).
fn calculate_zoom_factor(doc: &SvgDoc, canvas_rect: egui::Rect, state: &CanvasState) -> f32 {
    let svg_size = egui::vec2(doc.width, doc.height);
    let fit_scale = (canvas_rect.width() / svg_size.x)
        .min(canvas_rect.height() / svg_size.y)
        * 0.9;
    fit_scale * state.zoom
}

/// Distance from a point to a line segment.
fn point_to_line_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let ab_len_sq = ab.dot(ab);
    if ab_len_sq < f32::EPSILON {
        return (p - a).length();
    }
    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

/// Distance from a point to a cubic bezier curve (approximated).
fn point_to_cubic_bezier_distance(
    p: Pos2,
    p0: Pos2,
    p1: Pos2,
    p2: Pos2,
    p3: Pos2,
) -> f32 {
    let mut min_dist = f32::MAX;
    let steps = 30;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let pt = cubic_bezier_sample(p0, p1, p2, p3, t);
        let dist = (p - pt).length();
        min_dist = min_dist.min(dist);
    }
    min_dist
}

/// Distance from a point to a quadratic bezier curve (approximated).
fn point_to_quad_bezier_distance(p: Pos2, p0: Pos2, p1: Pos2, p2: Pos2) -> f32 {
    let mut min_dist = f32::MAX;
    let steps = 20;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let pt = quad_bezier_sample(p0, p1, p2, t);
        let dist = (p - pt).length();
        min_dist = min_dist.min(dist);
    }
    min_dist
}

fn cubic_bezier_sample(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    Pos2::new(
        mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
        mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
    )
}

fn quad_bezier_sample(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
    let mt = 1.0 - t;
    Pos2::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

/// Reverse the path's scale and rotation on a delta vector.
/// SVG applies: scale(origin) → rotate(pivot) → translate.
/// For deltas, translation is ignored. Reverse order: inverse_rotate → inverse_scale.
fn reverse_transform_delta(
    delta: egui::Vec2,
    scale_x: f32,
    scale_y: f32,
    rotation_deg: f32,
) -> egui::Vec2 {
    let mut dx = delta.x;
    let mut dy = delta.y;

    // 1. Inverse rotation
    if rotation_deg.abs() > 0.001 {
        let angle = -rotation_deg.to_radians();
        let cos = angle.cos();
        let sin = angle.sin();
        let new_dx = dx * cos - dy * sin;
        let new_dy = dx * sin + dy * cos;
        dx = new_dx;
        dy = new_dy;
    }

    // 2. Inverse scale
    let sx = if scale_x.abs() > 0.001 { 1.0 / scale_x } else { 1.0 };
    let sy = if scale_y.abs() > 0.001 { 1.0 / scale_y } else { 1.0 };
    dx *= sx;
    dy *= sy;

    egui::vec2(dx, dy)
}
