use egui::{Color32, Pos2, Shape, Stroke};
use epaint::{CircleShape, ColorMode, PathShape, PathStroke};
use residat::common::{Fixed16, UFixed16, Fixed32, Vec2, Vec3};
use residat::re2::{CharacterId, Item, MAX_PARTS};

use crate::app::{DrawParams, Floor, GameObject, ObjectType, WorldPos};
use crate::collision::{CapsuleType, EllipseCollider, Motion, RectCollider};
use crate::record::State;

mod ai;
pub use ai::*;

mod hit;
pub use hit::*;

const INTERACTION_DISTANCE: Fixed32 = Fixed32(620);
pub const PLAYER_COLLISION_MASK: u16 = 0x8000;
pub const ENEMY_COLLISION_MASK: u16 = 0x400;
pub const ENEMY_EXTRA_COLLISION_MASK: u16 = 0x200;
pub const SHERRY_COLLISION_MASK: u16 = 0x800;
pub const ALLY_COLLISION_MASK: u16 = 0x1000;

const ARROW_HEAD_HEIGHT: f32 = 6.0;
const ARROW_HEAD_WIDTH: f32 = 6.0;
const ARROW_SHAFT_WIDTH: f32 = 1.5;
const MOTION_PROJECTION_LENGTH: f32 = 0.25;
const POINT_RADIUS: f32 = 3.0;

const SLOW_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 0, 0, 255);
const FAST_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 255, 0, 255);

const CHARACTER_COLLISION_DENY: u16 = 0x100;

const FLAG_ENABLED: u32 = 1;
const FLAG_NO_COLLISION: u32 = 2;
const FLAG_COLLISION_RECEIVER: u32 = 4;
const FLAG_COLLISION_MUTUAL_EXCLUSION: u32 = 0x1000;
const FLAG_LIMIT_COLLISION_DISPLACEMENT: u32 = 0x100000;

const COLLISION_DISPLACEMENT_LIMIT: Fixed32 = Fixed32(100);

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum CharacterType {
    Player,
    Ally,
    Neutral,
    Enemy,
}

impl CharacterType {
    pub const fn from_character_id(id: CharacterId) -> Self {
        match id {
            CharacterId::AdaNpc | CharacterId::SherryNpc => CharacterType::Ally,
            CharacterId::FuseArm
            | CharacterId::FuseHousing
            | CharacterId::Irons
            | CharacterId::IronsTorso
            | CharacterId::AdaWounded
            | CharacterId::BenDead
            | CharacterId::Ben
            | CharacterId::Annette
            | CharacterId::Kendo
            | CharacterId::Marvin
            | CharacterId::MayorsDaughter
            | CharacterId::LeonNpc
            | CharacterId::ClaireNpc
            | CharacterId::LeonBandagedNpc
            => CharacterType::Neutral,
            _ if id.is_player() => CharacterType::Player,
            _ => CharacterType::Enemy,
        }
    }
}

impl From<CharacterId> for CharacterType {
    fn from(id: CharacterId) -> Self {
        Self::from_character_id(id)
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub flags: u32,
    pub center: Vec2,
    pub size: Vec2,
    pub shape: RectCollider,
    floor: Floor,
    pub index: usize,
}

impl Object {
    pub const fn new(flags: u32, x: Fixed32, z: Fixed32, width: UFixed16, height: UFixed16, floor: Floor) -> Self {
        let game_x = Fixed32(x.0 - width.0 as i32);
        let game_z = Fixed32(z.0 - height.0 as i32);
        let game_width = UFixed16(width.0 << 1).to_32();
        let game_height = UFixed16(height.0 << 1).to_32();

        let center = Vec2 { x, z };
        let size = Vec2 { x: Fixed32(width.0 as i32), z: Fixed32(height.0 as i32) };

        Self {
            flags,
            center,
            size,
            shape: RectCollider::new(WorldPos::rect(Vec2 {x: game_x, z: game_z }, Vec2 { x: game_width, z: game_height }, floor), CapsuleType::None),
            floor,
            index: usize::MAX,
        }
    }

    pub const fn empty() -> Self {
        Self::new(0, Fixed32(0), Fixed32(0), UFixed16(0), UFixed16(0), Floor::Id(0))
    }

