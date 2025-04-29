use crate::math::{Fixed32, Vec2};

const HIGHLIGHT_MAX_INTENSITY: f32 = 0.5;
const HIGHLIGHT: egui::Rgba = egui::Rgba::from_rgba_premultiplied(0.25, 0.25, 0.25, 0.0);
const HIGHLIGHT_STROKE: f32 = 2.0;
const HIGHLIGHT_ALPHA: f32 = 1.5;

#[derive(Debug, Clone)]
pub struct DrawParams {
    pub origin: egui::Pos2,
    pub scale: f32,
    pub fill_color: egui::Color32,
    pub stroke: egui::Stroke,
    pub stroke_kind: egui::StrokeKind,
}

impl DrawParams {
    pub fn transform<T, U, V, W>(&self, x: T, z: U, w: V, h: W) -> (f32, f32, f32, f32)
    where T: Into<Fixed32>, U: Into<Fixed32>, V: Into<Fixed32>, W: Into<Fixed32>
    {
        let h = h.into();
        let z_f32 = (z.into() + h).to_f32();
        (
            x.into() * self.scale - self.origin.x,
            -z_f32 * self.scale - self.origin.y,
            w.into() * self.scale,
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

    pub fn outline(&mut self) {
        if self.is_stroke() {
            return;
        }

        self.stroke.color = egui::Color32::BLACK;
        self.stroke.width = HIGHLIGHT_STROKE;
    }
}

const fn rect_contains_point(pos: Vec2, size: Vec2, point: Vec2) -> bool {
    point.x.0 >= pos.x.0 && point.x.0 < pos.x.0 + size.x.0 && point.z.0 >= pos.z.0 && point.z.0 < pos.z.0 + size.z.0
}

#[derive(Debug, Clone)]
pub struct RectCollider {
    pos: Vec2,
    size: Vec2,
    corner_radius: f32,
}

impl RectCollider {
    pub const fn new(x: Fixed32, z: Fixed32, width: Fixed32, height: Fixed32, corner_radius: f32) -> Self {
        Self {
            pos: Vec2 { x, z },
            size: Vec2 { x: width, z: height },
            corner_radius,
        }
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.x, self.pos.z, self.size.x, self.size.z);
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

    pub fn contains_point<T: Into<Vec2>>(&self, point: T) -> bool {
        // TODO: implement capsule logic
        rect_contains_point(self.pos, self.size, point.into())
    }

    pub fn set_pos<T: Into<Vec2>>(&mut self, pos: T) {
        self.pos = pos.into();
    }

    pub fn set_size<T: Into<Vec2>>(&mut self, size: T) {
        self.size = size.into();
    }
}

#[derive(Debug)]
pub struct DiamondCollider {
    pos: Vec2,
    size: Vec2,
}

impl DiamondCollider {
    pub const fn new(x: Fixed32, z: Fixed32, width: Fixed32, height: Fixed32) -> Self {
        Self { pos: Vec2 { x, z }, size: Vec2 { x: width, z: height } }
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.x, self.pos.z, self.size.x, self.size.z);
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

    pub fn contains_point<T: Into<Vec2>>(&self, point: T) -> bool {
        let point = point.into();

        let center_x = (self.size.x >> 1) + self.pos.x;
        let center_z = (self.size.z >> 1) + self.pos.z;

        let quadrant = (((point.z - center_z) >> 0x1e).0 & 2) | ((point.x - center_x) >> 0x1f).0;
        match quadrant {
            0 => self.is_point_in_quadrant0(point),
            1 => self.is_point_in_quadrant1(point),
            2 => self.is_point_in_quadrant2(point),
            3 => self.is_point_in_quadrant3(point),
            _ => unreachable!(),
        }
    }

    const fn is_point_in_quadrant0(&self, point: Vec2) -> bool {
        let center_x = (self.size.x.0 >> 1) + self.pos.x.0;
        let center_z = (self.size.z.0 >> 1) + self.pos.z.0;

        let far_x = self.pos.x.0 + self.size.x.0;
        let far_z = self.pos.z.0 + self.size.z.0;

        let px = point.x.0;
        let pz = point.z.0;

        let x_diff1 = far_x - center_x;
        let x_diff2 = px - center_x;

        let z_diff1 = center_z - far_z;
        let z_diff2 = pz - far_z;

        let term1 = (x_diff2 * z_diff1) / x_diff1;

        if term1 <= z_diff2 {
            return false;
        }

        let z_diff3 = far_z - center_z;

        let term2 = z_diff2 - term1;
        let term3 = (x_diff1 * term2) / z_diff3;

        let term4 = term2 * term2 + term3 * term3;

        let term5 = (term3 * term2 * term2) / term4;
        let term6 = (term3 * term3 * term2) / term4;

        let term5_sign = term5 >> 0x1f;
        let term6_sign = term6 >> 0x1f;

        if (term5 ^ term5_sign) - term5_sign < 0x191 && (term6 ^ term6_sign) - term6_sign < 0x191 {
            let new_x = px - term5;
            let new_z = pz - term6;

            if (new_x / 0x12 - center_x / 0x12) * (center_z / 0x12 - far_z / 0x12) - (new_z / 0x12 - far_z / 0x12) * (far_x / 0x12 - center_x / 0x12) < 1 {
                return true;
            }
        }

        false
    }

    const fn is_point_in_quadrant1(&self, point: Vec2) -> bool {
        let x = self.pos.x.0;

        let center_x = (self.size.x.0 >> 1) + self.pos.x.0;
        let center_z = (self.size.z.0 >> 1) + self.pos.z.0;

        let far_z = self.pos.z.0 + self.size.z.0;

        let px = point.x.0;
        let pz = point.z.0;

        let x_diff1 = center_x - x;
        let x_diff2 = px - x;

        let z_diff1 = far_z - center_z;
        let z_diff2 = pz - center_z;

        let term1 = (x_diff2 * z_diff1) / x_diff1;

        if term1 <= z_diff2 {
            return false;
        }

        let term2 = z_diff2 - term1;
        let term3 = (x_diff1 * term2) / z_diff1;

        let term4 = term2 * term2 + term3 * term3;

        let term5 = (term3 * term2 * term2) / term4;
        let term6 = (term3 * term3 * term2) / term4;

        let term5_sign = term5 >> 0x1f;
        let term6_sign = term6 >> 0x1f;

        if (term5 ^ term5_sign) - term5_sign < 0x191 && (term6 ^ term6_sign) - term6_sign < 0x191 {
            let new_x = px + term5;
            let new_z = pz - term6;

            let x_div = x / 0x12;

            if (new_x / 0x12 - x_div) * (far_z / 0x12 - center_z / 0x12) - (new_z / 0x12 - center_z / 0x12) * (center_x / 0x12 - x_div) < 1 {
                return true;
            }
        }

        false
    }

    const fn is_point_in_quadrant2(&self, point: Vec2) -> bool {
        let z = self.pos.z.0;

        let center_x = (self.size.x.0 >> 1) + self.pos.x.0;
        let center_z = (self.size.z.0 >> 1) + self.pos.z.0;

        let far_x = self.pos.x.0 + self.size.x.0;

        let px = point.x.0;
        let pz = point.z.0;

        let x_diff1 = far_x - center_x;
        let x_diff2 = px - center_x;

        let z_diff1 = center_z - z;
        let z_diff2 = pz - z;

        let term1 = (x_diff2 * z_diff1) / x_diff1;

        if z_diff2 <= term1 {
            return false;
        }

        let term2 = z_diff2 - term1;
        let term3 = (x_diff1 * term2) / z_diff1;

        let term4 = term2 * term2 + term3 * term3;

        let term5 = (term3 * term2 * term2) / term4;
        let term6 = (term3 * term3 * term2) / term4;

        let term5_sign = term5 >> 0x1f;
        let term6_sign = term6 >> 0x1f;

        if (term5 ^ term5_sign) - term5_sign < 0x191 && (term6 ^ term6_sign) - term6_sign < 0x191 {
            let new_x = px + term5;
            let new_z = pz - term6;

            let z_div = z / 0x12;

            if -1 < (new_x / 0x12 - center_x / 0x12) * (center_z / 0x12 - z_div) - (new_z / 0x12 - z_div) * (far_x / 0x12 - center_x / 0x12) {
                return true;
            }
        }

        false
    }

    const fn is_point_in_quadrant3(&self, point: Vec2) -> bool {
        let x = self.pos.x.0;
        let z = self.pos.z.0;

        let center_x = (self.size.x.0 >> 1) + self.pos.x.0;
        let center_z = (self.size.z.0 >> 1) + self.pos.z.0;

        let px = point.x.0;
        let pz = point.z.0;

        let x_diff1 = center_x - x;
        let x_diff2 = px - x;

        let z_diff1 = z - center_z;
        let z_diff2 = pz - center_z;

        let term1 = (x_diff2 * z_diff1) / x_diff1;

        if z_diff2 <= term1 {
            return false;
        }

        let z_diff3 = center_z - z;

        let term2 = z_diff2 - term1;
        let term3 = (x_diff1 * term2) / z_diff3;

        let term4 = term2 * term2 + term3 * term3;

        let term5 = (term3 * term2 * term2) / term4;
        let term6 = (term3 * term3 * term2) / term4;

        let term5_sign = term5 >> 0x1f;
        let term6_sign = term6 >> 0x1f;

        if (term5 ^ term5_sign) - term5_sign < 0x191 && (term6 ^ term6_sign) - term6_sign < 0x191 {
            let new_x = px - term5;
            let new_z = pz - term6;

            let x_div = x / 0x12;

            if -1 < (new_x / 0x12 - x_div) * (z / 0x12 - center_z / 0x12) - (new_z / 0x12 - center_z / 0x12) * (center_x / 0x12 - x_div) {
                return true;
            }
        }

        false
    }
}

#[derive(Debug, Clone)]
pub struct EllipseCollider {
    pos: Vec2,
    size: Vec2,
}

impl EllipseCollider {
    pub const fn new(x: Fixed32, z: Fixed32, width: Fixed32, height: Fixed32) -> Self {
        Self { pos: Vec2 { x, z }, size: Vec2 { x: width, z: height } }
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.x, self.pos.z, self.size.x, self.size.z);

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

    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    pub fn set_pos<T: Into<Vec2>>(&mut self, pos: T) {
        self.pos = pos.into();
    }

    pub fn set_size<T: Into<Vec2>>(&mut self, size: T) {
        self.size = size.into();
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn contains_point<T: Into<Vec2>>(&self, point: T) -> bool {
        // FIXME: this logic makes it seem like this is truly a circle and not an ellipse? z radius is ignored?
        let radius = self.size.x >> 1;
        let rel_point = point.into() - self.pos - Vec2::new(radius, radius);
        rel_point.len() < radius
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TriangleType {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

impl TriangleType {
    pub const fn offsets(&self) -> [(f32, f32); 3] {
        match self {
            Self::BottomLeft => [(0.0, 1.0), (0.0, 0.0), (1.0, 1.0)],
            Self::BottomRight => [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
            Self::TopLeft => [(0.0, 1.0), (0.0, 0.0), (1.0, 0.0)],
            Self::TopRight => [(1.0, 1.0), (1.0, 0.0), (0.0, 0.0)],
        }
    }
}

#[derive(Debug)]
pub struct TriangleCollider {
    pos: Vec2,
    size: Vec2,
    type_: TriangleType,
}

impl TriangleCollider {
    pub const fn new(x: Fixed32, z: Fixed32, width: Fixed32, height: Fixed32, type_: TriangleType) -> Self {
        Self { pos: Vec2 { x, z }, size: Vec2 { x: width, z: height }, type_ }
    }

    pub const fn offsets(&self) -> [(f32, f32); 3] {
        self.type_.offsets()
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.x, self.pos.z, self.size.x, self.size.z);
        let offsets = self.offsets();

        let x1 = x + offsets[0].0 * width;
        let y1 = y + offsets[0].1 * height;
        let x2 = x + offsets[1].0 * width;
        let y2 = y + offsets[1].1 * height;
        let x3 = x + offsets[2].0 * width;
        let y3 = y + offsets[2].1 * height;

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

    const fn contains_point_top_left(&self, point: Vec2) -> bool {
        let x1 = self.pos.x.0;
        let z1 = self.pos.z.0;

        let x2 = self.pos.x.0 + self.size.x.0;
        let z2 = self.pos.z.0 + self.size.z.0;

        let px = point.x.0;
        let pz = point.z.0;

        let width = x2 - x1;
        let height = z2 - z1;

        let x_dist = px - x1;
        let z_dist = pz - z1;

        let scaled_dist = (height * x_dist) / width;
        if z_dist <= scaled_dist {
            return false;
        }

        let x1_div = x1 / 0x12;
        let z1_div = z1 / 0x12;
        let z2_div = z2 / 0x12;
        let x2_div = x2 / 0x12;
        let height_div = z2_div - z1_div;
        let width_div = x2_div - x1_div;

        if (((px / 0x12) * height_div - (pz / 0x12) * width_div) - z2_div * x1_div) + x2_div * z1_div < 0 {
            if x_dist < self.size.x.0 && z_dist < self.size.z.0 {
                return rect_contains_point(self.pos, self.size, point);
            }
        }

        true
    }

    const fn contains_point_top_right(&self, point: Vec2) -> bool {
        let x1 = self.pos.x.0;
        let z1 = self.pos.z.0;

        let x2 = self.pos.x.0 + self.size.x.0;
        let z2 = self.pos.z.0 + self.size.z.0;

        let px = point.x.0;
        let pz = point.z.0;

        let width = self.size.x.0;
        let height = self.size.z.0;

        let x_dist = px - x1;
        let z_dist = pz - z1 - height;

        let scaled_dist = (height * x_dist) / width;
        if scaled_dist <= -z_dist {
            return false;
        }

        let x1_div = x1 / 0x12;
        let z1_div = z1 / 0x12;
        let z2_div = z2 / 0x12;
        let x2_div = x2 / 0x12;

        let z1_minus_z2_div = z1_div - z2_div;
        let x2_minus_x1_div = x2_div - x1_div;

        if (((px / 0x12) * z1_minus_z2_div - (pz / 0x12) * x2_minus_x1_div) - z1_div * x1_div) + x2_div * z2_div < 0 {
            if x_dist < self.size.x.0 && (pz - z1) < self.size.z.0 {
                return rect_contains_point(self.pos, self.size, point);
            }
        }

        true
    }

    const fn contains_point_bottom_right(&self, point: Vec2) -> bool {
        let x1 = self.pos.x.0;
        let z1 = self.pos.z.0;

        let x2 = self.pos.x.0 + self.size.x.0;
        let z2 = self.pos.z.0 + self.size.z.0;

        let px = point.x.0;
        let pz = point.z.0;

        let width = x2 - x1;
        let height = z2 - z1;

        let x_dist = px - x1;
        let z_dist = pz - z1;

        let scaled_dist = (height * x_dist) / width;
        if scaled_dist <= z_dist {
            return false;
        }

        let x1_div = x1 / 0x12;
        let z1_div = z1 / 0x12;
        let z2_div = z2 / 0x12;
        let x2_div = x2 / 0x12;
        let height_div = z2_div - z1_div;
        let width_div = x2_div - x1_div;

        if (((px / 0x12) * height_div - (pz / 0x12) * width_div) - z2_div * x1_div) + x2_div * z1_div >= 1 {
            if x_dist < self.size.x.0 && z_dist < self.size.z.0 {
                return rect_contains_point(self.pos, self.size, point);
            }
        }

        true
    }

    const fn contains_point_bottom_left(&self, point: Vec2) -> bool {
        let x1 = self.pos.x.0;
        let z1 = self.pos.z.0;

        let x2 = self.pos.x.0 + self.size.x.0;
        let z2 = self.pos.z.0 + self.size.z.0;

        let px = point.x.0;
        let pz = point.z.0;

        let width = x2 - x1;
        let height = z1 - z2;

        let x_dist = px - x1;
        let z_dist = pz - z1;

        let scaled_dist = (height * x_dist) / width;
        if scaled_dist <= (pz - z2) {
            return false;
        }

        let x1_div = x1 / 0x12;
        let z2_div = z2 / 0x12;
        let x2_div = x2 / 0x12;
        let height_div = z1 / 0x12 - z2_div;
        let width_div = x2_div - x1_div;

        if (((px / 0x12) * height_div - (pz / 0x12) * width_div) - (z1 / 0x12) * x1_div) + x2_div * z2_div >= 1 {
            if x_dist < self.size.x.0 && z_dist < self.size.z.0 {
                return rect_contains_point(self.pos, self.size, point);
            }
        }

        true
    }

    pub fn contains_point<T: Into<Vec2>>(&self, point: T) -> bool {
        let point = point.into();
        
        match self.type_ {
            TriangleType::BottomLeft => self.contains_point_bottom_left(point),
            TriangleType::BottomRight => self.contains_point_bottom_right(point),
            TriangleType::TopLeft => self.contains_point_top_left(point),
            TriangleType::TopRight => self.contains_point_top_right(point),
        }
    }
}

#[derive(Debug)]
pub struct QuadCollider {
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    p4: Vec2,
}

impl QuadCollider {
    pub const fn new(x1: Fixed32, z1: Fixed32, x2: Fixed32, z2: Fixed32, x3: Fixed32, z3: Fixed32, x4: Fixed32, z4: Fixed32) -> Self {
        Self {
            p1: Vec2 { x: x1, z: z1 },
            p2: Vec2 { x: x2, z: z2 },
            p3: Vec2 { x: x3, z: z3 },
            p4: Vec2 { x: x4, z: z4 },
        }
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let x1 = self.p1.x * draw_params.scale - draw_params.origin.x;
        let y1 = -self.p1.z * draw_params.scale - draw_params.origin.y;
        let x2 = self.p2.x * draw_params.scale - draw_params.origin.x;
        let y2 = -self.p2.z * draw_params.scale - draw_params.origin.y;
        let x3 = self.p3.x * draw_params.scale - draw_params.origin.x;
        let y3 = -self.p3.z * draw_params.scale - draw_params.origin.y;
        let x4 = self.p4.x * draw_params.scale - draw_params.origin.x;
        let y4 = -self.p4.z * draw_params.scale - draw_params.origin.y;

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

    pub fn contains_point<T: Into<Vec2>>(&self, point: T) -> bool {
        let point = point.into();

        let px_minus_x1 = point.x - self.p1.x;
        let pz_minus_z1 = point.z - self.p1.z;

        let x2_minus_x1 = self.p2.x - self.p1.x;
        let z2_minus_z1 = self.p2.z - self.p1.z;

        let x4_minus_x1 = self.p4.x - self.p1.x;
        let z4_minus_z1 = self.p4.z - self.p1.z;

        if (x2_minus_x1.0 * pz_minus_z1.0) <= (z2_minus_z1.0 * px_minus_x1.0) && (z4_minus_z1.0 * px_minus_x1.0) <= (x4_minus_x1.0 * pz_minus_z1.0) {
            let px_minus_x3 = point.x - self.p3.x;
            let pz_minus_z3 = point.z - self.p3.z;

            let x2_minus_x3 = self.p2.x - self.p3.x;
            let z2_minus_z3 = self.p2.z - self.p3.z;

            let x4_minus_x3 = self.p4.x - self.p3.x;
            let z4_minus_z3 = self.p4.z - self.p3.z;

            if (z2_minus_z3.0 * px_minus_x3.0) <= (x2_minus_x3.0 * pz_minus_z3.0) && (x4_minus_x3.0 * pz_minus_z3.0) <= (z4_minus_z3.0 * px_minus_x3.0) {
                return true;
            }
        }

        false
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
                    format!("X1: {}", quad.p1.x),
                    format!("Z1: {}", quad.p1.z),
                    format!("X2: {}", quad.p2.x),
                    format!("Z2: {}", quad.p2.z),
                    format!("X3: {}", quad.p3.x),
                    format!("Z3: {}", quad.p3.z),
                    format!("X4: {}", quad.p4.x),
                    format!("Z4: {}", quad.p4.z),
                ]));
            }
            Self::Rect(RectCollider { pos, size, .. })
            | Self::Diamond(DiamondCollider { pos, size, .. })
            | Self::Ellipse(EllipseCollider { pos, size, .. })
            | Self::Triangle(TriangleCollider { pos, size, .. })
            => {
                groups.push((label, vec![
                    format!("X: {}", pos.x),
                    format!("Z: {}", pos.z),
                    format!("W: {}", size.x),
                    format!("H: {}", size.z),
                ]));
            }
        }
        
        // calculated geometry where it might be useful
        let label = String::from("Calculated");
        match self {
            Self::Ellipse(ellipse) => {
                let x_radius = ellipse.size.x >> 1;
                let z_radius = ellipse.size.z >> 1;
                let center_x = ellipse.pos.x + x_radius;
                let center_z = ellipse.pos.z + z_radius;
                
                groups.push((label, vec![
                    format!("CX: {}", center_x),
                    format!("CZ: {}", center_z),
                    format!("RX: {}", x_radius),
                    format!("RZ: {}", z_radius),
                ]));
            }
            Self::Triangle(tri) => {
                let offsets = tri.offsets();

                let x1 = tri.pos.x + if offsets[0].0 > 0.0 { tri.size.x } else { Fixed32(0) };
                let z1 = tri.pos.z + if offsets[0].1 > 0.0 { tri.size.z } else { Fixed32(0) };
                let x2 = tri.pos.x + if offsets[1].0 > 0.0 { tri.size.x } else { Fixed32(0) };
                let z2 = tri.pos.z + if offsets[1].1 > 0.0 { tri.size.z } else { Fixed32(0) };
                let x3 = tri.pos.x + if offsets[2].0 > 0.0 { tri.size.x } else { Fixed32(0) };
                let z3 = tri.pos.z + if offsets[2].1 > 0.0 { tri.size.z } else { Fixed32(0) };
                
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
                let radius_x = diamond.size.x >> 1;
                let radius_z = diamond.size.z >> 1;

                let x = diamond.pos.x;
                let z = diamond.pos.z;
                let width = diamond.size.x;
                let height = diamond.size.z;
                groups.push((label, vec![
                    format!("X1: {}", x + radius_x),
                    format!("Z1: {}", z),
                    format!("X2: {}", x + width),
                    format!("Z2: {}", z + radius_z),
                    format!("X3: {}", x + radius_x),
                    format!("Z3: {}", z + height),
                    format!("X4: {}", x),
                    format!("Z4: {}", z + radius_z),
                ]));
            }
            Self::Rect(rect) => {
                let nx = rect.pos.x;
                let nz = rect.pos.z;
                let fx = rect.pos.x + rect.size.x;
                let fz = rect.pos.z + rect.size.z;
                
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

    pub fn contains_point<T: Into<Vec2>>(&self, point: T) -> bool {
        match self {
            Self::Rect(rect) => rect.contains_point(point),
            Self::Ellipse(ellipse) => ellipse.contains_point(point),
            Self::Diamond(diamond) => diamond.contains_point(point),
            Self::Triangle(triangle) => triangle.contains_point(point),
            Self::Quad(quad) => quad.contains_point(point),
        }
    }
}