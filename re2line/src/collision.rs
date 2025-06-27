use residat::common::{Fixed32, Vec2};

use crate::app::{DrawParams, Floor, GameObject, ObjectType, WorldPos};
use crate::record::State;

#[derive(Debug, Clone)]
pub struct Motion {
    pub origin: WorldPos,
    pub to: Vec2,
    pub offset: Vec2,
}

impl Motion {
    pub const fn new(origin: WorldPos, to: Vec2, offset: Vec2) -> Self {
        Self {
            origin,
            to,
            offset,
        }
    }

    pub const fn point(point: Vec2, floor: Floor) -> Self {
        Self {
            origin: WorldPos::point(point, floor),
            to: point,
            offset: Vec2::zero(),
        }
    }

    pub const fn point_with_motion(point: Vec2, floor: Floor) -> Self {
        Self {
            origin: WorldPos::point(Vec2 { x: point.x.dec(), z: point.z }, floor),
            to: point,
            offset: Vec2::zero(),
        }
    }

    pub const fn from(&self) -> Vec2 {
        self.origin.pos
    }

    pub const fn size(&self) -> Vec2 {
        self.origin.size
    }

    pub fn angle(&self) -> Fixed32 {
        self.from().angle_between(&self.to)
    }

    pub fn size_in_direction_of(&self, pos: Vec2, size: Vec2) -> Fixed32 {
        let radius = size >> 1;
        let our_size = self.size();
        let offset_to = self.to + our_size;
        let angle = ((radius.z + pos.z) - offset_to.z).atan2((radius.x - offset_to.x) + pos.x);
        let rel_angle = angle - self.angle();

        let mut norm_angle = rel_angle & Fixed32(0xfff);
        if rel_angle & 0xc00 == 0xc00 {
            norm_angle = Fixed32(0x1000) - norm_angle;
        } else if rel_angle & 0x800 == 0x800 {
            norm_angle -= Fixed32(0x800);
        } else if norm_angle & 0x400 == 0x400 {
            norm_angle = Fixed32(0x800) - norm_angle;
        }

        if our_size.z < our_size.x {
            norm_angle.cos() * (our_size.x - our_size.z) + our_size.z
        } else {
            norm_angle.sin() * (our_size.z - our_size.x) + our_size.x
        }
    }

    pub fn is_destination_in_collision_bounds(&self, pos: &WorldPos) -> bool {
        if !self.origin.can_collide_with(&pos) {
            return false;
        }

        // it's accurate to the game that we use this same size for both axes
        let motion_size = self.size().x << 1;
        let size = pos.size;
        let x_size = (size.x + motion_size).0 as u32;
        let z_size = (size.z + motion_size).0 as u32;

        let rel = (self.to + self.size()) - pos.pos;
        let wrapped_x = rel.x.0 as u32;
        let wrapped_z = rel.z.0 as u32;

        wrapped_x < x_size && wrapped_z < z_size
    }
}

const RECT_THRESHOLD: Fixed32 = Fixed32(0x191);

fn push_to_rect_nearest_edge(motion: &Motion, x_edge_offset: Fixed32, z_edge_offset: Fixed32) -> Vec2 {
    let rel = motion.to - motion.from();
    let x_edge_abs = x_edge_offset.abs();
    let z_edge_abs = z_edge_offset.abs();

    let quadrant = (((rel.x ^ x_edge_offset) >> 1).0 | ((rel.z ^ z_edge_offset) & 0xbfffffffu32 as i32)) >> 0x1e;

    if quadrant == 1 {
        if x_edge_abs < RECT_THRESHOLD {
            return motion.to + Vec2::new(x_edge_offset, Fixed32(0));
        }
    } else if quadrant == 2 {
        if z_edge_abs < RECT_THRESHOLD {
            return motion.to + Vec2::new(Fixed32(0), z_edge_offset);
        }
    } else if quadrant != 3 {
        return if x_edge_abs < z_edge_abs {
            motion.to + Vec2::new(x_edge_offset, Fixed32(0))
        } else {
            motion.to + Vec2::new(Fixed32(0), z_edge_offset)
        };
    }

    if x_edge_abs < z_edge_abs {
        if x_edge_abs < (RECT_THRESHOLD << 1) {
            return motion.to + Vec2::new(x_edge_offset, Fixed32(0));
        }
    } else if z_edge_abs < (RECT_THRESHOLD << 1) {
        return motion.to + Vec2::new(Fixed32(0), z_edge_offset);
    }

    motion.from()
}