    pub fn set_floor(&mut self, floor: Floor) {
        self.floor = floor;
        self.shape.set_floor(floor);
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub fn set_pos(&mut self, x: impl Into<Fixed32>, z: impl Into<Fixed32>) {
        self.center = Vec2::new(x, z);
        let pos = self.center - self.size;
        self.shape.set_pos(pos);
    }

    pub fn set_size(&mut self, width: impl Into<Fixed32>, height: impl Into<Fixed32>) {
        self.size.x = width.into();
        self.size.z = height.into();
        let size = Vec2::new(self.size.x << 1, self.size.z << 1);
        self.shape.set_size(size);
    }

    pub const fn is_pushable(&self) -> bool {
        self.flags & 2 == 0
    }
}

impl GameObject for Object {
    fn object_type(&self) -> ObjectType {
        ObjectType::Object
    }

    fn contains_point(&self, point: Vec2) -> bool {
        self.shape.contains_point(point)
    }

    fn name(&self) -> String {
        String::from("Object")
    }

    fn name_prefix(&self, _index: usize) -> String {
        format!("Object #{}", self.index)
    }

    fn description(&self) -> String {
        format!("X: {} | Z: {}", self.center.x, self.center.z)
    }

    fn details(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = Vec::new();

        groups.push((String::from("Object"), vec![
            format!("Flags: {:08X}", self.flags),
        ]));

        groups.push((String::from("Position"), vec![
            format!("X: {}", self.center.x),
            format!("Z: {}", self.center.z),
            format!("XR: {}", self.size.x),
            format!("ZR: {}", self.size.z),
            format!("Floor: {}", self.floor),
        ]));

        groups
    }

    fn floor(&self) -> Floor {
        self.floor
    }

    fn gui_shape(&self, params: &DrawParams, _state: &State) -> Shape {
        self.shape.gui_shape(params)
    }
}

#[derive(Debug, Clone)]
pub struct Part {
    pos: Vec3,
    size: Vec3,
    size_offset: UFixed16,
}

impl Part {
    pub const fn new(pos: Vec3, size: Vec3, size_offset: UFixed16) -> Self {
        Self { pos, size, size_offset }
    }

    pub const fn from_pos(pos: Vec3) -> Self {
        Self::new(pos, Vec3::zero(), UFixed16(0))
    }

    pub const fn from_size(size: Vec3, offset: UFixed16) -> Self {
        Self::new(Vec3::zero(), size, offset)
    }

    pub fn set_pos(&mut self, pos: impl Into<Vec3>) {
        self.pos = pos.into();
    }

    pub fn set_size(&mut self, x: impl Into<Fixed32>, y: impl Into<Fixed32>, z: impl Into<Fixed32>, offset: impl Into<UFixed16>) {
        self.size = Vec3::new(x, y, z);
        self.size_offset = offset.into();
    }

    pub fn local_pos(&self, root_pos: Vec3, angle: Fixed32) -> Vec3 {
        (self.pos - root_pos).rotate_y(-angle)
    }
}

#[derive(Debug, Clone)]
pub struct Character {
    pub flags: u32,
    pub id: CharacterId,
    center: Vec3,
    prev_center: Vec3,
    parts: [Option<Part>; MAX_PARTS],
    part_offset: Vec2,
    model_part_centers: Vec<Vec2>,
    pub size: Vec2,
    pub shape: EllipseCollider,
    pub outline_shape: RectCollider,
    pub angle: Fixed32,
    current_health: i16,
    max_health: i16,
    pub state: [u8; 4],
    floor: Floor,
    pub velocity: Vec2,
    pub type_: u8,
    pub index: usize,
}

impl Character {
    pub const fn new(flags: u32, id: CharacterId, health: i16, x: Fixed32, y: Fixed32, z: Fixed32, width: UFixed16, height: UFixed16, angle: Fixed16, floor: Floor, velocity: Vec2) -> Self {
        let game_x = Fixed32(x.0 - width.0 as i32);
        let game_z = Fixed32(z.0 - height.0 as i32);
        let game_width = UFixed16(width.0 << 1).to_32();
        let game_height = UFixed16(height.0 << 1).to_32();
        
        let center = Vec3 { x, y, z };

        Self {
            flags,
            id,
            center,
            prev_center: center,
            parts: [const { None }; MAX_PARTS],
            part_offset: Vec2::zero(),
            model_part_centers: Vec::new(),
            size: Vec2 { x: Fixed32(width.0 as i32), z: Fixed32(height.0 as i32) },
            shape: EllipseCollider::new(WorldPos::rect(Vec2 { x: game_x, z: game_z }, Vec2 { x: game_width, z: game_height }, floor)),
            outline_shape: RectCollider::new(WorldPos::rect(Vec2 { x: game_x, z: game_z }, Vec2 { x: game_width, z: game_height }, floor), CapsuleType::None),
            angle: angle.to_32(),
            current_health: health,
            max_health: health,
            state: [0; 4],
            floor,
            velocity,
            type_: 0,
            index: usize::MAX,
        }
    }

