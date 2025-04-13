use egui::{Color32, Pos2, Vec2};
use epaint::{CubicBezierShape, PathStroke};

const MAX_ARC_ANGLE: f32 = std::f32::consts::PI / 2.0;

pub fn get_path_for_semicircle(center: Pos2, radius: f32, facing_angle: f32, half_arc_angle: f32, inverted: bool) -> Vec<Pos2> {
    let mut path = vec![center];
    if radius <= 0.0 {
        return path;
    }

    let mut min_angle = facing_angle - half_arc_angle;
    let mut max_angle = facing_angle + half_arc_angle;

    if inverted {
        let new_max = min_angle + std::f32::consts::PI * 2.0;
        min_angle = max_angle;
        max_angle = new_max;
    }

    let mut start_angle = min_angle;
    while start_angle < max_angle {
        let arc_angle = (max_angle - start_angle).min(MAX_ARC_ANGLE);
        let end_angle = start_angle + arc_angle;

        let p0 = center + Vec2::angled(start_angle) * radius;
        let p3 = center + Vec2::angled(end_angle) * radius;

        let d = radius * (arc_angle / 4.0).tan() * 4.0 / 3.0;

        let start_tangent = Vec2::new(-start_angle.sin(), start_angle.cos());
        let end_tangent = Vec2::new(-end_angle.sin(), end_angle.cos());

        let p1 = p0 + start_tangent * d;
        let p2 = p3 - end_tangent * d;

        let bezier = CubicBezierShape::from_points_stroke(
            [p0, p1, p2, p3],
            true,
            Color32::WHITE,
            PathStroke::new(1.0, Color32::WHITE),
        );
        path.extend(bezier.flatten(Some(0.1)));

        start_angle = end_angle;
    }

    path
}