fn push_out_of_rect(pos: Vec2, size: Vec2, motion: &Motion) -> Vec2 {
    let directional_size = motion.size_in_direction_of(pos, size);

    let mut max_x_outside = (size.x - motion.to.x) + pos.x.inc() + motion.size().x;
    let min_x_outside = (pos.x - motion.to.x - motion.size().x).dec();
    if max_x_outside > -min_x_outside {
        max_x_outside = min_x_outside;
    }

    let min_z_outside = (pos.z - motion.to.z - directional_size).dec();
    let mut max_z_outside = (size.z - motion.to.z) + pos.z.inc() + directional_size;
    if max_z_outside > -min_z_outside {
        max_z_outside = min_z_outside;
    }

    push_to_rect_nearest_edge(motion, max_x_outside, max_z_outside)
}

fn rect_clip_motion(pos: &WorldPos, motion: &Motion) -> Vec2 {
    if !motion.is_destination_in_collision_bounds(pos) {
        return motion.to;
    }

    let size = pos.size;
    let pos = pos.pos;

    let rel = (motion.size() - pos) + motion.from();
    let total_size = size + (motion.size() << 1);
    let mut outside_flags = if total_size.x <= rel.x {
        2u32
    } else {
        0u32
    } | if total_size.z <= rel.z {
        1u32
    } else {
        0u32
    };

    if rel.x == Fixed32(-1) || rel.x == total_size.x.inc() {
        outside_flags = 2;
    }

    if rel.z == Fixed32(-1) || rel.z == total_size.z.inc() {
        outside_flags = 1;
    } else if outside_flags == 0 {
        return push_out_of_rect(pos, size, motion);
    }

    let mut clipped = motion.to;
    if outside_flags & 2 != 0 {
        let xr = size.x >> 1;
        let mut adjustment = xr.inc() + motion.size().x;
        if !(motion.to.x - motion.from().x).is_negative() {
            adjustment = -adjustment;
        }
        clipped.x = adjustment + xr + pos.x;
    }

    if outside_flags & 1 != 0 {
        let zr = size.z >> 1;
        let mut adjustment = zr.inc() + motion.size().z;
        if !(motion.to.z - motion.from().z).is_negative() {
            adjustment = -adjustment;
        }
        clipped.z = adjustment + zr + pos.z;
    }

    clipped
}

fn rect_contains_point(pos: &WorldPos, point: Vec2) -> bool {
    rect_clip_motion(pos, &Motion::point(point, Floor::ANY)) != point
}

fn circle_clip_motion(pos: &WorldPos, motion: &Motion) -> Vec2 {
    if !motion.is_destination_in_collision_bounds(pos) {
        return motion.to;
    }

    let size = pos.size;
    let pos = pos.pos;

    let radius = size.x >> 1;
    let rel = (motion.to - pos) - Vec2::new(radius, radius);
    let distance_to_center = rel.len();
    let distance_to_edge = (radius - distance_to_center) + motion.size_in_direction_of(pos, size);
    if !distance_to_edge.is_positive() {
        return motion.to;
    }

    let distance_to_center = distance_to_center.0;
    let distance_to_edge = distance_to_edge.0;
    let x_offset = ((rel.x.0 + 8) * distance_to_edge) / distance_to_center;
    let z_offset = ((rel.z.0 + 8) * distance_to_edge) / distance_to_center;

    motion.to + Vec2::new(x_offset, z_offset)
}

fn circle_contains_point(pos: &WorldPos, point: Vec2) -> bool {
    circle_clip_motion(pos, &Motion::point(point, Floor::ANY)) != point
}

const fn tri_adjustments(a: Fixed32, b: Fixed32) -> (Fixed32, Fixed32) {
    let denom = a.0 * a.0 + b.0 * b.0;
    let x = Fixed32(a.0.overflowing_mul(b.0).0.overflowing_mul(b.0).0 / denom);
    let z = Fixed32(a.0.overflowing_mul(a.0).0.overflowing_mul(b.0).0 / denom);
    (x, z)
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
    pos: WorldPos,
    capsule_type: CapsuleType,
    special_rect_type: SpecialRectType,
}