    pub const fn empty(id: CharacterId) -> Self {
        Self::new(0, id, 0, Fixed32(0), Fixed32(0), Fixed32(0), UFixed16(0), UFixed16(0), Fixed16(0), Floor::Id(0), Vec2::zero())
    }

    pub const fn is_enabled(&self) -> bool {
        self.flags & FLAG_ENABLED != 0
    }

    pub const fn has_collision(&self) -> bool {
        self.flags & FLAG_NO_COLLISION == 0
    }

    pub const fn is_collision_receiver(&self) -> bool {
        self.flags & FLAG_COLLISION_RECEIVER != 0
    }

    pub const fn has_collision_mutual_exclusion(&self) -> bool {
        self.flags & FLAG_COLLISION_MUTUAL_EXCLUSION != 0
    }

    pub const fn has_limited_collision_displacement(&self) -> bool {
        self.flags & FLAG_LIMIT_COLLISION_DISPLACEMENT != 0
    }

    pub const fn center(&self) -> Vec2 {
        self.center.xz()
    }

    pub const fn center_3d(&self) -> Vec3 {
        self.center
    }

    pub const fn prev_center(&self) -> Vec2 {
        self.prev_center.xz()
    }

    pub const fn prev_center_3d(&self) -> Vec3 {
        self.prev_center
    }

    pub fn set_floor(&mut self, floor: Floor) {
        self.floor = floor;
        self.shape.set_floor(floor);
        self.outline_shape.set_floor(floor);
    }

    pub const fn name(&self) -> &'static str {
        self.id.name()
    }

    pub const fn type_(&self) -> CharacterType {
        CharacterType::from_character_id(self.id)
    }

    pub const fn current_health(&self) -> i16 {
        self.current_health
    }

    pub const fn max_health(&self) -> i16 {
        self.max_health
    }

    pub const fn set_health(&mut self, health: i16) {
        self.current_health = health;
        if self.max_health <= 0 {
            self.max_health = health;
        }
    }
    
    pub const fn index(&self) -> usize {
        self.index
    }
    
    pub const fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub const fn parts(&self) -> &[Option<Part>] {
        &self.parts
    }

    pub fn active_parts(&self) -> impl Iterator<Item = &Part> {
        self.parts.iter().filter_map(Option::as_ref)
    }

    pub const fn parts_mut(&mut self) -> &mut [Option<Part>] {
        &mut self.parts
    }

    pub fn active_parts_mut(&mut self) -> impl Iterator<Item = &mut Part> {
        self.parts.iter_mut().filter_map(Option::as_mut)
    }

    pub const fn part_center(&self) -> Vec2 {
        match &self.parts[0] {
            Some(part) => part.pos.xz(),
            None => Vec2::zero(),
        }
    }
    
    pub const fn part_offset(&self) -> Vec2 {
        self.part_offset
    }
    
    pub const fn set_part_offset(&mut self, part_offset: Vec2) {
        self.part_offset = part_offset;
    }

    pub fn model_part_centers(&self) -> &[Vec2] {
        &self.model_part_centers
    }

    pub fn set_model_part_center(&mut self, i: usize, model_part_center: Vec2) {
        if self.model_part_centers.len() <= i {
            self.model_part_centers.resize(i + 1, Vec2::zero());
        }
        self.model_part_centers[i] = model_part_center;
    }

