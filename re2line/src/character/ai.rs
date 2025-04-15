use std::f32::consts::{PI, TAU};

use egui::{Color32, Shape, Stroke};
use epaint::{CircleShape, ColorMode, PathShape, PathStroke};

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
    Hit,
}

impl BehaviorType {
    pub fn default_color(&self) -> Color32 {
        match self {
            Self::Aggro => Color32::from_rgba_unmultiplied(0xfc, 0x98, 0x03, 0xb0),
            Self::Attack => Color32::from_rgba_unmultiplied(0xfc, 0x1c, 0x03, 0xb0),
            Self::ChangeTactic => Color32::from_rgba_unmultiplied(0x5e, 0x03, 0xfc, 0xb0),
            Self::Hit => Color32::from_rgba_unmultiplied(0x4a, 0x04, 0x2e, 0xb0),
        }
    }
}

#[derive(Debug)]
pub struct AiCone {
    pub name: &'static str,
    pub description: &'static str,
    pub behavior_type: BehaviorType,
    pub half_angle: Fixed12,
    pub offset_angle: Fixed12,
    pub radius: UFixed12,
    pub inverted: bool,
    pub state_mask: [StateMask; 4],
}

impl AiCone {
    pub fn gui_shape(&self, facing_angle: f32, draw_params: DrawParams) -> Shape {
        let radians = self.half_angle.to_radians();
        let radius = self.radius.to_f32() * draw_params.scale;
        if radians.abs() >= PI {
            // just use a circle
            // for an inverted circle, we treat the outside of the circle as being in the zone, and
            // we just draw an outline rather than doing a fill out to the edges of the map
            return Shape::Circle(if self.inverted {
                CircleShape {
                    center: draw_params.origin,
                    radius,
                    fill: Color32::TRANSPARENT,
                    stroke: Stroke {
                        width: 2.0,
                        color: draw_params.fill_color,
                    },
                }
            } else {
                CircleShape {
                    center: draw_params.origin,
                    radius,
                    fill: draw_params.fill_color,
                    stroke: draw_params.stroke.clone(),
                }
            });
        }

        let offset = self.offset_angle.to_radians();
        let points = get_path_for_semicircle(draw_params.origin, radius, facing_angle + offset, radians, self.inverted);
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

pub fn describe_crawling_zombie_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Crawl",
        [0x01, 0x01, _, _] => "Bite",
        [0x02, _, _, _] => "Hit",
        [0x03, _, _, _] => "Dying",
        [0x07, _, _, _] => "Dead",
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
        [0x07, _, _, _] => "Dead",
        _ => "Unknown",
    }
}

pub const LICKER_AI_CONES: [AiCone; 17] = [
    AiCone {
        name: "Slash hit",
        description: "Licker's slash attack hits you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x154),
        offset_angle: Fixed12(0),
        radius: UFixed12(2800),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x09), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Ranged",
        description: "Licker has a random chance to jump or lick outside this range (fine health = 37.5% to jump, 31.25% to lick; lower health = 25% to jump)",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x800),
        offset_angle: Fixed12(0),
        radius: UFixed12(4000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Slash",
        description: "Licker will slash at you with its claws",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x300),
        offset_angle: Fixed12(0),
        radius: UFixed12(2000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::None, StateMask::None],
    },
    // TODO: implement a minimum radius, as the below cones should have breaks between them
    AiCone {
        name: "Jump",
        description: "Licker has a random chance to jump (fine health = 37.5% to jump; lower health = 25% to jump)",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x600),
        offset_angle: Fixed12(0),
        radius: UFixed12(10000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Jump",
        description: "Licker has a random chance to jump (fine health = 37.5% to jump; lower health = 25% to jump)",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(6500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Lick",
        description: "Licker will lick at you",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x200),
        offset_angle: Fixed12(0),
        radius: UFixed12(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Ranged",
        description: "Licker has a random chance to jump or lick (fine health = 37.5% to jump, 31.25% to lick; lower health = 25% to jump)",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Slash",
        description: "Licker will slash at you with its claws",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x800),
        offset_angle: Fixed12(0),
        radius: UFixed12(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(2000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::None, StateMask::Exactly(0x19)],
    },
    AiCone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::None, StateMask::Exactly(0x17)],
    },
    AiCone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(4200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::None, StateMask::Exactly(0x15)],
    },
    AiCone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::None, StateMask::Exactly(0x14)],
    },
    AiCone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(4600),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::None, StateMask::Exactly(0x13)],
    },
    AiCone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(3500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::None, StateMask::Any2(0x12, 0x16)],
    },
    AiCone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x100),
        offset_angle: Fixed12(0),
        radius: UFixed12(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::None, StateMask::Any2(0x11, 0x18)],
    },
    AiCone {
        name: "Attack",
        description: "Licker will attack if possible",
        behavior_type: BehaviorType::ChangeTactic,
        half_angle: Fixed12(0x800),
        offset_angle: Fixed12(0),
        radius: UFixed12(2200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x14), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Jump hit",
        description: "Jump attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(461),
        offset_angle: Fixed12(262),
        radius: UFixed12(2199),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0A), StateMask::Any2(4, 5), StateMask::None],
    },
];