impl RectCollider {
    pub const fn new(pos: WorldPos, capsule_type: CapsuleType) -> Self {
        Self {
            pos,
            capsule_type,
            special_rect_type: SpecialRectType::None,
        }
    }
    
    pub const fn collision_mask(&self) -> u16 {
        self.pos.collision_mask
    }
    
    pub const fn with_special_rect_type(mut self, special_rect_type: SpecialRectType) -> Self {
        self.special_rect_type = special_rect_type;
        self
    }
    
    pub const fn set_floor(&mut self, floor: Floor) {
        self.pos.floor = floor;
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.pos.x, self.pos.pos.z, self.pos.size.x, self.pos.size.z);
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
        if matches!(self.special_rect_type, SpecialRectType::Ramp | SpecialRectType::Floor) {
            // ramps and floors don't inhibit motion, so a clip test won't tell us if the point is in the rect
            return rect_contains_point(&self.pos, point);
        }

        self.clip_motion(&Motion::point(point, Floor::ANY)) != point
    }

    pub fn clip_motion(&self, motion: &Motion) -> Vec2 {
        // FIXME: add correct handling for half pipes
        if matches!(self.special_rect_type, SpecialRectType::Ramp | SpecialRectType::Floor) {
            return motion.to; // ramps and floors don't inhibit motion
        }

        if !motion.is_destination_in_collision_bounds(&self.pos) {
            return motion.to;
        }

        let pos = self.pos.pos;
        let size = self.pos.size;

        match self.capsule_type {
            CapsuleType::Horizontal => {
                let z_radius = size.z >> 1;
                let side = (((motion.to.x - (pos.x - z_radius + size.x)).0 as u32 & 0xbfffffff)
                    | ((motion.to.x - (pos.x + z_radius)) >> 1).0 as u32) >> 0x1e;
                match side {
                    0 => {
                        let pos = WorldPos::rect(Vec2::new((pos.x - size.z) + size.x, pos.z), Vec2::new(size.z, size.z), self.pos.floor);
                        return circle_clip_motion(&pos, motion);
                    }
                    3 => {
                        let pos = WorldPos::rect(pos, Vec2::new(size.z, size.z), self.pos.floor);
                        return circle_clip_motion(&pos, motion);
                    }
                    _ => (),
                }
            }
            CapsuleType::Vertical => {
                let x_radius = size.x >> 1;
                let side = (((motion.to.z - (pos.z - x_radius + size.z)).0 as u32 & 0xbfffffff)
                    | ((motion.to.z - (pos.z + x_radius)) >> 1).0 as u32) >> 0x1e;
                match side {
                    0 => {
                        let pos = WorldPos::rect(Vec2::new(pos.x, pos.z + (size.z - size.x)), Vec2::new(size.x, size.x), self.pos.floor);
                        return circle_clip_motion(&pos, motion);
                    }
                    3 => {
                        let pos = WorldPos::rect(pos, Vec2::new(size.z, size.z), self.pos.floor);
                        return circle_clip_motion(&pos, motion);
                    }
                    _ => (),
                }
            }
            _ => (),
        }

        rect_clip_motion(&self.pos, motion)
    }

    pub fn set_pos<T: Into<Vec2>>(&mut self, pos: T) {
        self.pos.pos = pos.into();
    }

    pub fn set_size<T: Into<Vec2>>(&mut self, size: T) {
        self.pos.size = size.into();
    }
}

#[derive(Debug)]
pub struct DiamondCollider {
    pos: WorldPos,
}

impl DiamondCollider {
    pub const fn new(pos: WorldPos) -> Self {
        Self {
            pos,
        }
    }

    pub const fn collision_mask(&self) -> u16 {
        self.pos.collision_mask
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.pos.x, self.pos.pos.z, self.pos.size.x, self.pos.size.z);
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

