use std::fmt::{Display, Formatter};

use eframe::emath::Pos2;
use egui::Color32;
use enum_map::Enum;
use residat::common::{Fixed32, Vec2};
use residat::re2::SceType;
use serde::{Deserialize, Serialize};

use crate::character::{BehaviorType, CharacterType};
use crate::draw::{VAlign, text_box};
use crate::record::State;

const FLOOR_HEIGHT: Fixed32 = Fixed32(-1800);

#[derive(Debug, Clone, Copy)]
pub enum Floor {
    Mask(u32),
    Id(u8),
    Aot(u8),
}

impl Floor {
    pub const ANY: Self = Self::Aot(0x80);

    pub const fn matches_any(&self) -> bool {
        if let Self::Aot(floor) = self {
            *floor & 0x80 != 0
        } else {
            false
        }
    }

    pub const fn mask(&self) -> u32 {
        match self {
            Self::Mask(mask) => *mask,
            Self::Aot(_) if self.matches_any() => 0xFFFFFFFF,
            Self::Id(floor) | Self::Aot(floor) => 1 << (*floor & 0x1f),
        }
    }

    pub const fn matches(&self, other: Self) -> bool {
        self.mask() & other.mask() != 0
    }

    pub const fn y(&self) -> Option<Fixed32> {
        match self {
            Self::Id(floor) | Self::Aot(floor) if !self.matches_any() => {
                Some(Fixed32(*floor as i32 * FLOOR_HEIGHT.0))
            }
            _ => None,
        }
    }
}

