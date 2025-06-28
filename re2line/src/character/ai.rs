use std::f32::consts::PI;

use egui::{Color32, Shape, Stroke};
use epaint::{CircleShape, ColorMode, PathShape, PathStroke};
use residat::common::*;
use residat::re2::CharacterId;

use crate::app::{DrawParams, Floor, GameObject, ObjectType};
use crate::draw::*;
use crate::record::State;

#[derive(Debug, Clone)]
pub enum ZoneOrigin {
    Base,
    ModelPart(usize),
}

#[derive(Debug, Clone)]
pub enum StateMask {
    Any,
    Exactly(u8),
    Either(u8, u8),
    OneOf3(u8, u8, u8),
    Between(u8, u8),
}

impl StateMask {
    pub fn matches(&self, state: u8) -> bool {
        match self {
            Self::Any => true,
            Self::Exactly(value) => state == *value,
            Self::Either(value1, value2) => state == *value1 || state == *value2,
            Self::OneOf3(value1, value2, value3) => state == *value1 || state == *value2 || state == *value3,
            Self::Between(value1, value2) => state >= *value1 && state <= *value2,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BehaviorType {
    Aggro,
    Attack,
    ChangeTactic,
    Hit,
}

impl BehaviorType {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Aggro => "Aggro",
            Self::Attack => "Attack",
            Self::ChangeTactic => "Change tactic",
            Self::Hit => "Hit",
        }
    }
}

#[derive(Debug)]
pub struct AiZone {
    pub name: &'static str,
    pub description: &'static str,
    pub behavior_type: BehaviorType,
    pub half_angle: Fixed16,
    pub offset_angle: Fixed16,
    pub radius: UFixed16,
    pub inverted: bool,
    pub state_mask: [StateMask; 4],
    pub type_mask: StateMask,
    pub origin: ZoneOrigin,
}

impl AiZone {
    pub const fn new(name: &'static str, description: &'static str, behavior_type: BehaviorType, half_angle: Fixed16, offset_angle: Fixed16, radius: UFixed16, inverted: bool, state_mask: [StateMask; 4]) -> Self {
        Self {
            name,
            description,
            behavior_type,
            half_angle,
            offset_angle,
            radius,
            inverted,
            state_mask,
            type_mask: StateMask::Any,
            origin: ZoneOrigin::Base,
        }
    }

    pub const fn arc(name: &'static str, description: &'static str, behavior_type: BehaviorType, half_angle: Fixed16, radius: UFixed16, state_mask: [StateMask; 4]) -> Self {
        Self {
            name,
            description,
            behavior_type,
            half_angle,
            offset_angle: Fixed16(0),
            radius,
            inverted: false,
            state_mask,
            type_mask: StateMask::Any,
            origin: ZoneOrigin::Base,
        }
    }

    pub const fn circle(name: &'static str, description: &'static str, behavior_type: BehaviorType, radius: UFixed16, state_mask: [StateMask; 4]) -> Self {
        Self {
            name,
            description,
            behavior_type,
            half_angle: Fixed16(0x800),
            offset_angle: Fixed16(0),
            radius,
            inverted: false,
            state_mask,
            type_mask: StateMask::Any,
            origin: ZoneOrigin::Base,
        }
    }

    pub const fn with_type_mask(mut self, type_mask: StateMask) -> Self {
        self.type_mask = type_mask;
        self
    }

    pub const fn with_origin(mut self, origin: ZoneOrigin) -> Self {
        self.origin = origin;
        self
    }

    pub const fn inverted(mut self) -> Self {
        self.inverted = true;
        self
    }

