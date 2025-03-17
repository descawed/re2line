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
    pub fn describe(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = Vec::new();
        
        // type
        groups.push((String::from("Type"), vec![String::from(match self {
            Self::Rect(rect) => {
                if rect.corner_radius > 0.0 {
                    "Rectangle (rounded)"
                } else {
                    "Rectangle"
                }
            }
            Self::Diamond(_) => "Diamond",
            Self::Ellipse(_) => "Ellipse",
            Self::Triangle(_) => "Triangle",
            Self::Quad(_) => "Quadrilateral",
        })]));

        // basic shape parameters
        let label = String::from("Params");
        match self {
            Self::Quad(quad) => {
                groups.push((label, vec![
                    format!("X1: {}", quad.x1),
                    format!("Z1: {}", quad.z1),
                    format!("X2: {}", quad.x2),
                    format!("Z2: {}", quad.z2),
                    format!("X3: {}", quad.x3),
                    format!("Z3: {}", quad.z3),
                    format!("X4: {}", quad.x4),
                    format!("Z4: {}", quad.z4),
                ]));
            }
            Self::Rect(RectCollider { x, z, width, height, .. })
            | Self::Diamond(DiamondCollider { x, z, width, height })
            | Self::Ellipse(EllipseCollider { x, z, width, height })
            | Self::Triangle(TriangleCollider { x, z, width, height, .. })
            => {
                groups.push((label, vec![
                    format!("X: {}", x),
                    format!("Z: {}", z),
                    format!("W: {}", width),
                    format!("H: {}", height),
                ]));
            }
        }
        
        // calculated geometry where it might be useful
        let label = String::from("Calculated");
        match self {
            Self::Ellipse(ellipse) => {
                let x_radius = ellipse.width >> 1;
                let z_radius = ellipse.height >> 1;
                let center_x = ellipse.x + x_radius;
                let center_z = ellipse.z + z_radius;
                
                groups.push((label, vec![
                    format!("CX: {}", center_x),
                    format!("CZ: {}", center_z),
                    format!("RX: {}", x_radius),
                    format!("RZ: {}", z_radius),
                ]));
            }
            Self::Triangle(tri) => {
                let x1 = tri.x + if tri.offsets[0].0 > 0.0 { tri.width } else { UFixed12(0) };
                let z1 = tri.z + if tri.offsets[0].1 > 0.0 { tri.height } else { UFixed12(0) };
                let x2 = tri.x + if tri.offsets[1].0 > 0.0 { tri.width } else { UFixed12(0) };
                let z2 = tri.z + if tri.offsets[1].1 > 0.0 { tri.height } else { UFixed12(0) };
                let x3 = tri.x + if tri.offsets[2].0 > 0.0 { tri.width } else { UFixed12(0) };
                let z3 = tri.z + if tri.offsets[2].1 > 0.0 { tri.height } else { UFixed12(0) };
                
                groups.push((label, vec![
                    format!("X1: {}", x1),
                    format!("Z1: {}", z1),
                    format!("X2: {}", x2),
                    format!("Z2: {}", z2),
                    format!("X3: {}", x3),
                    format!("Z3: {}", z3),
                ]));
            }
            Self::Diamond(diamond) => {
                let radius_x = diamond.width >> 1;
                let radius_z = diamond.height >> 1;
                
                groups.push((label, vec![
                    format!("X1: {}", diamond.x + radius_x),
                    format!("Z1: {}", diamond.z),
                    format!("X2: {}", diamond.x + diamond.width),
                    format!("Z2: {}", diamond.z + radius_z),
                    format!("X3: {}", diamond.x + radius_x),
                    format!("Z3: {}", diamond.z + diamond.height),
                    format!("X4: {}", diamond.x),
                    format!("Z4: {}", diamond.z + radius_z),   
                ]));
            }
            Self::Rect(rect) => {
                let nx = rect.x;
                let nz = rect.z;
                let fx = rect.x + rect.width;
                let fz = rect.z + rect.height;
                
                groups.push((label, vec![
                    format!("X2: {}", fx),
                    format!("Z2: {}", nz),
                    format!("X3: {}", fx),
                    format!("Z3: {}", fz),
                    format!("X4: {}", nx),
                    format!("Z4: {}", fz),
                ]));
            }
            Self::Quad(_) => {} // no need for calculated for quad since all points are included in params
        }

        groups
    }

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