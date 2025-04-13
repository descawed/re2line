use egui::{Align, Color32, Pos2, Shape, Stroke, TextStyle, Ui};
use epaint::{ColorMode, PathShape, PathStroke, TextShape};
use epaint::text::LayoutJob;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::collision::{DrawParams, EllipseCollider};
use crate::math::{Fixed12, UFixed12, Vec2};

const ARROW_HEAD_HEIGHT: f32 = 6.0;
const ARROW_HEAD_WIDTH: f32 = 6.0;
const ARROW_SHAFT_WIDTH: f32 = 1.5;
const LABEL_CORNER_RADIUS: f32 = 5.0;
const LABEL_MARGIN: f32 = 10.0;
const LABEL_PADDING: f32 = 5.0;
const LABEL_WRAP_WIDTH: f32 = 150.0;
const MOTION_PROJECTION_LENGTH: f32 = 25.0;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum CharacterType {
    Player,
    Ally,
    Neutral,
    Enemy,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, IntoPrimitive, TryFromPrimitive)]
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
    Unknown62 = 62,
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
            CharacterId::Leon => "Leon",
            CharacterId::Claire => "Claire",
            CharacterId::Unknown2 => "Unknown 2",
            CharacterId::Unknown3 => "Unknown 3",
            CharacterId::LeonBandaged => "Leon (bandaged)",
            CharacterId::ClaireBlackTop => "Claire (black top)",
            CharacterId::Unknown6 => "Unknown 6",
            CharacterId::Unknown7 => "Unknown 7",
            CharacterId::LeonTankTop => "Leon (tank top)",
            CharacterId::ClaireBiker => "Claire (biker)",
            CharacterId::LeonSkullJacket => "Leon (skull jacket)",
            CharacterId::Chris => "Chris",
            CharacterId::Hunk => "Hunk",
            CharacterId::Tofu => "Tofu",
            CharacterId::Ada => "Ada",
            CharacterId::Sherry => "Sherry",
            CharacterId::ZombiePoliceHat => "Zombie (police hat)",
            CharacterId::Brad => "Brad",
            CharacterId::ZombieTornShirt => "Zombie (torn shirt)",
            CharacterId::Misty => "Misty",
            CharacterId::Unknown20 => "Unknown 20",
            CharacterId::ZombieLabWhite => "Zombie (lab, white)",
            CharacterId::ZombieLabYellow => "Zombie (lab, yellow)",
            CharacterId::NakedZombie => "Naked zombie",
            CharacterId::ZombieYellowShirt => "Zombie (yellow shirt)",
            CharacterId::Unknown25 => "Unknown 25",
            CharacterId::Unknown26 => "Unknown 26",
            CharacterId::Unknown27 => "Unknown 27",
            CharacterId::Unknown28 => "Unknown 28",
            CharacterId::Unknown29 => "Unknown 29",
            CharacterId::HeadlessZombieYellowShirt => "Headless zombie (yellow shirt)",
            CharacterId::ZombieRandom => "Zombie (random)",
            CharacterId::Dog => "Dog",
            CharacterId::Crow => "Crow",
            CharacterId::LickerRed => "Licker (red)",
            CharacterId::Croc => "Croc",
            CharacterId::LickerBlack => "Licker (black)",
            CharacterId::Spider => "Spider",
            CharacterId::SpiderBaby => "Baby spider",
            CharacterId::GYoung => "G Young",
            CharacterId::GAdult => "G Adult",
            CharacterId::Roach => "Roach",
            CharacterId::MrX => "Mr. X",
            CharacterId::SuperX => "Super X",
            CharacterId::Unknown44 => "Unknown 44",
            CharacterId::Hands => "Hands",
            CharacterId::Ivy => "Ivy",
            CharacterId::Tentacle => "Tentacle",
            CharacterId::G1 => "G1",
            CharacterId::G2 => "G2",
            CharacterId::Unknown50 => "Unknown 50",
            CharacterId::G3 => "G3",
            CharacterId::G4 => "G4",
            CharacterId::Unknown53 => "Unknown 53",
            CharacterId::G5 => "G5",
            CharacterId::G5Tentacle => "G5 Tentacle",
            CharacterId::Unknown56 => "Unknown 56",
            CharacterId::PoisonIvy => "Poison Ivy",
            CharacterId::Moth => "Moth",
            CharacterId::Larva => "Larva",
            CharacterId::Unknown60 => "Unknown 60",
            CharacterId::Unknown61 => "Unknown 61",
            CharacterId::Unknown62 => "Unknown 62",
            CharacterId::FuseHousing => "Fuse Housing",
            CharacterId::Irons => "Irons",
            CharacterId::AdaNpc => "Ada (NPC)",
            CharacterId::IronsTorso => "Irons (torso)",
            CharacterId::AdaWounded => "Ada (wounded)",
            CharacterId::BenDead => "Ben (dead)",
            CharacterId::SherryNpc => "Sherry (NPC)",
            CharacterId::Ben => "Ben",
            CharacterId::Annette => "Annette",
            CharacterId::Kendo => "Kendo",
            CharacterId::Unknown73 => "Unknown 73",
            CharacterId::Marvin => "Marvin",
            CharacterId::MayorsDaughter => "Mayor's daughter",
            CharacterId::Unknown76 => "Unknown 76",
            CharacterId::Unknown77 => "Unknown 77",
            CharacterId::Unknown78 => "Unknown 78",
            CharacterId::SherryVest => "Sherry (vest)",
            CharacterId::LeonNpc => "Leon (NPC)",
            CharacterId::ClaireNpc => "Claire (NPC)",
            CharacterId::Unknown82 => "Unknown 82",
            CharacterId::Unknown83 => "Unknown 83",
            CharacterId::LeonBandagedNpc => "Leon (bandaged, NPC)",
            CharacterId::Unknown => "Unknown",
        }
    }

    pub const fn type_(&self) -> CharacterType {
        match self {
            CharacterId::Leon
            | CharacterId::Claire
            | CharacterId::Unknown2
            | CharacterId::Unknown3
            | CharacterId::LeonBandaged
            | CharacterId::ClaireBlackTop
            | CharacterId::Unknown6
            | CharacterId::Unknown7
            | CharacterId::LeonTankTop
            | CharacterId::ClaireBiker
            | CharacterId::LeonSkullJacket
            | CharacterId::Chris
            | CharacterId::Hunk
            | CharacterId::Tofu
            | CharacterId::Ada
            | CharacterId::Sherry
            => CharacterType::Player,
            CharacterId::AdaNpc | CharacterId::SherryNpc => CharacterType::Ally,
            CharacterId::FuseHousing
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
            _ => CharacterType::Enemy,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Character {
    pub id: CharacterId,
    pub shape: EllipseCollider,
    pub angle: Fixed12,
    current_health: i16,
    max_health: i16,
    pub state: [u8; 4],
    pub floor: u8,
    pub velocity: Vec2,
}

impl Character {
    pub const fn new(id: CharacterId, health: i16, x: Fixed12, z: Fixed12, width: UFixed12, height: UFixed12, angle: Fixed12, velocity: Vec2) -> Self {
        Self {
            id,
            shape: EllipseCollider::new(x, z, width, height),
            angle,
            current_health: health,
            max_health: health,
            state: [0; 4],
            floor: 0,
            velocity,
        }
    }

    pub const fn empty(id: CharacterId) -> Self {
        Self::new(id, 0, Fixed12(0), Fixed12(0), UFixed12(0), UFixed12(0), Fixed12(0), Vec2::zero())
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
        self.shape.set_pos(x.into(), z.into());
    }

    pub fn set_size(&mut self, width: impl Into<UFixed12>, height:  impl Into<UFixed12>) {
        self.shape.set_size(width.into(), height.into());
    }

    pub fn label(&self) -> String {
        let (x, z) = self.shape.pos();
        format!(
            "{}\nState: {:02X} {:02X} {:02X} {:02X}\nX: {:7} Z:{:7}\nHP: {}/{}",
            self.id.name(),
            self.state[0], self.state[1], self.state[2], self.state[3],
            x, z,
            self.current_health, self.max_health,
        )
    }

    pub fn gui_shape(&self, draw_params: &DrawParams, ui: &Ui) -> Shape {
        let body_shape = self.shape.gui_shape(draw_params);
        let body_rect = body_shape.visual_bounding_rect();
        let body_center = body_rect.center();

        let center_x = body_center.x;
        let top_y = body_rect.min.y;
        let font_id = TextStyle::Body.resolve(&*ui.style());

        let bg_color = Color32::from_rgb(0x30, 0x30, 0x30);

        let text_shape = ui.fonts(|fonts| {
            // TODO: make colors configurable
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

        let vector = egui::Vec2::angled(self.angle.to_radians()) * MOTION_PROJECTION_LENGTH;
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

        Shape::Vec(vec![text_bg_shape, text_shape, body_shape, shaft_shape, arrow_shape])
    }
}