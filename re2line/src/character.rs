use egui::{Color32, Pos2, Shape, Stroke};
use epaint::{CircleShape, ColorMode, PathShape, PathStroke};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::aot::Item;
use crate::app::{DrawParams, GameObject, ObjectType};
use crate::collision::{CapsuleType, EllipseCollider, RectCollider};
use crate::math::{Fixed16, UFixed16, Fixed32, Vec2};
use crate::record::State;

mod ai;
pub use ai::*;

mod hit;
pub use hit::*;

const INTERACTION_DISTANCE: Fixed32 = Fixed32(620);

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

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum CharacterId {
    Leon = 0,
    Claire = 1,
    Unknown2 = 2,
    Unknown3 = 3,
    LeonBandaged = 4,
    ClaireBlackTop = 5,
    Unknown6 = 6,
    Unknown7 = 7,
    LeonTankTop = 8,
    ClaireBiker = 9,
    LeonSkullJacket = 10,
    Chris = 11,
    Hunk = 12,
    Tofu = 13,
    Ada = 14,
    Sherry = 15,
    ZombiePoliceHat = 16,
    Brad = 17,
    ZombieTornShirt = 18,
    Misty = 19,
    Unknown20 = 20,
    ZombieLabWhite = 21,
    ZombieLabYellow = 22,
    NakedZombie = 23,
    ZombieYellowShirt = 24,
    Unknown25 = 25,
    Unknown26 = 26,
    Unknown27 = 27,
    Unknown28 = 28,
    Unknown29 = 29,
    HeadlessZombieYellowShirt = 30,
    ZombieRandom = 31,
    Dog = 32,
    Crow = 33,
    LickerRed = 34,
    Croc = 35,
    LickerBlack = 36,
    Spider = 37,
    SpiderBaby = 38,
    GYoung = 39,
    GAdult = 40,
    Roach = 41,
    MrX = 42,
    SuperX = 43,
    Unknown44 = 44,
    Hands = 45,
    Ivy = 46,
    Tentacle = 47,
    G1 = 48,
    G2 = 49,
    Unknown50 = 50,
    G3 = 51,
    G4 = 52,
    Unknown53 = 53,
    G5 = 54,
    G5Tentacle = 55,
    Unknown56 = 56,
    PoisonIvy = 57,
    Moth = 58,
    Larva = 59,
    Unknown60 = 60,
    Unknown61 = 61,
    FuseArm = 62,
    FuseHousing = 63,
    Irons = 64,
    AdaNpc = 65,
    IronsTorso = 66,
    AdaWounded = 67,
    BenDead = 68,
    SherryNpc = 69,
    Ben = 70,
    Annette = 71,
    Kendo = 72,
    Unknown73 = 73,
    Marvin = 74,
    MayorsDaughter = 75,
    Unknown76 = 76,
    Unknown77 = 77,
    Unknown78 = 78,
    SherryVest = 79,
    LeonNpc = 80,
    ClaireNpc = 81,
    Unknown82 = 82,
    Unknown83 = 83,
    LeonBandagedNpc = 84,
    Unknown = 255,
}