    pub fn gui_shape(&self, angle: Fixed32, pos: Vec2, mut draw_params: DrawParams, state: &State) -> Shape {
        let facing_angle = angle.to_radians();
        
        let (gui_x, gui_y, _, _) = draw_params.transform(pos.x, pos.z, 0, 0);
        let gui_pos = egui::Pos2::new(gui_x, gui_y);
        
        // if the player is in this zone, draw it with an outline
        if let Some(ref player) = state.characters()[0] {
            if self.is_point_in_zone(player.center().saturating_sub(pos), angle) {
                // add an outline to the shape when the player is inside
                draw_params.stroke.width = 3.0;
                draw_params.stroke.color = Color32::from_rgb(0x42, 0x03, 0x03);
            }
        }
        
        let radians = self.half_angle.to_radians();
        let radius = self.radius.to_f32() * draw_params.scale;
        if radians.abs() >= PI {
            // just use a circle
            // for an inverted circle, we treat the outside of the circle as being in the zone, and
            // we just draw an outline rather than doing a fill out to the edges of the map
            return Shape::Circle(if self.inverted {
                CircleShape {
                    center: gui_pos,
                    radius,
                    fill: Color32::TRANSPARENT,
                    stroke: Stroke {
                        width: 2.0,
                        color: draw_params.fill_color,
                    },
                }
            } else {
                CircleShape {
                    center: gui_pos,
                    radius,
                    fill: draw_params.fill_color,
                    stroke: draw_params.stroke.clone(),
                }
            });
        }

        let offset = self.offset_angle.to_radians();
        let points = get_path_for_semicircle(gui_pos, radius, facing_angle + offset, radians, self.inverted);
        Shape::Path(PathShape {
            points,
            closed: true,
            fill: draw_params.fill_color,
            stroke: PathStroke {
                width: draw_params.stroke.width,
                color: ColorMode::Solid(draw_params.stroke.color),
                kind: draw_params.stroke_kind,
            },
        })
    }

    pub fn check_state(&self, state: &[u8; 4], type_: u8) -> bool {
        for (i, mask) in self.state_mask.iter().enumerate() {
            if !mask.matches(state[i]) {
                return false;
            }
        }
        
        self.type_mask.matches(type_)
    }

    /// Is the given point within the AI zone?
    ///
    /// Note that AiZone does not keep track of its center, so the given point should be relative
    /// to the center of the cone.
    pub fn is_point_in_zone(&self, point: Vec2, facing_angle: Fixed32) -> bool {
        if point.len() > self.radius.to_32() {
            return false;
        }

        let threshold = self.half_angle.to_32();
        let angle_to_point = Vec2::zero().angle_between(&point);
        let angle = (angle_to_point - facing_angle + threshold).0 & 0xfff;
        // the & 0xffff here should be redundant since we just did & 0xfff, but this is what the
        // game does, so we'll do it too.
        ((angle & 0xffff) < threshold.0 * 2) ^ self.inverted
    }
}

#[derive(Debug)]
pub struct PositionedAiZone {
    pub ai_zone: &'static AiZone,
    pub character_id: CharacterId,
    pub character_index: usize,
    pub pos: Vec2,
    pub angle: Fixed32,
    pub floor: Floor,
}

impl PositionedAiZone {
    pub fn new(ai_zone: &'static AiZone, character_id: CharacterId, character_index: usize, pos: Vec2, angle: Fixed32, floor: Floor) -> Self {
        PositionedAiZone {
            ai_zone,
            character_id,
            character_index,
            pos,
            angle,
            floor,
        }
    }
}

impl GameObject for PositionedAiZone {
    fn object_type(&self) -> ObjectType {
        self.ai_zone.behavior_type.into()
    }

    fn contains_point(&self, point: Vec2) -> bool {
        self.ai_zone.is_point_in_zone(point - self.pos, self.angle)
    }

    fn name(&self) -> String {
        self.ai_zone.name.to_string()
    }

    fn description(&self) -> String {
        format!(
            "Arc: {:.1}° | Angle: {:.1}° | Radius: {}\n{}",
            self.ai_zone.half_angle.to_degrees() * 2.0,
            self.angle.to_degrees(),
            self.ai_zone.radius,
            self.ai_zone.description
        )
    }

    fn details(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = Vec::new();

        groups.push((String::from("AI Zone"), vec![
            format!("Behavior: {}", self.ai_zone.behavior_type.name()),
            format!("Arc: {:.1}°", self.ai_zone.half_angle.to_degrees() * 2.0),
            format!("Angle: {:.1}°", self.angle.to_degrees()),
            format!("Radius: {}", self.ai_zone.radius),
            format!("Inverted: {}", self.ai_zone.inverted),
        ]));

        groups
    }

