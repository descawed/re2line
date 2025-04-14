use egui::{Align, Color32, Pos2, Shape, Stroke, TextStyle, Ui};
use epaint::{ColorMode, PathShape, PathStroke, TextShape};
use epaint::text::LayoutJob;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::collision::{DrawParams, EllipseCollider};
use crate::math::{Fixed12, UFixed12, Vec2};

mod ai;
use ai::*;

const ARROW_HEAD_HEIGHT: f32 = 6.0;
const ARROW_HEAD_WIDTH: f32 = 6.0;
const ARROW_SHAFT_WIDTH: f32 = 1.5;
const LABEL_CORNER_RADIUS: f32 = 5.0;
const LABEL_MARGIN: f32 = 10.0;
const LABEL_PADDING: f32 = 5.0;
const LABEL_WRAP_WIDTH: f32 = 150.0;
const MOTION_PROJECTION_LENGTH: f32 = 0.25;

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
        // TODO: do Brad, Marvin, and naked zombies use the same AI as regular zombies?
        matches!(self,
            Self::ZombiePoliceHat
            | Self::ZombieTornShirt
            | Self::ZombieYellowShirt
            | Self::ZombieRandom
            | Self::ZombieLabWhite
            | Self::ZombieLabYellow
            | Self::Misty
        )
    }
}

#[derive(Debug, Clone)]
pub struct Character {
    pub id: CharacterId,
    pub center: Vec2,
    pub width: UFixed12,
    pub height: UFixed12,
    pub shape: EllipseCollider,
    pub angle: Fixed12,
    current_health: i16,
    max_health: i16,
    pub state: [u8; 4],
    pub floor: u8,
    pub velocity: Vec2,
    pub type_: u8,
}

impl Character {
    pub const fn new(id: CharacterId, health: i16, x: Fixed12, z: Fixed12, width: UFixed12, height: UFixed12, angle: Fixed12, velocity: Vec2) -> Self {
        Self {
            id,
            center: Vec2 { x, z },
            width,
            height,
            shape: EllipseCollider::new(
                Fixed12((x.0 as i32 - width.0 as i32) as i16), Fixed12((z.0 as i32 - height.0 as i32) as i16),
                UFixed12(width.0 << 1), UFixed12(height.0 << 1),
            ),
            angle,
            current_health: health,
            max_health: health,
            state: [0; 4],
            floor: 0,
            velocity,
            type_: 0,
        }
    }