impl CharacterId {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Leon => "Leon",
            Self::Claire => "Claire",
            Self::Unknown2 => "Unknown 2",
            Self::Unknown3 => "Unknown 3",
            Self::LeonBandaged => "Leon (bandaged)",
            Self::ClaireBlackTop => "Claire (black top)",
            Self::Unknown6 => "Unknown 6",
            Self::Unknown7 => "Unknown 7",
            Self::LeonTankTop => "Leon (tank top)",
            Self::ClaireBiker => "Claire (biker)",
            Self::LeonSkullJacket => "Leon (skull jacket)",
            Self::Chris => "Chris",
            Self::Hunk => "Hunk",
            Self::Tofu => "Tofu",
            Self::Ada => "Ada",
            Self::Sherry => "Sherry",
            Self::ZombiePoliceHat => "Zombie (police hat)",
            Self::Brad => "Brad",
            Self::ZombieTornShirt => "Zombie (torn shirt)",
            Self::Misty => "Misty",
            Self::Unknown20 => "Unknown 20",
            Self::ZombieLabWhite => "Zombie (lab, white)",
            Self::ZombieLabYellow => "Zombie (lab, yellow)",
            Self::NakedZombie => "Naked zombie",
            Self::ZombieYellowShirt => "Zombie (yellow shirt)",
            Self::Unknown25 => "Unknown 25",
            Self::Unknown26 => "Unknown 26",
            Self::Unknown27 => "Unknown 27",
            Self::Unknown28 => "Unknown 28",
            Self::Unknown29 => "Unknown 29",
            Self::HeadlessZombieYellowShirt => "Headless zombie (yellow shirt)",
            Self::ZombieRandom => "Zombie (random)",
            Self::Dog => "Dog",
            Self::Crow => "Crow",
            Self::LickerRed => "Licker (red)",
            Self::Croc => "Croc",
            Self::LickerBlack => "Licker (black)",
            Self::Spider => "Spider",
            Self::SpiderBaby => "Baby spider",
            Self::GYoung => "G Young",
            Self::GAdult => "G Adult",
            Self::Roach => "Roach",
            Self::MrX => "Mr. X",
            Self::SuperX => "Super X",
            Self::Unknown44 => "Unknown 44",
            Self::Hands => "Hands",
            Self::Ivy => "Ivy",
            Self::Tentacle => "Tentacle",
            Self::G1 => "G1",
            Self::G2 => "G2",
            Self::Unknown50 => "Unknown 50",
            Self::G3 => "G3",
            Self::G4 => "G4",
            Self::Unknown53 => "Unknown 53",
            Self::G5 => "G5",
            Self::G5Tentacle => "G5 Tentacle",
            Self::Unknown56 => "Unknown 56",
            Self::PoisonIvy => "Poison Ivy",
            Self::Moth => "Moth",
            Self::Larva => "Larva",
            Self::Unknown60 => "Unknown 60",
            Self::Unknown61 => "Unknown 61",
            Self::FuseArm => "Fuse Arm",
            Self::FuseHousing => "Fuse Housing",
            Self::Irons => "Irons",
            Self::AdaNpc => "Ada (NPC)",
            Self::IronsTorso => "Irons (torso)",
            Self::AdaWounded => "Ada (wounded)",
            Self::BenDead => "Ben (dead)",
            Self::SherryNpc => "Sherry (NPC)",
            Self::Ben => "Ben",
            Self::Annette => "Annette",
            Self::Kendo => "Kendo",
            Self::Unknown73 => "Unknown 73",
            Self::Marvin => "Marvin",
            Self::MayorsDaughter => "Mayor's daughter",
            Self::Unknown76 => "Unknown 76",
            Self::Unknown77 => "Unknown 77",
            Self::Unknown78 => "Unknown 78",
            Self::SherryVest => "Sherry (vest)",
            Self::LeonNpc => "Leon (NPC)",
            Self::ClaireNpc => "Claire (NPC)",
            Self::Unknown82 => "Unknown 82",
            Self::Unknown83 => "Unknown 83",
            Self::LeonBandagedNpc => "Leon (bandaged, NPC)",
            Self::Unknown => "Unknown",
        }
    }

    pub const fn type_(&self) -> CharacterType {
        match self {
            Self::Leon
            | Self::Claire
            | Self::Unknown2
            | Self::Unknown3
            | Self::LeonBandaged
            | Self::ClaireBlackTop
            | Self::Unknown6
            | Self::Unknown7
            | Self::LeonTankTop
            | Self::ClaireBiker
            | Self::LeonSkullJacket
            | Self::Chris
            | Self::Hunk
            | Self::Tofu
            | Self::Ada
            | Self::Sherry
            => CharacterType::Player,
            Self::AdaNpc | Self::SherryNpc => CharacterType::Ally,
            Self::FuseArm
            | Self::FuseHousing
            | Self::Irons
            | Self::IronsTorso
            | Self::AdaWounded
            | Self::BenDead
            | Self::Ben
            | Self::Annette
            | Self::Kendo
            | Self::Marvin
            | Self::MayorsDaughter
            | Self::LeonNpc
            | Self::ClaireNpc
            | Self::LeonBandagedNpc
            => CharacterType::Neutral,
            _ => CharacterType::Enemy,
        }
    }

    pub const fn is_player(&self) -> bool {
        matches!(self.type_(), CharacterType::Player)
    }

    pub const fn is_zombie(&self) -> bool {
        matches!(self,
            Self::ZombiePoliceHat
            | Self::ZombieTornShirt
            | Self::ZombieYellowShirt
            | Self::ZombieRandom
            | Self::ZombieLabWhite
            | Self::ZombieLabYellow
            | Self::Misty
            | Self::Unknown20
            | Self::Unknown25
            | Self::Unknown26
            | Self::Unknown27
            | Self::Unknown28
            | Self::Unknown29
            | Self::Brad
            | Self::NakedZombie
            | Self::HeadlessZombieYellowShirt
        )
    }

    pub const fn is_licker(&self) -> bool {
        matches!(self, Self::LickerRed | Self::LickerBlack)
    }
}