    fn floor(&self) -> Floor {
        self.floor
    }

    fn gui_shape(&self, params: &DrawParams, state: &State) -> Shape {
        self.ai_zone.gui_shape(self.angle, self.pos, params.clone(), state)
    }
}

pub const fn describe_player_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x01, _, _] => "Walk",
        [0x01, 0x02, _, _] => "Run",
        [0x01, 0x03, _, _] => "Backpedal",
        [0x01, 0x04, _, _] => "Turn",
        [0x01, 0x05, _, _] => "Weapon",
        [0x01, 0x07, _, _] => "Stairs",
        [0x01, 0x08, _, _] => "Climb",
        [0x01, 0x0A, _, _] => "Push object",
        [0x05, 0x00, 0x03, _] => "Grabbed",
        [0x05, 0x00, 0x05, _] => "Push enemy",
        _ => "Unknown",
    }
}

pub const fn describe_crawling_zombie_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Crawl",
        [0x01, 0x01, _, _] => "Bite",
        [0x02, _, _, _] => "Hit",
        [0x03, _, _, _] => "Dying",
        [0x07, _, _, _] => "Dead",
        _ => "Unknown",
    }
}

pub const fn describe_zombie_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, 0x03, _] => "Idle wander",
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x01, _, _] => "Walk",
        [0x01, 0x02, _, _] => "Walk (arms raised)",
        [0x01, 0x03, _, _] => "Grab",
        [0x01, 0x05, _, _] => "Knockdown",
        [0x01, 0x08, _, _] => "Eat",
        [0x01, 0x09, _, _] => "Knockback",
        [0x01, 0x0C, _, _] => "Lunge",
        [0x01, 0x0E, _, _] => "Puke",
        [0x02, _, _, _] => "Hit",
        [0x03, _, _, _] => "Die",
        [0x07, _, _, _] => "Dead",
        _ => "Unknown",
    }
}

pub const fn describe_licker_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x02, _, _] => "Recoil",
        [0x01, 0x06, _, _] => "Threatened",
        [0x01, 0x08, _, _] => "Lick",
        [0x01, 0x09, _, _] => "Slash",
        [0x01, 0x0A, _, _] => "Jump",
        [0x01, 0x0C, _, _] => "Investigate",
        [0x01, 0x0E, _, timer] => if *timer <= 10 {
            "Pre-alert"
        } else {
            "Alert"
        },
        [0x01, 0x0F, _, _] => "Try attack",
        [0x01, 0x14, _, _] => "Pursue",
        _ => "Unknown",
    }
}

pub const fn describe_dog_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x01, _, _] => "Walk",
        [0x01, 0x02, _, _] => "Run",
        [0x01, 0x03, _, _] => "Jump",
        [0x01, 0x05, _, _] => "Growl",
        [0x01, 0x06, _, _] => "Get up",
        [0x02, _, _, _] => "Hit",
        [0x03, _, _, _] => "Dying",
        [0x07, _, _, _] => "Dead",
        _ => "Unknown",
    }
}

pub const fn describe_spider_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x01, _, _] => "Pursue fast",
        [0x01, 0x02, _, _] => "Pursue medium",
        [0x01, 0x03, _, _] => "Pursue slow",
        [0x01, 0x04, _, _] => "Turn",
        [0x01, 0x05, _, _] => "Face target coarse",
        [0x01, 0x06, _, _] => "Face target fine",
        [0x01, 0x07, _, _] => "Leg attack",
        [0x01, 0x08, _, _] => "Poison",
        [0x01, 0x09, _, _] => "Investigate",
        [0x02, _, _, _] => "Hit",
        [0x03, _, _, _] => "Dying",
        [0x07, _, _, _] => "Dead",
        _ => "Unknown",
    }
}

pub const fn describe_g2_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x01, _, _] => "Pursue",
        [0x01, 0x02, _, _] => "Heavy slash",
        [0x01, 0x03, _, _] => "Taunt",
        [0x01, 0x04, _, _] => "Attack",
        [0x01, 0x05, _, _] => "Retreat",
        [0x01, 0x06, _, _] => "Invincible",
        [0x01, 0x07, _, _] => "Cutscene",
        [0x01, 0x08, _, _] => "Thrust",
        [0x02, 0x05, _, _] => "Staggered",
        [0x02, _, _, _] => "Hit",
        [0x03, _, _, _] => "Dying",
        [0x07, _, _, _] => "Dead",
        _ => "Unknown",
    }
}