    pub fn gui_interaction_point(&self, draw_params: &DrawParams) -> Pos2 {
        let interaction_point = self.interaction_point();
        let (x, y, _, _) = draw_params.transform(interaction_point.x, interaction_point.z, 0, 0);
        Pos2::new(x, y)
    }

    pub fn interaction_point(&self) -> Vec2 {
        let interaction_vec = Vec2::new(INTERACTION_DISTANCE, 0).rotate_y(self.angle);
        self.center.xz() + interaction_vec
    }

    pub fn set_pos(&mut self, pos: impl Into<Vec3>) {
        self.center = pos.into();
        let pos = self.center.xz() - self.size;
        self.shape.set_pos(pos);
        self.outline_shape.set_pos(pos);
    }
    
    pub fn set_prev_pos(&mut self, pos: impl Into<Vec3>) {
        self.prev_center = pos.into();
    }

    pub fn set_size(&mut self, width: impl Into<Fixed32>, height: impl Into<Fixed32>) {
        self.size.x = width.into();
        self.size.z = height.into();
        let size = Vec2::new(self.size.x << 1, self.size.z << 1);
        self.shape.set_size(size);
        self.outline_shape.set_size(size);
    }

    pub const fn collision_mask(&self) -> u16 {
        match self.id {
            CharacterId::SherryNpc | CharacterId::SherryVest => SHERRY_COLLISION_MASK,
            // FIXME: G4 also uses the extra collision mask but only in certain circumstances
            CharacterId::Spider => ENEMY_COLLISION_MASK | ENEMY_EXTRA_COLLISION_MASK,
            // TODO: figure out if all neutral NPCs use this collision or only the actual sidekicks
            CharacterId::FuseArm | CharacterId::FuseHousing | CharacterId::Irons | CharacterId::AdaNpc
            | CharacterId::IronsTorso | CharacterId::AdaWounded | CharacterId::BenDead | CharacterId::Ben
            | CharacterId::Annette | CharacterId::Kendo | CharacterId::Unknown73
            | CharacterId::MayorsDaughter | CharacterId::Unknown76 | CharacterId::Unknown77
            | CharacterId::Unknown78 | CharacterId::LeonNpc | CharacterId::ClaireNpc | CharacterId::Unknown82
            | CharacterId::Unknown83 | CharacterId::LeonBandagedNpc => ALLY_COLLISION_MASK,
            _ if self.id.is_player() => PLAYER_COLLISION_MASK,
            _ => ENEMY_COLLISION_MASK,
        }
    }
    
    pub fn motion(&self) -> Motion {
        Motion::new(
            WorldPos::new(self.prev_center.xz(), self.size, self.floor, self.collision_mask(), CHARACTER_COLLISION_DENY),
            self.center.xz(),
            self.part_offset(),
        )
    }

    pub const fn is_moving(&self) -> bool {
        // only supported for player for now
        if !self.id.is_player() {
            return false;
        }

        matches!(self.state,
            [0x01, 0x01, _, _] // walking
            | [0x01, 0x02, _, _] // running
            | [0x01, 0x03, _, _] // backpedaling
            | [0x01, 0x07, 0x03 | 0x04 | 0x05 | 0x06 | 0x07, _] // ?? unknown
            // disabled for now because the movement only happens on certain animation frames, which
            // we don't track at the moment
            // | [0x01, 0x08, _, 0x02 | 0x03] // climbing up
            | [0x01, 0x09, _, 0x02] // ?? unknown
            | [0x01, 0x0a, 0x04 | 0x05, _] // pushing object
        )
    }

    pub fn clone_for_collision(&self) -> Self {
        let mut clone = self.clone();
        let directed_velocity = Vec3::from(self.velocity.rotate_y(self.angle));
        let motion_center = self.prev_center + directed_velocity;

        for part in clone.active_parts_mut() {
            part.pos = (part.pos - self.center) + motion_center;
        }
        
        clone.center = motion_center;

        clone
    }