impl Display for Floor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(floor) => write!(f, "{}", floor)?,
            Self::Aot(floor) => if self.matches_any() {
                write!(f, "Any")
            } else {
                write!(f, "{}", floor)
            }?,
            Self::Mask(mask) => {
                let mut wrote = false;
                for i in 0..32 {
                    if mask & (1 << i) != 0 {
                        if wrote {
                            write!(f, ", ")?;
                        } else {
                            wrote = true;
                        }

                        write!(f, "{}", i)?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorldPos {
    pub pos: Vec2,
    pub size: Vec2,
    pub floor: Floor,
    pub collision_mask: u16,
    pub collision_deny_mask: u16,
    pub quadrant_mask: Option<u16>,
}

impl WorldPos {
    pub const fn new(pos: Vec2, size: Vec2, floor: Floor, collision_mask: u16, collision_deny_mask: u16) -> Self {
        Self {
            pos,
            size,
            floor,
            collision_mask,
            collision_deny_mask,
            quadrant_mask: None,
        }
    }

    pub const fn point(pos: Vec2, floor: Floor) -> Self {
        Self {
            pos,
            size: Vec2::zero(),
            floor,
            collision_mask: 0xffff,
            collision_deny_mask: 0,
            quadrant_mask: None,
        }
    }

    pub const fn rect(pos: Vec2, size: Vec2, floor: Floor) -> Self {
        Self {
            pos,
            size,
            floor,
            collision_mask: 0xffff,
            collision_deny_mask: 0,
            quadrant_mask: None,
        }
    }

    pub fn with_quadrant_mask(mut self, quadrant_mask: u16) -> Self {
        self.quadrant_mask = Some(quadrant_mask);
        self
    }

    pub const fn can_collide_with(&self, other: &Self) -> bool {
        self.floor.matches(other.floor)
            && self.collision_mask & other.collision_mask != 0
            && self.collision_deny_mask & other.collision_mask == 0
            && self.collision_mask & other.collision_deny_mask == 0
            && if let (Some(self_mask), Some(other_mask)) = (self.quadrant_mask, other.quadrant_mask) {
            self_mask & other_mask != 0
        } else {
            true
        }
    }

    pub fn set_quadrant_mask(&mut self, cell_center: Vec2) {
        let rel = self.pos - cell_center;
        self.collision_mask |= (1 << (rel.x.0 as u32 >> 0x1f)) << ((rel.z.0 as u32 >> 0x1e) & 2);
    }
}

///
#[derive(Debug, Enum, PartialEq, Eq, Hash, Clone, Copy, Deserialize, Serialize)]
pub enum ObjectType {
    Floor,
    Collider,
    // AOTs
    Auto,
    Door,
    Item,
    Normal,
    Message,
    Event,
    FlagChg,
    Water,
    Move,
    Save,
    ItemBox,
    Damage,
    Status,
    Hikidashi,
    Windows,
    // characters
    Object,
    Enemy,
    Player,
    Ally,
    Neutral,
    // AI zones
    AiHitZone,
    AiAttackZone,
    AiAggroZone,
    AiTacticZone,
    WeaponRange,
    // GUI objects
    CharacterPath,
}

impl ObjectType {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Floor => "Floor",
            Self::Collider => "Collider",
            Self::Auto => "Auto AOT",
            Self::Door => "Door",
            Self::Item => "Item",
            Self::Normal => "Normal AOT",
            Self::Message => "Message",
            Self::Event => "Event",
            Self::FlagChg => "Flag Change",
            Self::Water => "Water",
            Self::Move => "Move AOT",
            Self::Save => "Typewriter",
            Self::ItemBox => "Item Box",
            Self::Damage => "Damage AOT",
            Self::Status => "Status AOT",
            Self::Hikidashi => "Hikidashi AOT",
            Self::Windows => "Windows",
            Self::Object => "Object",
            Self::Enemy => "Enemy",
            Self::Player => "Player",
            Self::Ally => "NPC Ally",
            Self::Neutral => "NPC",
            Self::AiHitZone => "AI Hit Zone",
            Self::AiAttackZone => "AI Attack Zone",
            Self::AiAggroZone => "AI Aggro Zone",
            Self::AiTacticZone => "AI Tactic Zone",
            Self::WeaponRange => "Weapon Range",
            Self::CharacterPath => "Character Path",
        }
    }

    pub const fn is_ai_zone(&self) -> bool {
        matches!(self, Self::AiAggroZone | Self::AiAttackZone | Self::AiHitZone | Self::AiTacticZone)
    }

    pub const fn is_aot(&self) -> bool {
        matches!(
            self,
            Self::Auto | Self::Door | Self::Item | Self::Normal | Self::Message | Self::Event | Self::FlagChg | Self::Water | Self::Move | Self::Save | Self::ItemBox | Self::Damage | Self::Status | Self::Hikidashi | Self::Windows,
        )
    }

    pub const fn is_character(&self) -> bool {
        matches!(self, Self::Enemy | Self::Player | Self::Ally | Self::Neutral)
    }

    pub const fn is_collider(&self) -> bool {
        matches!(self, Self::Collider)
    }

    pub const fn is_floor(&self) -> bool {
        matches!(self, Self::Floor)
    }
}

impl From<SceType> for ObjectType {
    fn from(value: SceType) -> Self {
        match value {
            SceType::Auto | SceType::Unknown => Self::Auto,
            SceType::Door => Self::Door,
            SceType::Item => Self::Item,
            SceType::Normal => Self::Normal,
            SceType::Message => Self::Message,
            SceType::Event => Self::Event,
            SceType::FlagChg => Self::Event,
            SceType::Water => Self::Water,
            SceType::Move => Self::Move,
            SceType::Save => Self::Save,
            SceType::ItemBox => Self::ItemBox,
            SceType::Damage => Self::Damage,
            SceType::Status => Self::Status,
            SceType::Hikidashi => Self::Hikidashi,
            SceType::Windows => Self::Windows,
        }
    }
}

impl From<CharacterType> for ObjectType {
    fn from(value: CharacterType) -> Self {
        match value {
            CharacterType::Player => Self::Player,
            CharacterType::Ally => Self::Ally,
            CharacterType::Neutral => Self::Neutral,
            CharacterType::Enemy => Self::Enemy,
        }
    }
}

impl From<BehaviorType> for ObjectType {
    fn from(value: BehaviorType) -> Self {
        match value {
            BehaviorType::Hit => Self::AiHitZone,
            BehaviorType::Attack => Self::AiAttackZone,
            BehaviorType::Aggro => Self::AiAggroZone,
            BehaviorType::ChangeTactic => Self::AiTacticZone,
        }
    }
}

///
const HIGHLIGHT_MAX_INTENSITY: f32 = 0.5;
const HIGHLIGHT: egui::Rgba = egui::Rgba::from_rgba_premultiplied(0.25, 0.25, 0.25, 0.0);
const HIGHLIGHT_STROKE: f32 = 2.0;
const HIGHLIGHT_ALPHA: f32 = 1.5;

#[derive(Debug, Clone)]
pub struct DrawParams {
    pub origin: Pos2,
    pub scale: f32,
    pub fill_color: Color32,
    pub stroke: egui::Stroke,
    pub stroke_kind: egui::StrokeKind,
    pub draw_at_origin: bool,
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

    pub fn transform_point(&self, point: Vec2) -> Pos2 {
        let (x, y, _, _) = self.transform(point.x, point.z, 0, 0);
        Pos2::new(x, y)
    }

    pub const fn is_stroke(&self) -> bool {
        self.stroke.width > 0.0 && self.stroke.color.a() > 0
    }

    pub const fn color(&self) -> Color32 {
        if self.is_stroke() {
            self.stroke.color
        } else {
            self.fill_color
        }
    }

    pub const fn set_color(&mut self, color: Color32) {
        if self.is_stroke() {
            self.stroke.color = color;
        } else {
            self.fill_color = color;
        }
    }

    pub fn fade(&mut self, factor: f32) {
        self.set_color(self.color().gamma_multiply(factor))
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

        self.stroke.color = Color32::BLACK;
        self.stroke.width = HIGHLIGHT_STROKE;
    }

    pub fn set_draw_origin(&mut self, origin: Pos2) {
        self.origin = origin;
        self.draw_at_origin = true;
    }
}

///
const LABEL_MARGIN: f32 = 10.0;

pub trait GameObject {
    fn object_type(&self) -> ObjectType;

    fn contains_point(&self, point: Vec2) -> bool;

    fn name(&self) -> String;

    fn name_prefix(&self, index: usize) -> String {
        format!("#{index}")
    }

    fn description(&self) -> String;

    fn details(&self) -> Vec<(String, Vec<String>)>;

    fn floor(&self) -> Floor;

    fn collision_mask(&self) -> u16 {
        0xFFFF
    }

    fn gui_shape(&self, params: &DrawParams, state: &State) -> egui::Shape;

    fn gui_tooltip(&self, params: &DrawParams, state: &State, ui: &egui::Ui, name_prefix: &str) -> egui::Shape {
        let name = format!("{} {}", name_prefix, self.name());

        let (x, y) = if params.draw_at_origin {
            (params.origin.x, params.origin.y)
        } else {
            let body_shape = self.gui_shape(params, state);
            let body_rect = body_shape.visual_bounding_rect();
            let body_center = body_rect.center();

            (body_center.x, body_rect.min.y)
        };

        let text = format!("{}\n{}", name, self.description());

        let (text_bg_shape, text_shape) = text_box(
            text,
            Pos2::new(x, y - LABEL_MARGIN),
            VAlign::Bottom,
            Color32::from_rgb(0x30, 0x30, 0x30),
            Color32::from_rgb(0xe0, 0xe0, 0xe0),
            ui,
        );

        egui::Shape::Vec(vec![text_bg_shape, text_shape])
    }
}