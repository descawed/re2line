use residat::common::Vec2;
use residat::re2::Instruction;

use crate::aot::{Entity, EntityForm};
use crate::app::{Floor, WorldPos};
use crate::collision::{CapsuleType, Collider, QuadCollider, RectCollider};

pub trait InstructionExt {
    fn to_entity(&self) -> Option<Entity>;
}

impl InstructionExt for Instruction {
    fn to_entity(&self) -> Option<Entity> {
        Some(match self {
            Self::AotSet { aot, sce, sat, n_floor, x, z, w, h, .. } => Entity::new(
                EntityForm::Other,
                Collider::Rect(RectCollider::new(WorldPos::rect(Vec2::new(*x, *z), Vec2::new(*w, *h), Floor::Aot(*n_floor)), CapsuleType::None)),
                *n_floor,
                *aot as u8,
                *sce,
                *sat,
            ),
            Self::DoorAotSet { aot, sce, sat, n_floor, x, z, w, h, next_pos_x, next_pos_y, next_pos_z, next_cdir_y, next_stage, next_room, next_nfloor, .. } =>
                Entity::new(
                    EntityForm::Door {
                        next_pos_x: *next_pos_x,
                        next_pos_y: *next_pos_y,
                        next_pos_z: *next_pos_z,
                        next_cdir_y: *next_cdir_y,
                        next_stage: *next_stage,
                        next_room: *next_room,
                        next_n_floor: *next_nfloor,
                    },
                    Collider::Rect(RectCollider::new(WorldPos::rect(Vec2::new(*x, *z), Vec2::new(*w, *h), Floor::Aot(*n_floor)), CapsuleType::None)),
                    *n_floor,
                    *aot,
                    *sce,
                    *sat,
                ),
            Self::AotSet4p { aot, sce, sat, n_floor, x0, z0, x1, z1, x2, z2, x3, z3, .. } => Entity::new(
                EntityForm::Other,
                Collider::Quad(QuadCollider::new((*x0).to_32(), (*z0).to_32(), (*x1).to_32(), (*z1).to_32(), (*x2).to_32(), (*z2).to_32(), (*x3).to_32(), (*z3).to_32(), Floor::Aot(*n_floor))),
                *n_floor,
                *aot,
                *sce,
                *sat,
            ),
            Self::DoorAotSet4p { aot, sce, sat, n_floor, x0, z0, x1, z1, x2, z2, x3, z3, next_pos_x, next_pos_y, next_pos_z, next_cdir_y, next_stage, next_room, next_nfloor, .. } =>
                Entity::new(
                    EntityForm::Door {
                        next_pos_x: *next_pos_x,
                        next_pos_y: *next_pos_y,
                        next_pos_z: *next_pos_z,
                        next_cdir_y: *next_cdir_y,
                        next_stage: *next_stage,
                        next_room: *next_room,
                        next_n_floor: *next_nfloor,
                    },
                    Collider::Quad(QuadCollider::new((*x0).to_32(), (*z0).to_32(), (*x1).to_32(), (*z1).to_32(), (*x2).to_32(), (*z2).to_32(), (*x3).to_32(), (*z3).to_32(), Floor::Aot(*n_floor))),
                    *n_floor,
                    *aot,
                    *sce,
                    *sat,
                ),
            Self::ItemAotSet4p { aot, sce, sat, n_floor, x0, z0, x1, z1, x2, z2, x3, z3, i_item, n_item, flag, md1, action, .. } => Entity::new(
                EntityForm::Item {
                    i_item: *i_item,
                    n_item: *n_item,
                    flag: *flag,
                    md1: *md1,
                    action: *action,
                },
                Collider::Quad(QuadCollider::new((*x0).to_32(), (*z0).to_32(), (*x1).to_32(), (*z1).to_32(), (*x2).to_32(), (*z2).to_32(), (*x3).to_32(), (*z3).to_32(), Floor::Aot(*n_floor))),
                *n_floor,
                *aot,
                *sce,
                *sat,
            ),
            Self::ItemAotSet { aot, sce, sat, n_floor, x, z, w, h, i_item, n_item, flag, md1, action, .. } |
            Self::ItemAotSet2 { aot, sce, sat, n_floor, x, z, w, h, i_item, n_item, flag, md1, action, .. } => Entity::new(
                EntityForm::Item {
                    i_item: *i_item,
                    n_item: *n_item,
                    flag: *flag,
                    md1: *md1,
                    action: *action,
                },
                Collider::Rect(RectCollider::new(WorldPos::rect(Vec2::new(*x, *z), Vec2::new(*w, *h), Floor::Aot(*n_floor)), CapsuleType::None)),
                *n_floor,
                *aot,
                *sce,
                *sat,
            ),
            _ => return None,
        })
    }
}