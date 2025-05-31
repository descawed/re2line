use std::f32::consts::PI;

use egui::{Color32, Shape, Stroke};
use epaint::{CircleShape, ColorMode, PathShape, PathStroke};

use crate::app::{DrawParams, Floor, GameObject, ObjectType};
use crate::character::CharacterId;
use crate::draw::*;
use crate::math::*;
use crate::record::State;

#[derive(Debug, Clone)]
pub enum StateMask {
    Any,
    Exactly(u8),
    Either(u8, u8),
    Between(u8, u8),
}

impl StateMask {
    pub fn matches(&self, state: u8) -> bool {
        match self {
            Self::Any => true,
            Self::Exactly(value) => state == *value,
            Self::Either(value1, value2) => state == *value1 || state == *value2,
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
}

impl AiZone {
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
        }
    }

    pub fn gui_shape(&self, angle: Fixed32, pos: Vec2, mut draw_params: DrawParams, state: &State) -> Shape {
        let facing_angle = angle.to_radians();
        
        let (gui_x, gui_y, _, _) = draw_params.transform(pos.x, pos.z, 0, 0);
        let gui_pos = egui::Pos2::new(gui_x, gui_y);
        
        // if the player is in this zone, draw it with an outline
        if let Some(ref player) = state.characters()[0] {
            if self.is_point_in_zone(player.center.saturating_sub(pos), angle) {
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

    pub fn check_state(&self, state: &[u8; 4]) -> bool {
        for (i, mask) in self.state_mask.iter().enumerate() {
            if !mask.matches(state[i]) {
                return false;
            }
        }
        true
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

pub fn describe_player_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x01, _, _] => "Walk",
        [0x01, 0x02, _, _] => "Run",
        [0x01, 0x03, _, _] => "Backpedal",
        [0x01, 0x04, _, _] => "Turn",
        [0x01, 0x05, _, _] => "Weapon",
        [0x01, 0x0A, _, _] => "Push object",
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

pub fn describe_licker_ai_state(state: &[u8; 4]) -> &'static str {
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

pub fn describe_dog_ai_state(state: &[u8; 4]) -> &'static str {
    match state {
        [0x01, 0x00, _, _] => "Idle",
        [0x01, 0x01, _, _] => "Walk",
        [0x01, 0x02, _, _] => "Run",
        [0x01, 0x03, _, _] => "Jump",
        [0x07, _, _, _] => "Dead",
        _ => "Unknown",
    }
}

pub const DOG_AI_ZONES: [AiZone; 1] = [
    AiZone::arc(
        "Jump",
        "Dog will jump at you",
        BehaviorType::Attack,
        Fixed16(0x80),
        UFixed16(3000),
        [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    ),
];

pub const BLACK_LICKER_AI_ZONES: [AiZone; 24] = [
    // same as red licker
    AiZone {
        name: "De-aggro",
        description: "Licker may de-aggro outside this range",
        behavior_type: BehaviorType::ChangeTactic,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(8000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x02), StateMask::Any],
    },
    AiZone {
        name: "Investigate aggro",
        description: "Licker may attack",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x02), StateMask::Any],
    },
    AiZone {
        name: "Sound aggro",
        description: "Licker will hear you if you make an audible sound",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(5000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0E), StateMask::Any, StateMask::Between(0x0B, 0xFF)],
    },
    AiZone {
        name: "Sound aggro",
        description: "Licker will hear you if you move at all",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0E), StateMask::Any, StateMask::Between(0x0B, 0xFF)],
    },
    AiZone {
        name: "Sound alert",
        description: "Licker will be alerted if you make an audible sound",
        behavior_type: BehaviorType::ChangeTactic,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Slash hit",
        description: "Licker's slash attack hits you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x154),
        offset_angle: Fixed16(0),
        radius: UFixed16(2800),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x09), StateMask::Any, StateMask::Between(0x02, 0x06)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(2000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x19)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x17)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(4200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x15)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x14)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(4600),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x13)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(3500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Either(0x12, 0x16)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Either(0x11, 0x18)],
    },
    AiZone {
        name: "Jump hit",
        description: "Jump attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(461),
        offset_angle: Fixed16(262), // FIXME: hit detection seems weird?
        radius: UFixed16(2199),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0A), StateMask::Either(4, 5), StateMask::Any],
    },
    AiZone {
        name: "Slash",
        description: "Licker will slash at you with its claws",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x300),
        offset_angle: Fixed16(0),
        radius: UFixed16(2000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Recoil",
        description: "Licker will recoil from you",
        behavior_type: BehaviorType::ChangeTactic,
        half_angle: Fixed16(0x300),
        offset_angle: Fixed16(0),
        radius: UFixed16(2000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Pursuit aggro",
        description: "Licker may attack",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(2200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x14), StateMask::Any, StateMask::Any],
    },
    // different from red licker
    AiZone {
        name: "Jump 62.5%",
        description: "Licker has a 62.5% chance to jump at you",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x600),
        offset_angle: Fixed16(0),
        radius: UFixed16(10000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Caution jump 37.5%",
        description: "Licker has a 37.5% chance to jump at you if below fine health", // <= 100 HP
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x200),
        offset_angle: Fixed16(0),
        radius: UFixed16(6500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Jump 37.5%",
        description: "Licker has a 37.5% chance to jump at you",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(6500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Lick 50%",
        description: "Licker has a 50% chance to lick at you",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x200),
        offset_angle: Fixed16(0),
        radius: UFixed16(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Lick",
        description: "Licker will lick at you",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Slash 25%",
        description: "Licker has a 25% chance to slash at you",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Slash 50%",
        description: "Licker has a 50% chance to slash at you",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
];

pub const RED_LICKER_AI_ZONES: [AiZone; 24] = [
    AiZone {
        name: "Investigate aggro",
        description: "Licker may attack",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x02), StateMask::Any],
    },
    AiZone {
        name: "De-aggro",
        description: "Licker may de-aggro outside this range",
        behavior_type: BehaviorType::ChangeTactic,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(8000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x02), StateMask::Any],
    },
    AiZone {
        name: "Pursuit aggro",
        description: "Licker may attack",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(2200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x14), StateMask::Any, StateMask::Any],
    },
    // licker will hear you at any distance if you make a running footstep sound or fire a gun, but
    // still only in the below states
    AiZone {
        name: "Sound aggro",
        description: "Licker will hear you if you make any audible sound",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(5000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0E), StateMask::Any, StateMask::Between(0x0B, 0xFF)],
    },
    AiZone {
        name: "Sound aggro",
        description: "Licker will hear you if you move at all",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0E), StateMask::Any, StateMask::Between(0x0B, 0xFF)],
    },
    AiZone {
        name: "Sound alert",
        description: "Licker will be alerted if you make an audible sound",
        behavior_type: BehaviorType::ChangeTactic,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Slash hit",
        description: "Licker's slash attack hits you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x154),
        offset_angle: Fixed16(0),
        radius: UFixed16(2800),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x09), StateMask::Any, StateMask::Between(0x02, 0x06)],
    },
    AiZone {
        name: "Ranged",
        description: "Licker has a random chance to jump or lick outside this range (fine health = 37.5% to jump, 31.25% to lick; lower health = 25% to jump)",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(4000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Slash",
        description: "Licker will slash at you with its claws",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x300),
        offset_angle: Fixed16(0),
        radius: UFixed16(2000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Recoil",
        description: "Licker will recoil from you",
        behavior_type: BehaviorType::ChangeTactic,
        half_angle: Fixed16(0x300),
        offset_angle: Fixed16(0),
        radius: UFixed16(2000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x06), StateMask::Any, StateMask::Any],
    },
    // TODO: implement a minimum radius, as the below cones should have breaks between them
    AiZone {
        name: "Jump",
        description: "Licker has a random chance to jump (fine health = 62.5% to jump; lower health = 25% to jump)",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x600),
        offset_angle: Fixed16(0),
        radius: UFixed16(10000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Jump",
        description: "Licker has a random chance to jump (fine health = 62.5% to jump; lower health = 25% to jump)",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(6500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Lick",
        description: "Licker will lick at you",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x200),
        offset_angle: Fixed16(0),
        radius: UFixed16(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Ranged",
        description: "Licker has a random chance to jump or lick (fine health = 37.5% to jump, 31.25% to lick; lower health = 25% to jump)",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Slash",
        description: "Licker will slash at you with its claws",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0F), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(2000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x19)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x17)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(4200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x15)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(4500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x14)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(4600),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Exactly(0x13)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(3500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Either(0x12, 0x16)],
    },
    AiZone {
        name: "Lick hit",
        description: "Lick attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x100),
        offset_angle: Fixed16(0),
        radius: UFixed16(2500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x08), StateMask::Any, StateMask::Either(0x11, 0x18)],
    },
    AiZone {
        name: "Attack",
        description: "Licker will attack if possible",
        behavior_type: BehaviorType::ChangeTactic,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(2200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x14), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Jump hit",
        description: "Jump attack will hit you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(461),
        offset_angle: Fixed16(262),
        radius: UFixed16(2199),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0A), StateMask::Either(4, 5), StateMask::Any],
    },
];