        // unlike some other collision types, when we clip motion, we can just force the character
        // back to the original position. so, to determine whether the motion was clipped, we need
        // to ensure that the from and to positions are different.
        self.clip_motion(&Motion::point_with_motion(point, Floor::ANY)) != point
    }

    pub fn clip_motion(&self, motion: &Motion) -> Vec2 {
        if !motion.is_destination_in_collision_bounds(&self.pos) {
            return motion.to;
        }

        let center_x = (self.pos.size.x >> 1) + self.pos.pos.x;
        let center_z = (self.pos.size.z >> 1) + self.pos.pos.z;

        let quadrant = (((motion.to.z - center_z) >> 0x1e).0 & 2) | (((motion.to.x - center_x) >> 0x1f).0 & 1);
        match quadrant {
            0 => self.clip_motion_in_quadrant0(motion),
            1 => self.clip_motion_in_quadrant1(motion),
            2 => self.clip_motion_in_quadrant2(motion),
            3 => self.clip_motion_in_quadrant3(motion),
            _ => unreachable!(),
        }
    }

    fn clip_motion_in_quadrant0(&self, motion: &Motion) -> Vec2 {
        let pos = self.pos.pos;
        let size = self.pos.size;

        let directional_size = motion.size_in_direction_of(pos, size);

        let center = (size >> 1) + pos;
        let far = pos + size;

        let x_diff1 = far.x - center.x + directional_size;
        let x_diff2 = (motion.offset.x - center.x) + motion.to.x;

        let z_diff1 = center.z - far.z - directional_size;
        let z_diff2 = (motion.offset.z - far.z) + motion.to.z;

        let term1 = Fixed32((x_diff2.0 * z_diff1.0) / x_diff1.0);

        if term1 <= z_diff2 - directional_size {
            return motion.to;
        }

        let z_diff3 = (far.z - center.z) + directional_size;

        let term2 = z_diff2 - term1 - directional_size;
        let term3 = Fixed32((x_diff1.0 * term2.0) / z_diff3.0);

        let (x_adjustment, z_adjustment) = tri_adjustments(term3, term2);
        if x_adjustment.abs() < RECT_THRESHOLD && z_adjustment.abs() < RECT_THRESHOLD {
            Vec2::new(motion.to.x - x_adjustment, motion.to.z - z_adjustment)
            /*let new_x = (motion.to.x - x_adjustment + motion.offset.x).0;
            let new_z = (motion.to.z - z_adjustment + motion.offset.z).0;

            if (new_x / 0x12 - center.x.0 / 0x12) * (center.z.0 / 0x12 - far.z.0 / 0x12) - (new_z / 0x12 - far.z.0 / 0x12) * (far.x.0 / 0x12 - center.x.0 / 0x12) < 1 {
                return motion.to;
            }*/
        } else {
            motion.from()
        }
    }

    fn clip_motion_in_quadrant1(&self, motion: &Motion) -> Vec2 {
        let pos = self.pos.pos;
        let size = self.pos.size;

        let directional_size = motion.size_in_direction_of(pos, size);

        let center = (size >> 1) + pos;
        let far = pos + size;

        let x_diff1 = center.x - pos.x + directional_size;
        let x_diff2 = (directional_size - pos.x) + motion.to.x + directional_size;

        let z_diff1 = far.z - center.z + directional_size;
        let z_diff2 = motion.to.z + (motion.offset.z - center.z);

        let term1 = Fixed32((x_diff2.0 * z_diff1.0) / x_diff1.0);

        if term1 <= z_diff2 {
            return motion.to;
        }

        let term2 = z_diff2 - term1;
        let term3 = Fixed32((x_diff1.0 * term2.0) / z_diff1.0);

        let (x_adjustment, z_adjustment) = tri_adjustments(term3, term2);
        if x_adjustment.abs() < RECT_THRESHOLD && z_adjustment.abs() < RECT_THRESHOLD {
            Vec2::new(motion.to.x + x_adjustment, motion.to.z - z_adjustment)
            /*let new_x = (motion.to.x + x_adjustment + motion.offset.x).0;
            let new_z = (motion.to.z - z_adjustment + motion.offset.z).0;

            let x_div = self.pos.x.0 / 0x12;

            if (new_x / 0x12 - x_div) * (far.z.0 / 0x12 - center.z.0 / 0x12) - (new_z / 0x12 - center.z.0 / 0x12) * (center.x.0 / 0x12 - x_div) < 1 {
                return motion.to;
            }*/
        } else {
            motion.from()
        }
    }

    fn clip_motion_in_quadrant2(&self, motion: &Motion) -> Vec2 {
        let pos = self.pos.pos;
        let size = self.pos.size;

        let directional_size = motion.size_in_direction_of(pos, size);

        let center = (size >> 1) + pos;
        let far = pos + size;

        let x_diff1 = far.x - center.x + directional_size;
        let x_diff2 = (motion.offset.x - center.x) + motion.to.x;

        let z_diff1 = center.z - pos.z + directional_size;
        let z_diff2 = (motion.offset.z - pos.z) + motion.to.z;

        let term1 = Fixed32((x_diff2.0 * z_diff1.0) / x_diff1.0);
        
        if z_diff2 + directional_size <= term1 {
            return motion.to;
        }

        let term2 = z_diff2 - term1 + directional_size;
        let term3 = Fixed32((x_diff1.0 * term2.0) / z_diff1.0);

        let (x_adjustment, z_adjustment) = tri_adjustments(term3, term2);
        if x_adjustment.abs() < RECT_THRESHOLD && z_adjustment.abs() < RECT_THRESHOLD {
            Vec2::new(motion.to.x + x_adjustment, motion.to.z - z_adjustment)
            /*let mut clipped = motion.to;
            clipped.x = motion.to.x + x_adjustment;
            clipped.z = motion.to.z - z_adjustment;
            clipped

            let z_div = self.pos.z.0 / 0x12;

            if -1 < ((clipped.x + motion.offset.x).0 / 0x12 - center.x.0 / 0x12) * (center.z.0 / 0x12 - z_div) - ((clipped.z + motion.offset.z).0 / 0x12 - z_div) * (far.x.0 / 0x12 - center.x.0 / 0x12) {
                return motion.to;
            }*/
        } else {
            motion.from()
        }
    }

    fn clip_motion_in_quadrant3(&self, motion: &Motion) -> Vec2 {
        let pos = self.pos.pos;
        let size = self.pos.size;

        let directional_size = motion.size_in_direction_of(pos, size);

        let center = (size >> 1) + pos;

        let x_diff1 = center.x - pos.x + directional_size;
        let x_diff2 = (motion.offset.x - pos.x) + motion.to.x + directional_size;

        let z_diff1 = pos.z - center.z - directional_size;
        let z_diff2 = motion.to.z + (motion.offset.z - center.z);

        let term1 = Fixed32((x_diff2.0 * z_diff1.0) / x_diff1.0);

        if z_diff2 <= term1 {
            return motion.to;
        }

        let z_diff3 = center.z - pos.z + directional_size;

        let term2 = z_diff2 - term1;
        let term3 = Fixed32((x_diff1.0 * term2.0) / z_diff3.0);

        let (x_adjustment, z_adjustment) = tri_adjustments(term3, term2);
        if x_adjustment.abs() < RECT_THRESHOLD && z_adjustment.abs() < RECT_THRESHOLD {
            Vec2::new(motion.to.x - x_adjustment, motion.to.z - z_adjustment)
            /*let new_x = (motion.to.x - x_adjustment + motion.offset.x).0;
            let new_z = (motion.to.z - z_adjustment + motion.offset.z).0;

            let x_div = self.pos.x.0 / 0x12;

            if -1 < (new_x / 0x12 - x_div) * (self.pos.z.0 / 0x12 - center.z.0 / 0x12) - (new_z / 0x12 - center.z.0 / 0x12) * (center.x.0 / 0x12 - x_div) {
                return motion.to;
            }*/
        } else {
            motion.from()
        }
    }
}

