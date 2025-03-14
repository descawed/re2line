use crate::math::{Fixed12, UFixed12};

#[derive(Debug)]
pub struct DrawParams {
    pub origin: egui::Pos2,
    pub scale: f32,
    pub fill_color: egui::Color32,
    pub stroke: egui::Stroke,
    pub stroke_kind: egui::StrokeKind,
}

pub trait Collider {
    fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape;
}

#[derive(Debug)]
pub struct RectCollider {
    x: Fixed12,
    z: Fixed12,
    width: UFixed12,
    height: UFixed12,
}

impl RectCollider {
    pub fn new(x: Fixed12, z: Fixed12, width: UFixed12, height: UFixed12) -> Self {
        Self { x, z, width, height }
    }
}

impl Collider for RectCollider {
    fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let x = self.x * draw_params.scale - draw_params.origin.x;
        let y = -(self.z + self.height) * draw_params.scale - draw_params.origin.y;
        let width = self.width * draw_params.scale;
        let height = self.height * draw_params.scale;

        egui::Shape::Rect(epaint::RectShape::new(
            egui::Rect {
                min: egui::Pos2 { x, y },
                max: egui::Pos2 { x: x + width, y: y + height },
            },
            epaint::CornerRadius::ZERO,
            draw_params.fill_color,
            draw_params.stroke,
            draw_params.stroke_kind,
        ))
    }
}

#[derive(Debug)]
pub struct EllipseCollider {
    x: Fixed12,
    z: Fixed12,
    width: UFixed12,
    height: UFixed12,
}

impl EllipseCollider {
    pub fn new(x: Fixed12, z: Fixed12, width: UFixed12, height: UFixed12) -> Self {
        Self { x, z, width, height }
    }
}

impl Collider for EllipseCollider {
    fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let x = self.x * draw_params.scale - draw_params.origin.x;
        let y = -(self.z + self.height) * draw_params.scale - draw_params.origin.y;
        let width = self.width * draw_params.scale;
        let height = self.height * draw_params.scale;

        let radius_x = width / 2.0;
        let radius_y = height / 2.0;
        let center_x = x + radius_x;
        let center_y = y + radius_y;

        egui::Shape::Ellipse(epaint::EllipseShape {
            center: egui::Pos2::new(center_x, center_y),
            radius: egui::Vec2::new(radius_x, radius_y),
            fill: draw_params.fill_color,
            stroke: draw_params.stroke,
        })
    }
}