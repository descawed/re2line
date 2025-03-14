use crate::math::{Fixed12, UFixed12};

#[derive(Debug)]
pub struct DrawParams {
    pub origin: egui::Pos2,
    pub scale: f32,
    pub fill_color: egui::Color32,
    pub stroke: egui::Stroke,
    pub stroke_kind: egui::StrokeKind,
}

impl DrawParams {
    fn transform(&self, x: Fixed12, z: Fixed12, w: UFixed12, h: UFixed12) -> (f32, f32, f32, f32) {
        (
            x * self.scale - self.origin.x,
            -(z + h) * self.scale - self.origin.y,
            w * self.scale,
            h * self.scale,
        )
    }
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
        let (x, y, width, height) = draw_params.transform(self.x, self.z, self.width, self.height);

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
pub struct DiamondCollider {
    x: Fixed12,
    z: Fixed12,
    width: UFixed12,
    height: UFixed12,
}

impl DiamondCollider {
    pub fn new(x: Fixed12, z: Fixed12, width: UFixed12, height: UFixed12) -> Self {
        Self { x, z, width, height }
    }
}

impl Collider for DiamondCollider {
    fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.x, self.z, self.width, self.height);
        let x_radius = width / 2.0;
        let y_radius = height / 2.0;

        egui::Shape::Path(epaint::PathShape {
            points: vec![
                egui::Pos2::new(x + x_radius, y),
                egui::Pos2::new(x + width, y + y_radius),
                egui::Pos2::new(x + x_radius, y + height),
                egui::Pos2::new(x, y + y_radius),
            ],
            closed: true,
            fill: draw_params.fill_color,
            stroke: epaint::PathStroke {
                width: draw_params.stroke.width,
                color: epaint::ColorMode::Solid(draw_params.stroke.color),
                kind: draw_params.stroke_kind,
            },
        })
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
        let (x, y, width, height) = draw_params.transform(self.x, self.z, self.width, self.height);

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

#[derive(Debug)]
pub struct TriangleCollider {
    x: Fixed12,
    z: Fixed12,
    width: UFixed12,
    height: UFixed12,
    offsets: [(f32, f32); 3],
}

impl TriangleCollider {
    pub fn new(x: Fixed12, z: Fixed12, width: UFixed12, height: UFixed12, offsets: [(f32, f32); 3]) -> Self {
        Self { x, z, width, height, offsets }
    }
}

impl Collider for TriangleCollider {
    fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.x, self.z, self.width, self.height);

        let x1 = x + self.offsets[0].0 * width;
        let y1 = y + self.offsets[0].1 * height;
        let x2 = x + self.offsets[1].0 * width;
        let y2 = y + self.offsets[1].1 * height;
        let x3 = x + self.offsets[2].0 * width;
        let y3 = y + self.offsets[2].1 * height;

        egui::Shape::Path(epaint::PathShape {
            points: vec![
                egui::Pos2::new(x1, y1),
                egui::Pos2::new(x2, y2),
                egui::Pos2::new(x3, y3),
            ],
            closed: true,
            fill: draw_params.fill_color,
            stroke: epaint::PathStroke {
                width: draw_params.stroke.width,
                color: epaint::ColorMode::Solid(draw_params.stroke.color),
                kind: draw_params.stroke_kind,
            },
        })
    }
}