#[derive(Debug, Clone)]
pub struct EllipseCollider {
    pos: WorldPos,
}

impl EllipseCollider {
    pub const fn new(pos: WorldPos) -> Self {
        Self {
            pos,
        }
    }

    pub const fn collision_mask(&self) -> u16 {
        self.pos.collision_mask
    }
    
    pub const fn set_floor(&mut self, floor: Floor) {
        self.pos.floor = floor;
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.pos.x, self.pos.pos.z, self.pos.size.x, self.pos.size.z);

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
        self.pos.pos
    }

    pub fn set_pos<T: Into<Vec2>>(&mut self, pos: T) {
        self.pos.pos = pos.into();
    }

    pub fn set_size<T: Into<Vec2>>(&mut self, size: T) {
        self.pos.size = size.into();
    }

    pub fn size(&self) -> Vec2 {
        self.pos.size
    }

    pub fn contains_point<T: Into<Vec2>>(&self, point: T) -> bool {
        // FIXME: this logic makes it seem like this is truly a circle and not an ellipse? z radius is ignored?
        //  however, it IS used for the bounding rect test before we get into the actual circle logic. so the
        //  proper shape would be a circle clipped to the bounding rect, which we don't have an easy way to
        //  draw.
        circle_contains_point(&self.pos, point.into())
    }

    pub fn clip_motion(&self, motion: &Motion) -> Vec2 {
        circle_clip_motion(&self.pos, motion)
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
    pos: WorldPos,
    type_: TriangleType,
}