#[derive(Debug, Clone)]
pub struct Character {
    pub id: CharacterId,
    pub center: Vec2,
    pub prev_center: Vec2,
    pub size: Vec2,
    pub shape: EllipseCollider,
    pub outline_shape: RectCollider,
    pub angle: Fixed32,
    current_health: i16,
    max_health: i16,
    pub state: [u8; 4],
    pub floor: u8,
    pub velocity: Vec2,
    pub type_: u8,
    pub index: usize,
}

impl Character {
    pub const fn new(id: CharacterId, health: i16, x: Fixed16, z: Fixed16, width: UFixed16, height: UFixed16, angle: Fixed16, velocity: Vec2) -> Self {
        let game_x = Fixed32(x.0 as i32 - width.0 as i32);
        let game_z = Fixed32(z.0 as i32 - height.0 as i32);
        let game_width = UFixed16(width.0 << 1).to_32();
        let game_height = UFixed16(height.0 << 1).to_32();
        
        let center = Vec2 { x: Fixed32(x.0 as i32), z: Fixed32(z.0 as i32) };

        Self {
            id,
            center,
            prev_center: center,
            size: Vec2 { x: Fixed32(width.0 as i32), z: Fixed32(height.0 as i32) },
            shape: EllipseCollider::new(game_x, game_z, game_width, game_height),
            outline_shape: RectCollider::new(game_x, game_z, game_width, game_height, CapsuleType::None),
            angle: angle.to_32(),
            current_health: health,
            max_health: health,
            state: [0; 4],
            floor: 0,
            velocity,
            type_: 0,
            index: usize::MAX,
        }
    }

    pub const fn empty(id: CharacterId) -> Self {
        Self::new(id, 0, Fixed16(0), Fixed16(0), UFixed16(0), UFixed16(0), Fixed16(0), Vec2::zero())
    }

    pub const fn name(&self) -> &'static str {
        self.id.name()
    }

    pub const fn type_(&self) -> CharacterType {
        self.id.type_()
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
        } else {
            "Unknown"
        })
    }

    pub fn ai_zones(&self) -> Vec<PositionedAiZone> {
        let ai_zones = match self.id {
            CharacterId::LickerRed => &RED_LICKER_AI_ZONES[..],
            CharacterId::LickerBlack => &BLACK_LICKER_AI_ZONES[..],
            _ if self.is_crawling_zombie() => &CRAWLING_ZOMBIE_AI_ZONES[..],
            _ if self.id.is_zombie() => &ZOMBIE_AI_ZONES[..],
            _ => return Vec::new(),
        };
        
        let mut positioned_ai_zones = Vec::new();
        for ai_zone in ai_zones {
            if !ai_zone.check_state(&self.state) {
                // zone is not active in this state; skip it
                continue;
            }

            positioned_ai_zones.push(PositionedAiZone::new(ai_zone, self.id, self.index, self.center, self.angle));
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
        self.id.type_().into()
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
}

impl CharacterPath {
    pub fn new(points: Vec<Vec2>, character_id: CharacterId, character_index: usize) -> Self {
        Self { points, character_id, character_index }
    }
    
    pub fn len(&self) -> Fixed32 {
        self.points.iter().fold(Fixed32(0), |acc, p| acc + p.len())
    }
    
    pub fn max_speed(&self) -> Fixed32 {
        self.points.windows(2).fold(Fixed32(0), |acc, p| acc.max((p[1] - p[0]).len()))
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