pub const CRAWLING_ZOMBIE_AI_CONES: [AiCone; 1] = [
    AiCone {
        name: "Bite",
        description: "Zombie will bite you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x200),
        offset_angle: Fixed12(0),
        radius: UFixed12(1300),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Any2(0x00, 0x02), StateMask::None, StateMask::None],
    },
];

pub const ZOMBIE_AI_CONES: [AiCone; 10] = [
    AiCone {
        name: "Passive aggro",
        description: "Zombie will begin to pursue you if you are within this zone after a random amount of time",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed12(0x800),
        offset_angle: Fixed12(0),
        radius: UFixed12(7500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any2(0x01, 0x03), StateMask::None],
    },
    AiCone {
        name: "Aggro",
        description: "Zombie will begin to pursue you",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed12(0x400),
        offset_angle: Fixed12(0),
        radius: UFixed12(5000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Aggro far lunge",
        description: "Zombie has a 25% chance to lunge at you each frame",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x400),
        offset_angle: Fixed12(0),
        radius: UFixed12(3500),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Wander aggro",
        description: "Zombie will begin to pursue you if you enter this zone while the zombie is wandering",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed12(0x800),
        offset_angle: Fixed12(0),
        radius: UFixed12(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Exactly(0x03), StateMask::None],
    },
    AiCone {
        name: "Far lunge",
        description: "Zombie has a 50% chance to lunge at you each frame",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(800),
        offset_angle: Fixed12(0),
        radius: UFixed12(3000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Raised arm lunge",
        description: "Zombie has a 50% chance to lunge at you each frame",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x400),
        offset_angle: Fixed12(0),
        radius: UFixed12(3000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x02), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Aggro near lunge",
        description: "Zombie has a 50% chance to lunge at you each frame, in addition to the aggro far lunge chance",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x400),
        offset_angle: Fixed12(0),
        radius: UFixed12(2500),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Near lunge",
        description: "Zombie has a second 50% chance to lunge at you each frame, in addition to the far lunge chance",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed12(0x400),
        offset_angle: Fixed12(0),
        radius: UFixed12(2000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::None, StateMask::None],
    },
    AiCone {
        name: "Lunge bite",
        description: "Zombie will bite you if you are within this zone",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x200),
        offset_angle: Fixed12(0),
        radius: UFixed12(1300),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x03), StateMask::None],
    },
    AiCone {
        name: "Bite",
        description: "Zombie will bite you if you are within this zone",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed12(0x200),
        offset_angle: Fixed12(0),
        radius: UFixed12(1200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Any2(0x01, 0x02), StateMask::None, StateMask::None],
    },
    // TODO: puke attack; don't understand all the conditions here
    // could include zone for zombie raising its arms, but doesn't seem super useful
    // could include zone within which zombie will keep its arms raised until the timer expires, but that also doesn't seem super useful
];