impl TriangleCollider {
    pub const fn new(pos: WorldPos, type_: TriangleType) -> Self {
        Self {
            pos,
            type_,
        }
    }

    pub const fn collision_mask(&self) -> u16 {
        self.pos.collision_mask
    }

    pub const fn offsets(&self) -> [(f32, f32); 3] {
        self.type_.offsets()
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        let (x, y, width, height) = draw_params.transform(self.pos.pos.x, self.pos.pos.z, self.pos.size.x, self.pos.size.z);
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

    fn clip_motion_top_left(&self, motion: &Motion) -> Vec2 {
        let pos = self.pos.pos;
        let size = self.pos.size;

        let directional_size = motion.size_in_direction_of(pos, size);

        let dist = motion.to - pos;
        let far = pos + size;

        let width = size.x + directional_size;
        let height = size.z + directional_size;

        let scaled_dist = Fixed32((height.0 * dist.x.0) / width.0);
        if (dist.z + directional_size) <= scaled_dist {
            return motion.to;
        }

        let x1_div = pos.x.0 / 0x12;
        let z1_div = pos.z.0 / 0x12;
        let z2_div = far.z.0 / 0x12;
        let x2_div = far.x.0 / 0x12;
        let height_div = z2_div - z1_div;
        let width_div = x2_div - x1_div;

        if (((motion.from().x.0 / 0x12) * height_div - (motion.from().z.0 / 0x12) * width_div) - z2_div * x1_div) + x2_div * z1_div < 0 {
            if (dist.x + directional_size) < (size.x + directional_size) && dist.z < (size.z + directional_size) {
                return rect_clip_motion(&self.pos, motion);
            }
        } else {
            let term1 = (dist.z - scaled_dist) + directional_size;
            let term2 = Fixed32((width.0 * term1.0) / height.0);
            let (x_adjustment, z_adjustment) = tri_adjustments(term1, term2);
            if x_adjustment.abs() < RECT_THRESHOLD && z_adjustment.abs() < RECT_THRESHOLD {
                return Vec2::new(motion.to.x - x_adjustment, motion.to.z - z_adjustment);
            }
        }

        motion.to
    }

    fn clip_motion_top_right(&self, motion: &Motion) -> Vec2 {
        let pos = self.pos.pos;
        let size = self.pos.size;

        let directional_size = motion.size_in_direction_of(pos, size);

        let dist = motion.to - pos;
        let far = pos + size;

        let z_dist = dist.z - size.z;

        let scaled_dist = Fixed32(((size.z + (directional_size << 1)).0 * (dist.x + directional_size).0) / (size.x + (directional_size << 1)).0);
        if z_dist <= -scaled_dist {
            return motion.to;
        }

        let x1_div = pos.x.0 / 0x12;
        let z1_div = pos.z.0 / 0x12;
        let z2_div = far.z.0 / 0x12;
        let x2_div = far.x.0 / 0x12;

        let z1_minus_z2_div = z1_div - z2_div;
        let x2_minus_x1_div = x2_div - x1_div;

        if (((motion.from().x.0 / 0x12) * z1_minus_z2_div - (motion.from().z.0 / 0x12) * x2_minus_x1_div) - z1_div * x1_div) + x2_div * z2_div < 0 {
            if dist.x < (size.x + directional_size) && dist.z < (size.z + directional_size) {
                return rect_clip_motion(&self.pos, motion);
            }
        } else {
            let term1 = z_dist + scaled_dist;
            let term2 = Fixed32(((size.x + directional_size).0 * term1.0) / (size.z + directional_size).0);
            let (x_adjustment, z_adjustment) = tri_adjustments(term1, term2);
            if x_adjustment.abs() < RECT_THRESHOLD && z_adjustment.abs() < RECT_THRESHOLD {
                return Vec2::new(motion.to.x - x_adjustment, motion.to.z - z_adjustment);
            }
        }

        motion.to
    }

    fn clip_motion_bottom_right(&self, motion: &Motion) -> Vec2 {
        let pos = self.pos.pos;
        let size = self.pos.size;

        let directional_size = motion.size_in_direction_of(pos, size);

        let x1 = pos.x.0;
        let z1 = pos.z.0;

        let far = pos + size;
        let dist = motion.to - pos;

        let width = far.x - pos.x + directional_size;
        let height = far.z - pos.z + directional_size;

        let scaled_dist = Fixed32((height.0 * (directional_size + dist.x).0) / width.0);
        if scaled_dist <= dist.z {
            return motion.to;
        }

        let x1_div = x1 / 0x12;
        let z1_div = z1 / 0x12;
        let z2_div = far.z.0 / 0x12;
        let x2_div = far.x.0 / 0x12;
        let height_div = z2_div - z1_div;
        let width_div = x2_div - x1_div;

        if (((motion.from().x.0 / 0x12) * height_div - (motion.from().z.0 / 0x12) * width_div) - z2_div * x1_div) + x2_div * z1_div < 1 {
            let term1 = dist.z - scaled_dist;
            let term2 = Fixed32((width.0 * term1.0) / height.0);
            let (x_adjustment, z_adjustment) = tri_adjustments(term1, term2);
            if x_adjustment.abs() < RECT_THRESHOLD && z_adjustment.abs() < RECT_THRESHOLD {
                Vec2::new(motion.to.x + x_adjustment, motion.to.z - z_adjustment)
            } else {
                motion.from()
            }
        } else if dist.x < (size.x + directional_size) && (dist.z + directional_size) < (size.z + directional_size) {
            rect_clip_motion(&self.pos, motion)
        } else {
            motion.to
        }
    }

    fn clip_motion_bottom_left(&self, motion: &Motion) -> Vec2 {
        let pos = self.pos.pos;
        let size = self.pos.size;

        let directional_size = motion.size_in_direction_of(pos, size);

        let x1 = pos.x.0;
        let z1 = pos.z.0;

        let far = pos + size;

        let width = directional_size + (far.x - pos.x);
        let height = (pos.z - far.z) - directional_size;

        let dist = motion.to - pos;

        let scaled_dist = Fixed32((height.0 * dist.x.0) / width.0);
        if scaled_dist <= (motion.to.z - far.z) - directional_size {
            return motion.to;
        }

        let x1_div = x1 / 0x12;
        let z2_div = far.z.0 / 0x12;
        let x2_div = far.x.0 / 0x12;
        let height_div = z1 / 0x12 - z2_div;
        let width_div = x2_div - x1_div;

        if (((motion.from().x.0 / 0x12) * height_div - (motion.from().z.0 / 0x12) * width_div) - (z1 / 0x12) * x1_div) + x2_div * z2_div < 1 {
            let term1 = motion.to.z - far.z - scaled_dist - directional_size;
            let term2 = Fixed32((width.0 * term1.0) / (far.z - pos.z + directional_size).0);
            let (x_adjustment, z_adjustment) = tri_adjustments(term1, term2);
            if x_adjustment.abs() < RECT_THRESHOLD && z_adjustment.abs() < RECT_THRESHOLD {
                Vec2::new(motion.to.x - x_adjustment, motion.to.z - z_adjustment)
            } else {
                motion.from()
            }
        } else if (dist.x + directional_size) < (size.x + directional_size) && (dist.z + directional_size) < (size.z + directional_size) {
            rect_clip_motion(&self.pos, motion)
        } else {
            motion.to
        }
    }

    pub fn clip_motion(&self, motion: &Motion) -> Vec2 {
        if !motion.is_destination_in_collision_bounds(&self.pos) {
            return motion.to;
        }

        match self.type_ {
            TriangleType::BottomLeft => self.clip_motion_bottom_left(motion),
            TriangleType::BottomRight => self.clip_motion_bottom_right(motion),
            TriangleType::TopLeft => self.clip_motion_top_left(motion),
            TriangleType::TopRight => self.clip_motion_top_right(motion),
        }
    }

    pub fn contains_point<T: Into<Vec2>>(&self, point: T) -> bool {
        let point = point.into();

        self.clip_motion(&Motion::point_with_motion(point, Floor::ANY)) != point
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

    pub fn clip_motion(&self, motion: &Motion) -> Vec2 {
        match self {
            Self::Rect(rect) => rect.clip_motion(motion),
            Self::Ellipse(ellipse) => ellipse.clip_motion(motion),
            Self::Diamond(diamond) => diamond.clip_motion(motion),
            Self::Triangle(triangle) => triangle.clip_motion(motion),
            // quads never have collision
            Self::Quad(_) => motion.to,
        }
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
            Self::Rect(RectCollider { pos, .. })
            | Self::Diamond(DiamondCollider { pos, .. })
            | Self::Ellipse(EllipseCollider { pos, .. })
            | Self::Triangle(TriangleCollider { pos, .. })
            => {
                format!("X: {: >6} | Z: {: >6}\nW: {: >6} | H: {: >6}", pos.pos.x, pos.pos.z, pos.size.x, pos.size.z)
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
            Self::Rect(RectCollider { pos, .. })
            | Self::Diamond(DiamondCollider { pos, .. })
            | Self::Ellipse(EllipseCollider { pos, .. })
            | Self::Triangle(TriangleCollider { pos, .. })
            => {
                let mut params = vec![
                    format!("X: {}", pos.pos.x),
                    format!("Z: {}", pos.pos.z),
                    format!("W: {}", pos.size.x),
                    format!("H: {}", pos.size.z),
                    format!("Floor: {}", pos.floor),
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
                let x_radius = ellipse.pos.size.x >> 1;
                let z_radius = ellipse.pos.size.z >> 1;
                let center_x = ellipse.pos.pos.x + x_radius;
                let center_z = ellipse.pos.pos.z + z_radius;
                
                groups.push((label, vec![
                    format!("CX: {}", center_x),
                    format!("CZ: {}", center_z),
                    format!("RX: {}", x_radius),
                    format!("RZ: {}", z_radius),
                ]));
            }
            Self::Triangle(tri) => {
                let offsets = tri.offsets();

                let x1 = tri.pos.pos.x + if offsets[0].0 > 0.0 { tri.pos.size.x } else { Fixed32(0) };
                let z1 = tri.pos.pos.z + if offsets[0].1 > 0.0 { tri.pos.size.z } else { Fixed32(0) };
                let x2 = tri.pos.pos.x + if offsets[1].0 > 0.0 { tri.pos.size.x } else { Fixed32(0) };
                let z2 = tri.pos.pos.z + if offsets[1].1 > 0.0 { tri.pos.size.z } else { Fixed32(0) };
                let x3 = tri.pos.pos.x + if offsets[2].0 > 0.0 { tri.pos.size.x } else { Fixed32(0) };
                let z3 = tri.pos.pos.z + if offsets[2].1 > 0.0 { tri.pos.size.z } else { Fixed32(0) };
                
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
                let radius_x = diamond.pos.size.x >> 1;
                let radius_z = diamond.pos.size.z >> 1;

                let x = diamond.pos.pos.x;
                let z = diamond.pos.pos.z;
                let width = diamond.pos.size.x;
                let height = diamond.pos.size.z;
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
                let nx = rect.pos.pos.x;
                let nz = rect.pos.pos.z;
                let fx = rect.pos.pos.x + rect.pos.size.x;
                let fz = rect.pos.pos.z + rect.pos.size.z;
                
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
            Self::Rect(rect) => rect.pos.floor,
            Self::Diamond(diamond) => diamond.pos.floor,
            Self::Ellipse(ellipse) => ellipse.pos.floor,
            Self::Triangle(triangle) => triangle.pos.floor,
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