    pub fn collide_with(&mut self, receiver: &Self) -> bool {
        if !receiver.is_enabled() {
            return false;
        }

        if !self.has_collision() || !receiver.has_collision() {
            return false;
        }

        if self.has_collision_mutual_exclusion() && receiver.has_collision_mutual_exclusion() {
            return false;
        }

        if self.is_collision_receiver() {
            return false;
        }

        let mut had_collision = false;
        for receiver_part in receiver.active_parts() {
            for collider_part in &mut self.parts {
                let Some(collider_part) = collider_part else {
                    continue;
                };
                
                let combined_size = (collider_part.size_offset + receiver_part.size_offset).to_32();
                let extent = (combined_size << 1).0 as u32;

                let receiver_extent = (receiver_part.size_offset << 1).to_32();

                let rel = collider_part.pos - receiver_part.pos;
                let x_extent = (rel.x + combined_size).0 as u32;
                let z_extent = (rel.z + combined_size).0 as u32;

                if x_extent > extent || z_extent > extent {
                    continue;
                }

                let distance_2d = rel.xz().len();
                let overlap = combined_size - distance_2d;
                if !overlap.is_positive() {
                    continue;
                }

                let y_size = collider_part.size.y + receiver_part.size.y;
                if -y_size >= rel.y || rel.y >= y_size {
                    continue;
                }

                let mut x_adjust = rel.x.mul_div(overlap, distance_2d.inc());
                let mut z_adjust = rel.z.mul_div(overlap, distance_2d.inc());

                let collider_local = collider_part.local_pos(self.center, self.angle);
                let y_distance = collider_local.y + self.prev_center.y - receiver_part.pos.y;

                if y_distance <= -y_size || y_size <= y_distance {
                    if (self.prev_center.x < receiver.center.x && receiver.center.x < self.center.x) || (receiver.center.x < self.prev_center.x && self.center.x < receiver.center.x) {
                        if !x_adjust.is_negative() {
                            x_adjust = -(-x_adjust + receiver_extent);
                        } else {
                            x_adjust += receiver_extent;
                        }
                    }

                    if (self.prev_center.z < receiver.center.z && receiver.center.z < self.center.z) || (receiver.center.z < self.prev_center.z && self.center.z < receiver.center.z) {
                        if !z_adjust.is_negative() {
                            z_adjust = -(-z_adjust + receiver_extent);
                        } else {
                            z_adjust += receiver_extent;
                        }
                    }
                }

                if receiver.has_limited_collision_displacement() {
                    if x_adjust.abs() > COLLISION_DISPLACEMENT_LIMIT {
                        x_adjust = if x_adjust.is_negative() { -COLLISION_DISPLACEMENT_LIMIT } else { COLLISION_DISPLACEMENT_LIMIT };
                    }

                    if z_adjust.abs() > COLLISION_DISPLACEMENT_LIMIT {
                        z_adjust = if z_adjust.is_negative() { -COLLISION_DISPLACEMENT_LIMIT } else { COLLISION_DISPLACEMENT_LIMIT };
                    }
                }

                // we're moved by the collision
                self.center.x += x_adjust;
                self.center.z += z_adjust;

                collider_part.pos.x += x_adjust;
                collider_part.pos.z += z_adjust;
                
                had_collision = true;
            }
        }

        had_collision
    }

    const fn is_crawling_zombie(&self) -> bool {
        self.id.is_zombie() && matches!(self.type_ & 0x3f, 1 | 3 | 5 | 7 | 9 | 11 | 13)
    }

    fn describe_state(&self) -> String {
        String::from(if self.is_crawling_zombie() {
            describe_crawling_zombie_ai_state(&self.state)
        } else if self.id.is_zombie() {
            describe_zombie_ai_state(&self.state)
        } else if self.id.is_player() {
            describe_player_ai_state(&self.state)
        } else if self.id.is_licker() {
            describe_licker_ai_state(&self.state)
        } else if self.id == CharacterId::Dog {
            describe_dog_ai_state(&self.state)
        } else if self.id == CharacterId::Spider {
            describe_spider_ai_state(&self.state)
        } else if self.id == CharacterId::G2 {
            describe_g2_ai_state(&self.state)
        } else {
            "Unknown"
        })
    }

