use eframe::emath::Align;
use egui::{Color32, Pos2, Shape, TextStyle, Ui, Vec2};
use epaint::{CubicBezierShape, PathStroke, TextShape};
use epaint::text::LayoutJob;

const MAX_ARC_ANGLE: f32 = std::f32::consts::PI / 2.0;

const TEXT_BOX_CORNER_RADIUS: f32 = 5.0;
const TEXT_BOX_PADDING: f32 = 5.0;
const TEXT_BOX_WRAP_WIDTH: f32 = 250.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

pub fn get_path_for_semicircle(center: Pos2, radius: f32, facing_angle: f32, half_arc_angle: f32, inverted: bool) -> Vec<Pos2> {
    let mut path = vec![center];
    if radius <= 0.0 {
        return path;
    }

    let mut min_angle = facing_angle - half_arc_angle;
    let mut max_angle = facing_angle + half_arc_angle;

    if inverted {
        let new_max = min_angle + std::f32::consts::TAU;
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

pub fn text_box<T: Into<String>>(text: T, pos: Pos2, valign: VAlign, bg_color: Color32, text_color: Color32, ui: &Ui) -> (Shape, Shape) {
    let text = text.into();
    let font_id = TextStyle::Body.resolve(&*ui.style());

    let text_shape = ui.fonts(|fonts| {
        let mut job = LayoutJob::simple(
            text,
            font_id,
            text_color,
            TEXT_BOX_WRAP_WIDTH,
        );
        job.halign = Align::Center;

        let galley = fonts.layout_job(job);
        let offset = Vec2::new(0.0, match valign {
            VAlign::Bottom => galley.rect.height(),
            VAlign::Center => galley.rect.height() / 2.0,
            VAlign::Top => 0.0,       
        });

        Shape::Text(TextShape::new(
            pos - offset,
            galley,
            bg_color,
        ))
    });

    let bg_rect = text_shape.visual_bounding_rect().expand(TEXT_BOX_PADDING);
    let text_bg_shape = Shape::rect_filled(bg_rect, TEXT_BOX_CORNER_RADIUS, bg_color);

    (text_bg_shape, text_shape)
}