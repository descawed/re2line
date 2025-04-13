use std::f32::consts::{PI, TAU};

use egui::{Color32, Shape};
use epaint::{ColorMode, PathShape, PathStroke};

use crate::collision::DrawParams;
use crate::draw::*;
use crate::math::*;

#[derive(Debug, Clone)]
pub enum StateMask {
    None,
    Exactly(u8),
    Any2(u8, u8),
    Any3(u8, u8, u8),
    Any4(u8, u8, u8, u8),
}

impl StateMask {
    pub fn matches(&self, state: u8) -> bool {
        match self {
            Self::None => true,
            Self::Exactly(value) => state == *value,
            Self::Any2(value1, value2) => state == *value1 || state == *value2,
            Self::Any3(value1, value2, value3) => state == *value1 || state == *value2 || state == *value3,
            Self::Any4(value1, value2, value3, value4) => state == *value1 || state == *value2 || state == *value3 || state == *value4,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BehaviorType {
    Aggro,
    Attack,
    ChangeTactic,
}

impl BehaviorType {
    pub fn default_color(&self) -> Color32 {
        match self {
            Self::Aggro => Color32::from_rgba_unmultiplied(0xfc, 0x98, 0x03, 0xb0),
            Self::Attack => Color32::from_rgba_unmultiplied(0xfc, 0x1c, 0x03, 0xb0),
            Self::ChangeTactic => Color32::from_rgba_unmultiplied(0x5e, 0x03, 0xfc, 0xb0),
        }
    }
}

#[derive(Debug)]
pub struct AiCone {
    pub name: &'static str,
    pub description: &'static str,
    pub behavior_type: BehaviorType,
    pub half_angle: Fixed12,
    pub radius: UFixed12,
    pub inverted: bool,
    pub state_mask: [StateMask; 4],
}

impl AiCone {
    pub fn gui_shape(&self, facing_angle: f32, draw_params: DrawParams) -> Shape {
        let radius = self.radius.to_f32() * draw_params.scale;
        let points = get_path_for_semicircle(draw_params.origin, radius, facing_angle, self.half_angle.to_radians(), self.inverted);
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

    pub fn check_state(&self, state: &[u8; 4]) -> bool {
        for (i, mask) in self.state_mask.iter().enumerate() {
            if !mask.matches(state[i]) {
                return false;
            }
        }
        true
    }

    /// Is the given point within the AI cone?
    ///
    /// Note that AiCone does not keep track of its center, so the given point should be relative
    /// to the center of the cone.
    pub fn is_point_in_cone(&self, point: Vec2, facing_angle: f32) -> bool {
        if point.len() > self.radius {
            return false;
        }

        let x = point.x.to_f32();
        let z = point.z.to_f32();
        let angle = TAU - z.atan2(x);
        let angle = angle - facing_angle;
        let normalized = (angle + PI).rem_euclid(TAU) - PI;

        let is_inside = normalized.abs() <= self.half_angle.to_radians().abs();
        if self.inverted {
            !is_inside
        } else {
            is_inside
        }
    }
}

pub fn describe_player_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x01, _, _] => "Walk",
        [0x01, 0x02, _, _] => "Run",
        [0x01, 0x03, _, _] => "Backpedal",
        [0x01, 0x04, _, _] => "Turn",
        [0x01, 0x05, _, _] => "Weapon",
        [0x05, 0x00, 0x03, _] => "Grabbed",
        [0x05, 0x00, 0x05, _] => "Push enemy",
        _ => "Unknown",
    }
}

pub fn describe_zombie_ai_state(state: &[u8; 4]) -> &'static str {
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
        _ => "Unknown",
    }
}

pub const ZOMBIE_AI_CONES: [AiCone; 3] = [
    AiCone {
        name: "Aggro",
        description: "Zombie will begin moving towards you if it's not already",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed12(0x400),
        radius: UFixed12(5000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::None, StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Far lunge",
        description: "Zombie has a 50% chance to lunge at you each frame",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(800),
        radius: UFixed12(3000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::None, StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Near lunge",
        description: "Zombie has a second 50% chance to lunge at you each frame, in addition to the far lunge chance",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x400),
        radius: UFixed12(2000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::None, StateMask::None, StateMask::None],
    },
];