    pub fn ai_zones(&self) -> Vec<PositionedAiZone> {
        let ai_zones = match self.id {
            CharacterId::LickerRed => &RED_LICKER_AI_ZONES[..],
            CharacterId::LickerBlack => &BLACK_LICKER_AI_ZONES[..],
            CharacterId::Dog => &DOG_AI_ZONES[..],
            CharacterId::Spider => &SPIDER_AI_ZONES[..],
            CharacterId::G2 => &G2_AI_ZONES[..],
            _ if self.is_crawling_zombie() => &CRAWLING_ZOMBIE_AI_ZONES[..],
            _ if self.id.is_zombie() => &ZOMBIE_AI_ZONES[..],
            _ => return Vec::new(),
        };
        
        let mut positioned_ai_zones = Vec::new();
        for ai_zone in ai_zones {
            if !ai_zone.check_state(&self.state, self.type_ & 0x3f) {
                // zone is not active in this state; skip it
                continue;
            }

            let pos = match ai_zone.origin {
                ZoneOrigin::Base => self.center.xz(),
                ZoneOrigin::Part(i) => {
                    if i != 0 {
                        continue;
                    }

                    self.part_center()
                }
                ZoneOrigin::ModelPart(i) => {
                    if i >= self.model_part_centers.len() {
                        continue;
                    }

                    self.model_part_centers[i]
                }
            };

            positioned_ai_zones.push(PositionedAiZone::new(ai_zone, self.id, self.index, pos, self.angle, self.floor));
        }

        positioned_ai_zones
    }
    
    pub fn equipped_item(&self) -> Option<Item> {
        if self.id.is_player() {
            Item::try_from(self.type_ as u16).ok()
        } else {
            None
        }
    }
}

impl GameObject for Character {
    fn object_type(&self) -> ObjectType {
        CharacterType::from_character_id(self.id).into()
    }

    fn contains_point(&self, point: Vec2) -> bool {
        self.shape.contains_point(point)
    }
    
    fn name(&self) -> String {
        self.id.name().to_string()
    }

    fn name_prefix(&self, _index: usize) -> String {
        format!("#{}", self.index)
    }
    
    fn description(&self) -> String {
        format!(
            "State: {:02X} {:02X} {:02X} {:02X}\nHP: {}/{}",
            self.state[0], self.state[1], self.state[2], self.state[3],
            self.current_health, self.max_health,
        )
    }

    fn details(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = Vec::new();

        groups.push((String::from("Character"), vec![
            format!("Type: {} ({})", self.name(), self.id as u8),
            if self.id.is_player() {
                format!("Equipped: {}", Item::name_from_id(self.type_ as u16))
            } else {
                format!("Sub-type: {}", self.type_ & 0x3f)
            },
            format!("HP: {}/{}", self.current_health, self.max_health),
        ]));

        groups.push((String::from("Position"), vec![
            format!("X: {}", self.center.x),
            format!("Z: {}", self.center.z),
            format!("Angle: {:.1}°", self.angle.to_degrees() % 360.0),
            format!("Floor: {}", self.floor),
            format!("XR: {}", self.size.x),
            format!("ZR: {}", self.size.z),
        ]));

        groups.push((String::from("Velocity"), vec![
            format!("X: {}", self.velocity.x),
            format!("Z: {}", self.velocity.z),
            format!("Base: {}", self.velocity.len()),
            format!("Effective: {}", (self.center - self.prev_center).len()),
        ]));

        groups.push((String::from("State"), vec![
            format!("{:02X} {:02X} {:02X} {:02X}", self.state[0], self.state[1], self.state[2], self.state[3]),
            self.describe_state(),
        ]));

        groups
    }

    fn floor(&self) -> Floor {
        self.floor
    }

    fn collision_mask(&self) -> u16 {
        if self.id.is_player() {
            PLAYER_COLLISION_MASK
        } else {
            0xFFFF
        }
    }
    