pub const G2_AI_ZONES: [AiZone; 35] = [
    AiZone::circle(
        "Hit",
        "Birkin's attack hits you",
        BehaviorType::Hit,
        UFixed16(800),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Either(0x00, 0x01), StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x02)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::circle(
        "Hit",
        "Birkin's attack hits you",
        BehaviorType::Hit,
        UFixed16(500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Either(0x02, 0x03), StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x02)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::arc(
        "Retreat",
        "Birkin will retreat a short distance from you",
        BehaviorType::ChangeTactic,
        Fixed16(0x400),
        UFixed16(1500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x01)),
    AiZone::arc(
        "Slash",
        "Birkin will slash strongly at you with his claw",
        BehaviorType::Attack,
        Fixed16(0x270),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x01)),
    AiZone::arc(
        "Retreat",
        "Birkin will retreat a short distance from you",
        BehaviorType::ChangeTactic,
        Fixed16(0x400),
        UFixed16(1500),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x05, 0x19)),
    AiZone::arc(
        "Slash",
        "Birkin will slash strongly at you with his claw",
        BehaviorType::Attack,
        Fixed16(0x270),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x19)),
    AiZone::arc(
        "Pursue",
        "Birkin will pursue you",
        BehaviorType::Aggro,
        Fixed16(0x400),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x05), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x01)),
    AiZone::circle(
        "Hit",
        "Birkin's attack hits you",
        BehaviorType::Hit,
        UFixed16(800),
        [StateMask::Exactly(0x01), StateMask::Either(0x02, 0x04), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x01)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::arc(
        "Attack",
        "Birkin will either do a heavy slash or a regular attack",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x13)),
    AiZone::arc(
        "Slash",
        "Birkin will slash strongly at you with his claw",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x05)),
    AiZone::arc(
        "Attack",
        "Birkin will attack",
        BehaviorType::Attack,
        Fixed16(0x4c8),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x05)),
    AiZone::arc(
        "Thrust",
        "Birkin has a 25% chance to thrust with his claw at the sound of a running footstep",
        BehaviorType::Attack,
        Fixed16(0x270),
        UFixed16(3400),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x05)),
    AiZone::arc(
        "Retreat",
        "Birkin will retreat a short distance from you",
        BehaviorType::ChangeTactic,
        Fixed16(0x400),
        UFixed16(1500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::arc(
        "Slash",
        "Birkin will slash strongly at you with his claw",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::arc(
        "Attack",
        "Birkin will attack",
        BehaviorType::Attack,
        Fixed16(0x4c8),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::arc(
        "Thrust",
        "Birkin has a 25% chance to thrust with his claw at the sound of a running footstep",
        BehaviorType::Attack,
        Fixed16(0x270),
        UFixed16(3400),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::arc(
        "Disengagement attack",
        "Birkin will attack if you attempt to run away",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(1500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x03), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::arc(
        "Disengagement attack",
        "Birkin will attack if you attempt to run away",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(1500),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x07)),
    // FIXME: these two should have a minimum distance of 2500 and no maximum distance, but we don't
    //  currently support that
    AiZone::arc(
        "Pursue",
        "Birkin will pursue you",
        BehaviorType::Aggro,
        Fixed16(0x400),
        UFixed16(10000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x05), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::arc(
        "Pursue",
        "Birkin will pursue you",
        BehaviorType::Aggro,
        Fixed16(0x400),
        UFixed16(10000),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x09)),
    AiZone::arc(
        "Attack",
        "Birkin will either do a heavy slash or a regular attack",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x05), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::OneOf3(0x00, 0x01, 0x03)),
    AiZone::arc(
        "Attack",
        "Birkin will attack at the sound of a running footstep if you are in fine health",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x05), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::arc(
        "Attack",
        "Birkin will attack at the sound of a running footstep if you are in fine health",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x09)),
    // FIXME: these two zones should be the inverse of this, but not only in the sense of the angle
    //  the way we currently interpret it
    AiZone::arc(
        "Pursue",
        "Birkin will pursue you if you exit this zone",
        BehaviorType::Aggro,
        Fixed16(0x590),
        UFixed16(3800),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x02), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::arc(
        "Pursue",
        "Birkin will pursue you if you exit this zone",
        BehaviorType::Aggro,
        Fixed16(0x590),
        UFixed16(3800),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x10)),
    AiZone::circle(
        "Hit",
        "Birkin's attack will hit you",
        BehaviorType::Hit,
        UFixed16(1200),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x02), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::circle(
        "Hit",
        "Birkin's attack will hit you",
        BehaviorType::Hit,
        UFixed16(1200),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x10)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::circle(
        "Slash",
        "Birkin has a 75% chance to slash at you at the end of his animation",
        BehaviorType::Attack,
        UFixed16(3400),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x03), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)),
    AiZone::circle(
        "Slash",
        "Birkin has a 75% chance to slash at you at the end of his animation",
        BehaviorType::Attack,
        UFixed16(3400),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x11)),
    AiZone::circle(
        "Hit",
        "Birkin's attack will hit you",
        BehaviorType::Hit,
        UFixed16(800),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::circle(
        "Hit",
        "Birkin's attack will hit you",
        BehaviorType::Hit,
        UFixed16(800),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Any, StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x16)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::circle(
        "Hit",
        "Birkin's attack will hit you",
        BehaviorType::Hit,
        UFixed16(800),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Exactly(0x06), StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x12)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::circle(
        "Hit",
        "Birkin's attack will hit you",
        BehaviorType::Hit,
        UFixed16(800),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x04), StateMask::Exactly(0x06), StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)).with_origin(ZoneOrigin::ModelPart(6)),
    AiZone::circle(
        "Hit",
        "Birkin's attack will hit you",
        BehaviorType::Hit,
        UFixed16(800),
        [StateMask::Exactly(0x01), StateMask::Any, StateMask::Exactly(0x01), StateMask::Any],
    ).with_type_mask(StateMask::Exactly(0x12)).with_origin(ZoneOrigin::ModelPart(11)),
    AiZone::circle(
        "Hit",
        "Birkin's attack will hit you",
        BehaviorType::Hit,
        UFixed16(800),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x04), StateMask::Exactly(0x01), StateMask::Any],
    ).with_type_mask(StateMask::Either(0x00, 0x03)).with_origin(ZoneOrigin::ModelPart(11)),
];

