use egui::{Color32, Pos2, Shape, Stroke};
use epaint::{CircleShape, ColorMode, PathShape, PathStroke};
use residat::common::{Fixed16, UFixed16, Fixed32, Vec2};
use residat::re2::{CharacterId, Item};

use crate::app::{DrawParams, Floor, GameObject, ObjectType};
use crate::collision::{CapsuleType, EllipseCollider, RectCollider};
use crate::record::State;

mod ai;
pub use ai::*;

mod hit;
pub use hit::*;

const INTERACTION_DISTANCE: Fixed32 = Fixed32(620);
pub const PLAYER_COLLISION_MASK: u16 = 0x8000;

const ARROW_HEAD_HEIGHT: f32 = 6.0;
const ARROW_HEAD_WIDTH: f32 = 6.0;
const ARROW_SHAFT_WIDTH: f32 = 1.5;
const MOTION_PROJECTION_LENGTH: f32 = 0.25;
const POINT_RADIUS: f32 = 3.0;

const SLOW_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 0, 0, 255);
const FAST_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 255, 0, 255);

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
            shape: RectCollider::new(game_x, game_z, game_width, game_height, floor, CapsuleType::None),
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
pub struct Character {
    pub id: CharacterId,
    pub center: Vec2,
    pub prev_center: Vec2,
    part_center: Vec2,
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
    pub const fn new(id: CharacterId, health: i16, x: Fixed16, z: Fixed16, width: UFixed16, height: UFixed16, angle: Fixed16, floor: Floor, velocity: Vec2) -> Self {
        let game_x = Fixed32(x.0 as i32 - width.0 as i32);
        let game_z = Fixed32(z.0 as i32 - height.0 as i32);
        let game_width = UFixed16(width.0 << 1).to_32();
        let game_height = UFixed16(height.0 << 1).to_32();
        
        let center = Vec2 { x: Fixed32(x.0 as i32), z: Fixed32(z.0 as i32) };

        Self {
            id,
            center,
            prev_center: center,
            part_center: Vec2::zero(),
            model_part_centers: Vec::new(),
            size: Vec2 { x: Fixed32(width.0 as i32), z: Fixed32(height.0 as i32) },
            shape: EllipseCollider::new(game_x, game_z, game_width, game_height, floor),
            outline_shape: RectCollider::new(game_x, game_z, game_width, game_height, floor, CapsuleType::None),
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
        Self::new(id, 0, Fixed16(0), Fixed16(0), UFixed16(0), UFixed16(0), Fixed16(0), Floor::Id(0), Vec2::zero())
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

    pub const fn part_center(&self) -> Vec2 {
        self.part_center
    }

    pub const fn set_part_center(&mut self, part_center: Vec2) {
        self.part_center = part_center;
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
        self.center + interaction_vec
    }

    pub fn set_pos(&mut self, x: impl Into<Fixed32>, z: impl Into<Fixed32>) {
        self.center = Vec2::new(x, z);
        let pos = self.center - self.size;
        self.shape.set_pos(pos);
        self.outline_shape.set_pos(pos);
    }
    
    pub fn set_prev_pos(&mut self, x: impl Into<Fixed32>, z: impl Into<Fixed32>) {
        self.prev_center = Vec2::new(x, z);
    }

    pub fn set_size(&mut self, width: impl Into<Fixed32>, height: impl Into<Fixed32>) {
        self.size.x = width.into();
        self.size.z = height.into();
        let size = Vec2::new(self.size.x << 1, self.size.z << 1);
        self.shape.set_size(size);
        self.outline_shape.set_size(size);
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
                ZoneOrigin::Base => self.center,
                ZoneOrigin::Part(i) => {
                    if i != 0 {
                        continue;
                    }

                    self.part_center
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
}

impl CharacterPath {
    pub const fn new(points: Vec<Vec2>, character_id: CharacterId, character_index: usize, floor: Floor) -> Self {
        Self { points, character_id, character_index, floor }
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
        
        for segment in self.points.windows(2) {
            let start = segment[0];
            let end = segment[1];
            let speed = (end - start).len().to_f32();
            if speed <= 0.0 {
                // TODO: draw a circle or something here
                continue;
            }
            
            let t = speed / max_speed;
            let color = SLOW_COLOR.lerp_to_gamma(FAST_COLOR, t).gamma_multiply_u8(params.color().a());
            let gui_start = params.transform_point(start);
            let gui_end = params.transform_point(end);
            let mut stroke = params.stroke.clone();
            stroke.color = color;
            shapes.push(Shape::line_segment([gui_start, gui_end], stroke));
        }
        
        Shape::Vec(shapes)
    }
}