    pub const fn empty(id: CharacterId) -> Self {
        Self::new(id, 0, Fixed12(0), Fixed12(0), UFixed12(0), UFixed12(0), Fixed12(0), Vec2::zero())
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

    pub fn set_pos(&mut self, x: impl Into<Fixed12>, z: impl Into<Fixed12>) {
        self.center = Vec2::new(x.into(), z.into());
        self.shape.set_pos(self.center.x - self.width, self.center.z - self.height);
    }

    pub fn set_size(&mut self, width: impl Into<UFixed12>, height:  impl Into<UFixed12>) {
        self.width = width.into();
        self.height = height.into();
        self.shape.set_size(self.width << 1, self.height << 1);
    }

    pub fn label(&self) -> String {
        let (x, z) = self.shape.pos();
        format!(
            "{}\nState: {:02X} {:02X} {:02X} {:02X}\nX: {:7} Z: {:7}\nHP: {}/{}",
            self.id.name(),
            self.state[0], self.state[1], self.state[2], self.state[3],
            x, z,
            self.current_health, self.max_health,
        )
    }

    fn is_crawling_zombie(&self) -> bool {
        self.id.is_zombie() && matches!(self.type_ & 0x3f, 1 | 3 | 5 | 7 | 9 | 11 | 13)
    }

    fn describe_state(&self) -> String {
        String::from(if self.is_crawling_zombie() {
            describe_crawling_zombie_ai_state(&self.state)
        } else if self.id.is_zombie() {
            describe_zombie_ai_state(&self.state)
        } else if self.id.is_player() {
            describe_player_ai_state(&self.state)
        } else {
            "Unknown"
        })
    }

    pub fn describe(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = Vec::new();

        groups.push((String::from("Character"), vec![
            format!("Type: {} ({})", self.name(), self.id as u8),
            format!("Sub-type: {}", self.type_ & 0x3f),
            format!("HP: {}/{}", self.current_health, self.max_health),
        ]));

        groups.push((String::from("Position"), vec![
            format!("X: {}", self.center.x),
            format!("Z: {}", self.center.z),
            format!("Angle: {:.1}°", self.angle.to_degrees()),
            format!("Floor: {}", self.floor),
            format!("XR: {}", self.width),
            format!("ZR: {}", self.height),
        ]));

        groups.push((String::from("State"), vec![
            format!("{:02X} {:02X} {:02X} {:02X}", self.state[0], self.state[1], self.state[2], self.state[3]),
            self.describe_state(),
        ]));

        groups
    }

    pub fn gui_ai(&self, draw_params: &DrawParams, player_pos: Option<Vec2>) -> Shape {
        let mut shapes = Vec::new();

        let ai_cones = if self.is_crawling_zombie() {
            &CRAWLING_ZOMBIE_AI_CONES[..]
        } else if self.id.is_zombie() {
            &ZOMBIE_AI_CONES[..]
        } else if matches!(self.id, CharacterId::LickerRed) {
            &LICKER_AI_CONES[..]
        } else {
            return Shape::Vec(shapes);
        };

        let body_shape = self.shape.gui_shape(draw_params);
        let body_center = body_shape.visual_bounding_rect().center();

        for ai_cone in ai_cones {
            if !ai_cone.check_state(&self.state) {
                // cone is not active in this state; skip it
                continue;
            }

            let mut draw_params = draw_params.clone();
            draw_params.origin = body_center;
            draw_params.fill_color = ai_cone.behavior_type.default_color();

            let facing_angle = self.angle.to_radians();
            if let Some(player_pos) = player_pos {
                if ai_cone.is_point_in_cone(player_pos.saturating_sub(self.center), facing_angle) {
                    // add an outline to the shape when the player is inside
                    draw_params.stroke.width = 3.0;
                    draw_params.stroke.color = Color32::from_rgb(0x42, 0x03, 0x03);
                }
            }

            shapes.push(ai_cone.gui_shape(facing_angle, draw_params));
        }

        Shape::Vec(shapes)
    }

    pub fn gui_shape(&self, draw_params: &DrawParams, ui: &Ui, show_tooltip: bool) -> Shape {
        let body_shape = self.shape.gui_shape(draw_params);
        let body_rect = body_shape.visual_bounding_rect();
        let body_center = body_rect.center();

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

        if !show_tooltip {
            return Shape::Vec(vec![body_shape, shaft_shape, arrow_shape]);
        }

        let center_x = body_center.x;
        let top_y = body_rect.min.y;
        let font_id = TextStyle::Body.resolve(&*ui.style());
        // TODO: make colors configurable
        let bg_color = Color32::from_rgb(0x30, 0x30, 0x30);

        let text_shape = ui.fonts(|fonts| {
            let mut job = LayoutJob::simple(
                self.label(),
                font_id,
                Color32::from_rgb(0xe0, 0xe0, 0xe0),
                LABEL_WRAP_WIDTH,
            );
            job.halign = Align::Center;

            let galley = fonts.layout_job(job);

            Shape::Text(TextShape::new(
                Pos2::new(center_x, top_y - galley.rect.height() - LABEL_MARGIN),
                galley,
                bg_color,
            ))
        });

        let bg_rect = text_shape.visual_bounding_rect().expand(LABEL_PADDING);
        let text_bg_shape = Shape::rect_filled(bg_rect, LABEL_CORNER_RADIUS, bg_color);

        Shape::Vec(vec![body_shape, shaft_shape, arrow_shape, text_bg_shape, text_shape])
    }
}