// FIXME: spiders have different AI behavior depending whether they're on the ground, wall, or ceiling,
//  but we don't track the variable that tells us this
// FIXME: don't know enough about projectiles to show hit information for poison spit
pub const SPIDER_AI_ZONES: [AiZone; 8] = [
    AiZone::arc(
        "Face target",
        "Spider will turn towards its target",
        BehaviorType::Aggro,
        Fixed16(0x400),
        UFixed16(8000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    ).inverted(),
    AiZone::arc(
        "Leg attack",
        "Spider will attack with its front legs",
        BehaviorType::Attack,
        Fixed16(0x80),
        UFixed16(7500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Attack",
        "Spider will either attack with its front legs (90.625%) or spit poison (9.375%)",
        BehaviorType::Attack,
        Fixed16(0x80),
        UFixed16(6000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    ),
    AiZone::circle(
        "Stop pursuit",
        "Spider will stop pursuing its target",
        BehaviorType::ChangeTactic,
        UFixed16(0x5db),
        [StateMask::Exactly(0x01), StateMask::Between(0x01, 0x03), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Stop facing",
        "Spider will stop turning towards its target",
        BehaviorType::ChangeTactic,
        Fixed16(0x80),
        UFixed16(10000), // FIXME: there actually is no limit on the distance. how should we handle this in the UI?
        [StateMask::Exactly(0x01), StateMask::Exactly(0x05), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Stop facing",
        "Spider will stop turning towards its target",
        BehaviorType::ChangeTactic,
        Fixed16(0x20),
        UFixed16(10000), // FIXME: there actually is no limit on the distance. how should we handle this in the UI?
        [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Leg attack",
        "Spider's leg attack will be able to hit once you're in this zone",
        BehaviorType::Attack,
        Fixed16(0x40),
        UFixed16(10000), // FIXME: there actually is no limit on the distance. how should we handle this in the UI?
        [StateMask::Exactly(0x01), StateMask::Exactly(0x07), StateMask::Either(0x02, 0x03), StateMask::Any],
    ),
    // FIXME: this is kind of an oversimplification. the angle and distance checks are separate, so in theory,
    //  if you exited the arc on the same frame that you approached within the distance threshold, you would
    //  still get hit.
    AiZone::arc(
        "Leg attack hit",
        "Spider's leg attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x200),
        UFixed16(0x5dc),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x07), StateMask::Either(0x04, 0x05), StateMask::Any],
    ),
];

pub const DOG_AI_ZONES: [AiZone; 4] = [
    AiZone::arc(
        "Jump",
        "Dog will jump at you",
        BehaviorType::Attack,
        Fixed16(0x80),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ),
    // TODO: in some cases the dog doesn't immediately jump at you even when you're in this zone and
    //  I haven't been able to figure out why
    AiZone::arc(
        "Jump",
        "Dog will jump at you",
        BehaviorType::Attack,
        Fixed16(0x100),
        UFixed16(4000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x02), StateMask::Either(0x01, 0x03), StateMask::Any],
    ),
    AiZone::circle(
        "Bite",
        "Dog will bite you as it jumps at you",
        BehaviorType::Hit,
        UFixed16(1000), // range is reduced to 700 if player HP <= 12
        [StateMask::Exactly(0x01), StateMask::Exactly(0x03), StateMask::Exactly(0x01), StateMask::Any],
    ).with_origin(ZoneOrigin::ModelPart(4)),
    AiZone::circle(
        "Pursue",
        "Dog will begin to pursue you if you fire a gun in this zone",
        BehaviorType::Aggro,
        UFixed16(4000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0A), StateMask::Any, StateMask::Any],
    ),
];

pub const BLACK_LICKER_AI_ZONES: [AiZone; 24] = [
    AiZone::circle(
        "De-aggro",
        "Licker may de-aggro outside this range",
        BehaviorType::ChangeTactic,
        UFixed16(8000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x02), StateMask::Any],
    ).inverted(),
    AiZone::circle(
        "Investigate aggro",
        "Licker may attack",
        BehaviorType::Aggro,
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x02), StateMask::Any],
    ),
    AiZone::circle(
        "Sound aggro",
        "Licker will hear you if you make an audible sound",
        BehaviorType::Aggro,
        UFixed16(5000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0E), StateMask::Any, StateMask::Between(0x0B, 0xFF)],
    ),
    AiZone::circle(
        "Sound aggro",
        "Licker will hear you if you move at all",
        BehaviorType::Aggro,
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0E), StateMask::Any, StateMask::Between(0x0B, 0xFF)],
    ),
    AiZone::arc(
        "Sound alert",
        "Licker will be alerted if you make an audible sound",
        BehaviorType::ChangeTactic,
        Fixed16(0x800),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Slash hit",
        "Licker's slash attack hits you",
        BehaviorType::Hit,
        Fixed16(0x154),
        UFixed16(2800),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x09), StateMask::Any, StateMask::Between(0x02, 0x06)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(2000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x19)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x17)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(4200),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x15)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(4500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x14)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(4600),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x13)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(3500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Either(0x12, 0x16)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Either(0x11, 0x18)],
    ),
    AiZone::new(
        "Jump hit",
        "Jump attack will hit you",
        BehaviorType::Hit,
        Fixed16(461),
        Fixed16(262), // FIXME: hit detection seems weird?
        UFixed16(2199),
        false,
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0A), StateMask::Either(4, 5), StateMask::Any],
    ),
    AiZone::arc(
        "Slash",
        "Licker will slash at you with its claws",
        BehaviorType::Attack,
        Fixed16(0x300),
        UFixed16(2000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Recoil",
        "Licker will recoil from you",
        BehaviorType::ChangeTactic,
        Fixed16(0x300),
        UFixed16(2000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    ).inverted(),
    AiZone::circle(
        "Pursuit aggro",
        "Licker may attack",
        BehaviorType::Aggro,
        UFixed16(2200),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x14), StateMask::Any, StateMask::Any],
    ),
    // different from red licker
    AiZone::arc(
        "Jump 62.5%",
        "Licker has a 62.5% chance to jump at you",
        BehaviorType::Attack,
        Fixed16(0x600),
        UFixed16(10000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Caution jump 37.5%",
        "Licker has a 37.5% chance to jump at you if below fine health",
        BehaviorType::Attack,
        Fixed16(0x200),
        UFixed16(6500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Jump 37.5%",
        "Licker has a 37.5% chance to jump at you", // <= 100 HP
        BehaviorType::Attack,
        Fixed16(0x100),
        UFixed16(6500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Lick 50%",
        "Licker has a 50% chance to lick at you",
        BehaviorType::Attack,
        Fixed16(0x200),
        UFixed16(4500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Lick",
        "Licker will lick at you",
        BehaviorType::Attack,
        Fixed16(0x100),
        UFixed16(4500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::circle(
        "Slash 25%",
        "Licker has a 25% chance to slash at you",
        BehaviorType::Attack,
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Slash 50%",
        "Licker has a 50% chance to slash at you",
        BehaviorType::Attack,
        Fixed16(0x100),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
];

pub const RED_LICKER_AI_ZONES: [AiZone; 24] = [
    AiZone::circle(
        "Investigate aggro",
        "Licker may attack",
        BehaviorType::Aggro,
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x02), StateMask::Any],
    ),
    AiZone::circle(
        "De-aggro",
        "Licker may de-aggro outside this range",
        BehaviorType::ChangeTactic,
        UFixed16(8000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x02), StateMask::Any],
    ).inverted(),
    AiZone::circle(
        "Pursuit aggro",
        "Licker may attack",
        BehaviorType::Aggro,
        UFixed16(2200),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x14), StateMask::Any, StateMask::Any],
    ),
    // licker will hear you at any distance if you make a running footstep sound or fire a gun, but
    // still only in the below states
    AiZone::circle(
        "Sound aggro",
        "Licker will hear you if you make any audible sound",
        BehaviorType::Aggro,
        UFixed16(5000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0E), StateMask::Any, StateMask::Between(0x0B, 0xFF)],
    ),
    AiZone::circle(
        "Sound aggro",
        "Licker will hear you if you move at all",
        BehaviorType::Aggro,
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0E), StateMask::Any, StateMask::Between(0x0B, 0xFF)],
    ),
    AiZone::circle(
        "Sound alert",
        "Licker will be alerted if you make an audible sound",
        BehaviorType::ChangeTactic,
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Slash hit",
        "Licker's slash attack hits you",
        BehaviorType::Hit,
        Fixed16(0x154),
        UFixed16(2800),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x09), StateMask::Any, StateMask::Between(0x02, 0x06)],
    ),
    AiZone::circle(
        "Ranged",
        "Licker has a random chance to jump or lick outside this range (fine health = 37.5% to jump, 31.25% to lick; lower health = 25% to jump)",
        BehaviorType::Attack,
        UFixed16(4000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    ).inverted(),
    AiZone::arc(
        "Slash",
        "Licker will slash at you with its claws",
        BehaviorType::Attack,
        Fixed16(0x300),
        UFixed16(2000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Recoil",
        "Licker will recoil from you",
        BehaviorType::ChangeTactic,
        Fixed16(0x300),
        UFixed16(2000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    ).inverted(),
    // TODO: implement a minimum radius, as the below zones should have breaks between them
    AiZone::arc(
        "Jump",
        "Licker has a random chance to jump (fine health = 62.5% to jump; lower health = 25% to jump)",
        BehaviorType::Attack,
        Fixed16(0x600),
        UFixed16(10000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Jump",
        "Licker has a random chance to jump (fine health = 62.5% to jump; lower health = 25% to jump)",
        BehaviorType::Attack,
        Fixed16(0x100),
        UFixed16(6500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Lick",
        "Licker will lick at you",
        BehaviorType::Attack,
        Fixed16(0x200),
        UFixed16(4500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Ranged",
        "Licker has a random chance to jump or lick (fine health = 37.5% to jump, 31.25% to lick; lower health = 25% to jump)",
        BehaviorType::Attack,
        Fixed16(0x100),
        UFixed16(4500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::circle(
        "Slash",
        "Licker will slash at you with its claws",
        BehaviorType::Attack,
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(2000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x19)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x17)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(4200),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x15)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(4500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x14)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(4600),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x13)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(3500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Either(0x12, 0x16)],
    ),
    AiZone::arc(
        "Lick hit",
        "Lick attack will hit you",
        BehaviorType::Hit,
        Fixed16(0x100),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Either(0x11, 0x18)],
    ),
    AiZone::circle(
        "Attack",
        "Licker will attack if possible",
        BehaviorType::ChangeTactic,
        UFixed16(2200),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x14), StateMask::Any, StateMask::Any],
    ),
    AiZone::new(
        "Jump hit",
        "Jump attack will hit you",
        BehaviorType::Hit,
        Fixed16(461),
        Fixed16(262),
        UFixed16(2199),
        false,
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0A), StateMask::Either(4, 5), StateMask::Any],
    ),
];