pub const CRAWLING_ZOMBIE_AI_ZONES: [AiZone; 1] = [
    AiZone {
        name: "Bite",
        description: "Zombie will bite you",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x200),
        offset_angle: Fixed16(0),
        radius: UFixed16(1300),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Either(0x00, 0x02), StateMask::Any, StateMask::Any],
    },
];

pub const ZOMBIE_AI_ZONES: [AiZone; 10] = [
    AiZone {
        name: "Passive aggro",
        description: "Zombie will begin to pursue you if you are within this zone after a random amount of time",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(7500),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Either(0x01, 0x03), StateMask::Any],
    },
    AiZone {
        name: "Aggro",
        description: "Zombie will begin to pursue you",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x400),
        offset_angle: Fixed16(0),
        radius: UFixed16(5000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Aggro far lunge",
        description: "Zombie has a 25% chance to lunge at you each loud sound",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x400),
        offset_angle: Fixed16(0),
        radius: UFixed16(3500),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Wander aggro",
        description: "Zombie will begin to pursue you if you enter this zone while the zombie is wandering",
        behavior_type: BehaviorType::Aggro,
        half_angle: Fixed16(0x800),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Exactly(0x03), StateMask::Any],
    },
    AiZone {
        name: "Far lunge",
        description: "Zombie has a 50% chance to lunge at you each loud sound",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(800),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Raised arm lunge",
        description: "Zombie has a 50% chance to lunge at you each sound",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x400),
        offset_angle: Fixed16(0),
        radius: UFixed16(3000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x02), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Aggro near lunge",
        description: "Zombie has a 50% chance to lunge at you each sound, in addition to the aggro far lunge chance",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x400),
        offset_angle: Fixed16(0),
        radius: UFixed16(2500),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x01), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Near lunge",
        description: "Zombie has a second 50% chance to lunge at you each sound, in addition to the far lunge chance",
        behavior_type: BehaviorType::Attack,
        half_angle: Fixed16(0x400),
        offset_angle: Fixed16(0),
        radius: UFixed16(2000),
        inverted: true,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x00), StateMask::Any, StateMask::Any],
    },
    AiZone {
        name: "Lunge bite",
        description: "Zombie will bite you if you are within this zone",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x200),
        offset_angle: Fixed16(0),
        radius: UFixed16(1300),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Exactly(0x0C), StateMask::Exactly(0x03), StateMask::Any],
    },
    AiZone {
        name: "Bite",
        description: "Zombie will bite you if you are within this zone",
        behavior_type: BehaviorType::Hit,
        half_angle: Fixed16(0x200),
        offset_angle: Fixed16(0),
        radius: UFixed16(1200),
        inverted: false,
        state_mask: [StateMask::Exactly(0x01), StateMask::Either(0x01, 0x02), StateMask::Any, StateMask::Any],
    },
    // TODO: puke attack; don't understand all the conditions here
    // could include zone for zombie raising its arms, but doesn't seem super useful
    // could include zone within which zombie will keep its arms raised until the timer expires, but that also doesn't seem super useful
];