    fn gui_shape(&self, draw_params: &DrawParams, _state: &State) -> Shape {
        let body_shape = self.shape.gui_shape(draw_params);
        let body_rect = body_shape.visual_bounding_rect();
        let body_center = body_rect.center();

        let mut outline_draw_params = draw_params.clone();
        outline_draw_params.stroke.color = outline_draw_params.fill_color;
        outline_draw_params.stroke.width = ARROW_SHAFT_WIDTH;
        outline_draw_params.stroke_kind = egui::StrokeKind::Inside;
        outline_draw_params.fill_color = Color32::TRANSPARENT;
        let outline_shape = self.outline_shape.gui_shape(&outline_draw_params);

        let vector = egui::Vec2::angled(self.angle.to_radians()) * MOTION_PROJECTION_LENGTH * draw_params.scale;
        let dest_pos = body_center + vector;
        let vector_len = vector.length();
        let shaft_pos = body_center + ((vector_len - ARROW_HEAD_HEIGHT) / vector_len).max(0.0) * vector;
        let side_vector = vector.normalized().rot90() * ARROW_HEAD_WIDTH;
        let arrow_point1 = shaft_pos - side_vector;
        let arrow_point3 = shaft_pos + side_vector;

        let shaft_shape = Shape::line_segment(
            [body_center, shaft_pos],
            Stroke {
                width: ARROW_SHAFT_WIDTH,
                color: draw_params.fill_color,
            },
        );

        let arrow_shape = Shape::Path(PathShape {
            points: vec![shaft_pos, arrow_point1, dest_pos, arrow_point3],
            closed: true,
            stroke: PathStroke {
                width: draw_params.stroke.width,
                color: ColorMode::Solid(draw_params.stroke.color),
                kind: draw_params.stroke_kind,
            },
            fill: draw_params.fill_color,
        });

        let mut shapes = vec![outline_shape, body_shape, shaft_shape, arrow_shape];

        if self.id.is_player() {
            let interaction_point = Shape::Circle(CircleShape {
                center: self.gui_interaction_point(&draw_params),
                radius: POINT_RADIUS,
                fill: draw_params.fill_color,
                stroke: draw_params.stroke,
            });

            shapes.push(interaction_point);
        }

        Shape::Vec(shapes)
    }
}

#[derive(Debug, Clone)]
pub struct CharacterPath {
    pub points: Vec<Vec2>,
    pub character_id: CharacterId,
    pub character_index: usize,
    pub floor: Floor,
    pub limit: usize,
    pub dynamic_color: bool,
}

impl CharacterPath {
    pub const fn new(points: Vec<Vec2>, character_id: CharacterId, character_index: usize, floor: Floor) -> Self {
        Self { points, character_id, character_index, floor, limit: usize::MAX, dynamic_color: true }
    }
    
    pub fn len(&self) -> Fixed32 {
        self.points.iter().fold(Fixed32(0), |acc, p| acc + p.len())
    }
    
    pub fn max_speed(&self) -> Fixed32 {
        self.points.windows(2).fold(Fixed32(0), |acc, p| acc.max((p[1] - p[0]).len()))
    }
    
    pub const fn frames(&self) -> usize {
        self.points.len()
    }
    
    pub fn initial_segment(&self) -> &[Vec2] {
        let limit = self.limit.min(self.points.len());
        &self.points[0..limit]
    }
}

impl GameObject for CharacterPath {
    fn object_type(&self) -> ObjectType {
        ObjectType::CharacterPath
    }
    
    fn contains_point(&self, _point: Vec2) -> bool {
        false
    }
    
    fn name(&self) -> String {
        format!("{} path", self.character_id.name())
    }

    fn description(&self) -> String {
        format!("Frames: {} | Length: {}", self.points.len(), self.len())
    }

    fn details(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = Vec::new();

        groups.push((String::from("Path"), vec![
            format!("Frames: {}", self.points.len()),
            format!("Length: {}", self.len()),
        ]));
        
        groups
    }

    fn floor(&self) -> Floor {
        self.floor
    }

    fn gui_shape(&self, params: &DrawParams, _state: &State) -> Shape {
        let max_speed = self.max_speed().to_f32();
        let mut shapes = Vec::new();
        
        for segment in self.initial_segment().windows(2) {
            let start = segment[0];
            let end = segment[1];
            let speed = (end - start).len().to_f32();
            if speed <= 0.0 {
                // TODO: draw a circle or something here
                continue;
            }
            
            let gui_start = params.transform_point(start);
            let gui_end = params.transform_point(end);
            
            let mut stroke = params.stroke.clone();
            if self.dynamic_color {
                let t = speed / max_speed;
                let color = SLOW_COLOR.lerp_to_gamma(FAST_COLOR, t).gamma_multiply_u8(params.color().a());
                stroke.color = color;
            }
            
            shapes.push(Shape::line_segment([gui_start, gui_end], stroke));
        }
        
        Shape::Vec(shapes)
    }
}