pub const CRAWLING_ZOMBIE_AI_ZONES: [AiZone; 1] = [
    AiZone::arc(
        "Bite",
        "Zombie will bite you",
        BehaviorType::Hit,
        Fixed16(0x200),
        UFixed16(1300),
        [StateMask::Exactly(0x01), StateMask::Either(0x00, 0x02), StateMask::Any, StateMask::Any],
    ),
];

pub const ZOMBIE_AI_ZONES: [AiZone; 10] = [
    AiZone::circle(
        "Passive aggro",
        "Zombie will begin to pursue you if you are within this zone after a random amount of time",
        BehaviorType::Aggro,
        UFixed16(7500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Either(0x01, 0x03), StateMask::Any],
    ),
    AiZone::arc(
        "Aggro",
        "Zombie will begin to pursue you",
        BehaviorType::Aggro,
        Fixed16(0x400),
        UFixed16(5000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    ),
    AiZone::arc(
        "Aggro far lunge",
        "Zombie has a 25% chance to lunge at you each loud sound",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(3500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ).inverted(),
    AiZone::circle(
        "Wander aggro",
        "Zombie will begin to pursue you if you enter this zone while the zombie is wandering",
        BehaviorType::Aggro,
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Exactly(0x03), StateMask::Any],
    ),
    AiZone::arc(
        "Far lunge",
        "Zombie has a 50% chance to lunge at you each loud sound",
        BehaviorType::Attack,
        Fixed16(800),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    ).inverted(),
    AiZone::arc(
        "Raised arm lunge",
        "Zombie has a 50% chance to lunge at you each sound",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x02), StateMask::Any, StateMask::Any],
    ).inverted(),
    AiZone::arc(
        "Aggro near lunge",
        "Zombie has a 50% chance to lunge at you each sound, in addition to the aggro far lunge chance",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(2500),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ).inverted(),
    AiZone::arc(
        "Near lunge",
        "Zombie has a second 50% chance to lunge at you each sound, in addition to the far lunge chance",
        BehaviorType::Attack,
        Fixed16(0x400),
        UFixed16(2000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    ).inverted(),
    AiZone::arc(
        "Lunge bite",
        "Zombie will bite you if you are within this zone",
        BehaviorType::Hit,
        Fixed16(0x200),
        UFixed16(1300),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x03), StateMask::Any],
    ),
    AiZone::arc(
        "Bite",
        "Zombie will bite you if you are within this zone",
        BehaviorType::Hit,
        Fixed16(0x200),
        UFixed16(1200),
        [StateMask::Exactly(0x01), StateMask::Either(0x01, 0x02), StateMask::Any, StateMask::Any],
    ),
    // TODO: puke attack; don't understand all the conditions here
    // could include zone for zombie raising its arms, but doesn't seem super useful
    // could include zone within which zombie will keep its arms raised until the timer expires, but that also doesn't seem super useful
];