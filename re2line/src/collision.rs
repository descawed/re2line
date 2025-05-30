use crate::app::{DrawParams, Floor, GameObject, ObjectType};
use crate::math::{Fixed32, Vec2};
use crate::record::State;

const fn rect_contains_point(pos: Vec2, size: Vec2, point: Vec2) -> bool {
    point.x.0 >= pos.x.0 && point.x.0 < pos.x.0 + size.x.0 && point.z.0 >= pos.z.0 && point.z.0 < pos.z.0 + size.z.0
}

fn circle_contains_point(pos: Vec2, radius: Fixed32, point: Vec2) -> bool {
    let rel_point = point - pos - Vec2::new(radius, radius);
    rel_point.len() < radius
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CapsuleType {
    None,
    Horizontal,
    Vertical,
}

impl CapsuleType {
    pub const fn corner_radius(&self, width: f32, height: f32) -> epaint::CornerRadiusF32 {
        match self {
            Self::None => epaint::CornerRadiusF32::same(0.0),
            Self::Horizontal => epaint::CornerRadiusF32::same(width / 2.0),
            Self::Vertical => epaint::CornerRadiusF32::same(height / 2.0),
        }
    }
}

// these special types have additional 3D properties that we don't currently model, so we treat
// them as simple rects, but we do want to at least keep track of the fact that they aren't basic
// rects
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SpecialRectType {
    None,
    Ramp,
    HalfPipe,
    Floor,
}

#[derive(Debug, Clone)]
pub struct RectCollider {
    pos: Vec2,
    size: Vec2,
    capsule_type: CapsuleType,
    special_rect_type: SpecialRectType,
    floor: Floor,
    collision_mask: u16,
}

impl RectCollider {
    pub const fn new(x: Fixed32, z: Fixed32, width: Fixed32, height: Fixed32, floor: Floor, capsule_type: CapsuleType) -> Self {
        Self {
            pos: Vec2 { x, z },
            size: Vec2 { x: width, z: height },
            floor,
            capsule_type,
            special_rect_type: SpecialRectType::None,
            collision_mask: 0xFFFF,
        }
    }
    
    pub const fn collision_mask(&self) -> u16 {
        self.collision_mask
    }

    pub fn with_collision_mask(mut self, collision_mask: u16) -> Self {
        self.collision_mask = collision_mask;
        self
    }
    
    pub const fn with_special_rect_type(mut self, special_rect_type: SpecialRectType) -> Self {
        self.special_rect_type = special_rect_type;
        self
    }
    
    pub const fn set_floor(&mut self, floor: Floor) {
        self.floor = floor;
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.x, self.pos.z, self.size.x, self.size.z);
        let corner_radius = self.capsule_type.corner_radius(width, height);

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
        let point = point.into();

        match self.capsule_type {
            CapsuleType::Horizontal => {
                let z_radius = self.size.z >> 1;
                let side = (((point.x - (self.pos.x - z_radius + self.size.x)).0 as u32 & 0xbfffffff)
                    | ((point.x - (self.pos.x + z_radius)) >> 1).0 as u32) >> 0x1e;
                match side {
                    0 => {
                        let pos = Vec2::new((self.pos.x - self.size.z) + self.size.x, self.pos.z);
                        return circle_contains_point(pos, z_radius, point);
                    }
                    3 => return circle_contains_point(self.pos, z_radius, point),
                    _ => (),
                }
            }
            CapsuleType::Vertical => {
                let x_radius = self.size.x >> 1;
                let side = (((point.z - (self.pos.z - x_radius + self.size.z)).0 as u32 & 0xbfffffff)
                    | ((point.z - (self.pos.z + x_radius)) >> 1).0 as u32) >> 0x1e;
                match side {
                    0 => {
                        let pos = Vec2::new(self.pos.x, self.pos.z + (self.size.z - self.size.x));
                        return circle_contains_point(pos, x_radius, point);
                    }
                    3 => return circle_contains_point(self.pos, x_radius, point),
                    _ => (),
                }
            }
            _ => (),
        }

        rect_contains_point(self.pos, self.size, point)
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
    floor: Floor,
    collision_mask: u16,
}

impl DiamondCollider {
    pub const fn new(x: Fixed32, z: Fixed32, width: Fixed32, height: Fixed32, floor: Floor) -> Self {
        Self {
            pos: Vec2 { x, z },
            size: Vec2 { x: width, z: height },
            floor,
            collision_mask: 0xFFFF,
        }
    }
    
    pub fn with_collision_mask(mut self, collision_mask: u16) -> Self {
        self.collision_mask = collision_mask;
        self
    }

    pub const fn collision_mask(&self) -> u16 {
        self.collision_mask
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

        let quadrant = (((point.z - center_z) >> 0x1e).0 & 2) | (((point.x - center_x) >> 0x1f).0 & 1);
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

        term1 > z_diff2
        /*if term1 <= z_diff2 {
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

        false*/
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

        term1 > z_diff2
        /*if term1 <= z_diff2 {
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

        false*/
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
        
        z_diff2 > term1
        /*if z_diff2 <= term1 {
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

        false*/
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

        z_diff2 > term1
        /*if z_diff2 <= term1 {
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

        false*/
    }
}

#[derive(Debug, Clone)]
pub struct EllipseCollider {
    pos: Vec2,
    size: Vec2,
    floor: Floor,
    collision_mask: u16,
}

impl EllipseCollider {
    pub const fn new(x: Fixed32, z: Fixed32, width: Fixed32, height: Fixed32, floor: Floor) -> Self {
        Self {
            pos: Vec2 { x, z },
            size: Vec2 { x: width, z: height },
            floor,
            collision_mask: 0xFFFF,
        }
    }

    pub const fn collision_mask(&self) -> u16 {
        self.collision_mask
    }

    pub fn with_collision_mask(mut self, collision_mask: u16) -> Self {
        self.collision_mask = collision_mask;
        self
    }
    
    pub const fn set_floor(&mut self, floor: Floor) {
        self.floor = floor;
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
        circle_contains_point(self.pos, self.size.x >> 1, point.into())
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
    floor: Floor,
    type_: TriangleType,
    collision_mask: u16,
}

impl TriangleCollider {
    pub const fn new(x: Fixed32, z: Fixed32, width: Fixed32, height: Fixed32, floor: Floor, type_: TriangleType) -> Self {
        Self {
            pos: Vec2 { x, z },
            size: Vec2 { x: width, z: height },
            floor,
            type_,
            collision_mask: 0xFFFF,
        }
    }

    pub const fn collision_mask(&self) -> u16 {
        self.collision_mask
    }

    pub fn with_collision_mask(mut self, collision_mask: u16) -> Self {
        self.collision_mask = collision_mask;
        self
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

        false
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

        false
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

        false
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

        false
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
    floor: Floor,
}

impl QuadCollider {
    pub const fn new(x1: Fixed32, z1: Fixed32, x2: Fixed32, z2: Fixed32, x3: Fixed32, z3: Fixed32, x4: Fixed32, z4: Fixed32, floor: Floor) -> Self {
        Self {
            p1: Vec2 { x: x1, z: z1 },
            p2: Vec2 { x: x2, z: z2 },
            p3: Vec2 { x: x3, z: z3 },
            p4: Vec2 { x: x4, z: z4 },
            floor,
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
    fn type_string(&self) -> String {
        String::from(match self {
            Self::Rect(rect) => {
                match rect.capsule_type {
                    CapsuleType::None => match rect.special_rect_type {
                        SpecialRectType::None => "Rectangle",
                        SpecialRectType::Ramp => "Ramp",
                        SpecialRectType::HalfPipe => "Half pipe",
                        SpecialRectType::Floor => "Floor",
                    },
                    CapsuleType::Horizontal => "Capsule (horizontal)",
                    CapsuleType::Vertical => "Capsule (vertical)",
                }
            }
            Self::Diamond(_) => "Diamond",
            Self::Ellipse(_) => "Ellipse",
            Self::Triangle(_) => "Triangle",
            Self::Quad(_) => "Quadrilateral",
        })
    }
}

impl GameObject for Collider {
    fn object_type(&self) -> ObjectType {
        if let Self::Rect(rect) = self {
            if rect.special_rect_type == SpecialRectType::Floor {
                return ObjectType::Floor;
            }
        }
        
        ObjectType::Collider
    }

    fn contains_point(&self, point: Vec2) -> bool {
        match self {
            Self::Rect(rect) => rect.contains_point(point),
            Self::Ellipse(ellipse) => ellipse.contains_point(point),
            Self::Diamond(diamond) => diamond.contains_point(point),
            Self::Triangle(triangle) => triangle.contains_point(point),
            Self::Quad(quad) => quad.contains_point(point),
        }
    }

    fn name(&self) -> String {
        self.type_string()
    }

    fn description(&self) -> String {
        match self {
            Self::Quad(quad) => {
                format!("X1: {: >6} | Z1: {: >6}\nX2: {: >6} | Z2: {: >6}\nX3: {: >6} | Z3: {: >6}\nX4: {: >6} | Z4: {: >6}\n", quad.p1.x, quad.p1.z, quad.p2.x, quad.p2.z, quad.p3.x, quad.p3.z, quad.p4.x, quad.p4.z)
            }
            Self::Rect(RectCollider { pos, size, .. })
            | Self::Diamond(DiamondCollider { pos, size, .. })
            | Self::Ellipse(EllipseCollider { pos, size, .. })
            | Self::Triangle(TriangleCollider { pos, size, .. })
            => {
                format!("X: {: >6} | Z: {: >6}\nW: {: >6} | H: {: >6}", pos.x, pos.z, size.x, size.z)
            }
        }
    }

    fn details(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = Vec::new();
        
        // type
        groups.push((String::from("Type"), vec![self.type_string()]));

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
                    format!("Floor: {}", quad.floor),
                ]));
            }
            Self::Rect(RectCollider { pos, size, floor, .. })
            | Self::Diamond(DiamondCollider { pos, size, floor, .. })
            | Self::Ellipse(EllipseCollider { pos, size, floor, .. })
            | Self::Triangle(TriangleCollider { pos, size, floor, .. })
            => {
                let mut params = vec![
                    format!("X: {}", pos.x),
                    format!("Z: {}", pos.z),
                    format!("W: {}", size.x),
                    format!("H: {}", size.z),
                    format!("Floor: {}", floor),
                ];
                if self.collision_mask() != 0xFFFF {
                    params.push(format!("Collision: {:04X}", self.collision_mask()));
                }
                
                groups.push((label, params));
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

    fn floor(&self) -> Floor {
        match self {
            Self::Rect(rect) => rect.floor,
            Self::Diamond(diamond) => diamond.floor,
            Self::Ellipse(ellipse) => ellipse.floor,
            Self::Triangle(triangle) => triangle.floor,
            Self::Quad(quad) => quad.floor,
        }
    }

    fn collision_mask(&self) -> u16 {
        match self {
            Self::Rect(rect) => rect.collision_mask(),
            Self::Diamond(diamond) => diamond.collision_mask(),
            Self::Ellipse(ellipse) => ellipse.collision_mask(),
            Self::Triangle(triangle) => triangle.collision_mask(),
            Self::Quad(_) => 0xFFFF,
        }
    }

    fn gui_shape(&self, draw_params: &DrawParams, _state: &State) -> egui::Shape {
        match self {
            Self::Rect(rect) => rect.gui_shape(draw_params),
            Self::Diamond(diamond) => diamond.gui_shape(draw_params),
            Self::Ellipse(ellipse) => ellipse.gui_shape(draw_params),
            Self::Triangle(triangle) => triangle.gui_shape(draw_params),
            Self::Quad(quad) => quad.gui_shape(draw_params),
        }
    }
}