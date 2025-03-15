use crate::math::{Fixed12, UFixed12};

const HIGHLIGHT_MAX_INTENSITY: f32 = 0.5;
const HIGHLIGHT: egui::Rgba = egui::Rgba::from_rgba_premultiplied(0.25, 0.25, 0.25, 0.0);
const HIGHLIGHT_STROKE: f32 = 2.0;
const HIGHLIGHT_ALPHA: f32 = 1.5;

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

    const fn is_stroke(&self) -> bool {
        self.stroke.width > 0.0 && self.stroke.color.a() > 0
    }

    const fn color(&self) -> egui::Color32 {
        if self.is_stroke() {
            self.stroke.color
        } else {
            self.fill_color
        }
    }

    const fn set_color(&mut self, color: egui::Color32) {
        if self.is_stroke() {
            self.stroke.color = color;
        } else {
            self.fill_color = color;
        }
    }

    pub fn highlight(&mut self) {
        let rgba: egui::Rgba = self.color().into();
        let mut highlighted = (rgba + HIGHLIGHT).multiply(HIGHLIGHT_ALPHA);
        let intensity = highlighted.intensity();
        if intensity > HIGHLIGHT_MAX_INTENSITY {
            highlighted = highlighted * (HIGHLIGHT_MAX_INTENSITY / intensity);
        }
        
        self.set_color(highlighted.into());
        if self.is_stroke() {
            self.stroke.width *= HIGHLIGHT_STROKE;
        }
    }
}

#[derive(Debug)]
pub struct RectCollider {
    x: Fixed12,
    z: Fixed12,
    width: UFixed12,
    height: UFixed12,
    corner_radius: f32,
}

impl RectCollider {
    pub fn new(x: Fixed12, z: Fixed12, width: UFixed12, height: UFixed12, corner_radius: f32) -> Self {
        Self { x, z, width, height, corner_radius }
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.x, self.z, self.width, self.height);
        // TODO: verify in-game whether the corners are actually rounded or if they're sharply cut the way they appear
        //  in RE2RDTE
        let corner_radius = epaint::CornerRadiusF32::same(self.corner_radius * draw_params.scale);

        egui::Shape::Rect(epaint::RectShape::new(
            egui::Rect {
                min: egui::Pos2 { x, y },
                max: egui::Pos2 { x: x + width, y: y + height },
            },
            corner_radius,
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

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
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

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
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

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
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

#[derive(Debug)]
pub struct QuadCollider {
    x1: Fixed12,
    z1: Fixed12,
    x2: Fixed12,
    z2: Fixed12,
    x3: Fixed12,
    z3: Fixed12,
    x4: Fixed12,
    z4: Fixed12,
}

impl QuadCollider {
    pub fn new(x1: Fixed12, z1: Fixed12, x2: Fixed12, z2: Fixed12, x3: Fixed12, z3: Fixed12, x4: Fixed12, z4: Fixed12) -> Self {
        Self { x1, z1, x2, z2, x3, z3, x4, z4 }
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let x1 = self.x1 * draw_params.scale - draw_params.origin.x;
        let y1 = -self.z1 * draw_params.scale - draw_params.origin.y;
        let x2 = self.x2 * draw_params.scale - draw_params.origin.x;
        let y2 = -self.z2 * draw_params.scale - draw_params.origin.y;
        let x3 = self.x3 * draw_params.scale - draw_params.origin.x;
        let y3 = -self.z3 * draw_params.scale - draw_params.origin.y;
        let x4 = self.x4 * draw_params.scale - draw_params.origin.x;
        let y4 = -self.z4 * draw_params.scale - draw_params.origin.y;

        egui::Shape::Path(epaint::PathShape {
            points: vec![
                egui::Pos2::new(x1, y1),
                egui::Pos2::new(x2, y2),
                egui::Pos2::new(x3, y3),
                egui::Pos2::new(x4, y4),
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
pub enum Collider {
    Rect(RectCollider),
    Diamond(DiamondCollider),
    Ellipse(EllipseCollider),
    Triangle(TriangleCollider),
    Quad(QuadCollider),
}

impl Collider {
    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        match self {
            Self::Rect(rect) => rect.gui_shape(draw_params),
            Self::Diamond(diamond) => diamond.gui_shape(draw_params),
            Self::Ellipse(ellipse) => ellipse.gui_shape(draw_params),
            Self::Triangle(triangle) => triangle.gui_shape(draw_params),
            Self::Quad(quad) => quad.gui_shape(draw_params),
        }
    }
}