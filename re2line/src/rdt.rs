use anyhow::anyhow;
use residat::re2::{Collider, Instruction, Rdt};

use crate::aot::Entity;
use crate::app::Floor as FloorId;
use crate::collision;
use crate::script::InstructionExt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum CollisionShape {
    Rectangle,
    TriangleTopRight, // name indicates which quadrant the corner is in
    TriangleTopLeft,
    TriangleBottomRight,
    TriangleBottomLeft,
    Diamond,
    Circle,
    HorizontalCapsule,
    VerticalCapsule,
    Ramp,
    HalfPipe,
}

impl CollisionShape {
    pub fn from_collider(collider: &Collider) -> CollisionShape {
        // masking with 0x0f ensures this will never fail to match
        Self::try_from(collider.packed & 0x0f).unwrap()
    }
}

impl TryFrom<u32> for CollisionShape {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 | 9 | 10 | 12 | 14 | 15 => Ok(Self::Rectangle),
            1 => Ok(Self::TriangleTopRight),
            2 => Ok(Self::TriangleTopLeft),
            3 => Ok(Self::TriangleBottomRight),
            4 => Ok(Self::TriangleBottomLeft),
            5 => Ok(Self::Diamond),
            6 => Ok(Self::Circle),
            7 => Ok(Self::HorizontalCapsule),
            8 => Ok(Self::VerticalCapsule),
            11 => Ok(Self::Ramp),
            13 => Ok(Self::HalfPipe),
            _ => Err(anyhow!("Unknown collision shape type {}", value)),
        }
    }
}

pub trait RdtExt {
    fn get_floors(&self) -> Vec<collision::Collider>;

    fn get_colliders(&self) -> Vec<collision::Collider>;

    fn get_entities(&self) -> Vec<Entity>;
}

fn get_script_entities(vec: &mut Vec<Entity>, script: &[Instruction]) {
    for entity in script.iter().filter_map(Instruction::to_entity) {
        vec.push(entity);
    }
}

impl RdtExt for Rdt {
    fn get_floors(&self) -> Vec<collision::Collider> {
        let raw_floors = self.floors();
        let mut floors = Vec::with_capacity(raw_floors.len());

        for floor in raw_floors {
            floors.push(
                collision::Collider::Rect(
                    collision::RectCollider::new(
                        floor.x.to_32(), floor.z.to_32(), floor.width.to_32(), floor.height.to_32(), FloorId::Id(floor.level as u8), collision::CapsuleType::None
                    ).with_special_rect_type(collision::SpecialRectType::Floor)
                )
            );
        }

        floors
    }

    fn get_colliders(&self) -> Vec<collision::Collider> {
        let raw_colliders = &self.collision().colliders;
        let mut colliders = Vec::with_capacity(raw_colliders.len());

        for collider in raw_colliders {
            colliders.push(match CollisionShape::from_collider(collider) {
                CollisionShape::Rectangle => collision::Collider::Rect(
                    collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), FloorId::Mask(collider.floor), collision::CapsuleType::None)
                        .with_collision_mask(collider.collision_mask())
                ),
                CollisionShape::TriangleTopRight => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(),
                    FloorId::Mask(collider.floor), collision::TriangleType::TopRight,
                ).with_collision_mask(collider.collision_mask())),
                CollisionShape::TriangleTopLeft => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(),
                    FloorId::Mask(collider.floor), collision::TriangleType::TopLeft,
                ).with_collision_mask(collider.collision_mask())),
                CollisionShape::TriangleBottomRight => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(),
                    FloorId::Mask(collider.floor), collision::TriangleType::BottomRight,
                ).with_collision_mask(collider.collision_mask())),
                CollisionShape::TriangleBottomLeft => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(),
                    FloorId::Mask(collider.floor), collision::TriangleType::BottomLeft,
                ).with_collision_mask(collider.collision_mask())),
                CollisionShape::Diamond => collision::Collider::Diamond(
                    collision::DiamondCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), FloorId::Mask(collider.floor))
                        .with_collision_mask(collider.collision_mask())
                ),
                CollisionShape::Circle => collision::Collider::Ellipse(
                    collision::EllipseCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), FloorId::Mask(collider.floor))
                        .with_collision_mask(collider.collision_mask())
                ),
                CollisionShape::HorizontalCapsule => collision::Collider::Rect(
                    collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), FloorId::Mask(collider.floor), collision::CapsuleType::Horizontal)
                        .with_collision_mask(collider.collision_mask())
                ),
                CollisionShape::VerticalCapsule => collision::Collider::Rect(
                    collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), FloorId::Mask(collider.floor), collision::CapsuleType::Vertical)
                        .with_collision_mask(collider.collision_mask())
                ),
                CollisionShape::Ramp => collision::Collider::Rect(
                    collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), FloorId::Mask(collider.floor), collision::CapsuleType::None)
                        .with_special_rect_type(collision::SpecialRectType::Ramp)
                        .with_collision_mask(collider.collision_mask()),
                ),
                CollisionShape::HalfPipe => collision::Collider::Rect(
                    collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), FloorId::Mask(collider.floor), collision::CapsuleType::None)
                        .with_special_rect_type(collision::SpecialRectType::HalfPipe)
                        .with_collision_mask(collider.collision_mask()),
                ),
            });
        }

        colliders
    }

    fn get_entities(&self) -> Vec<Entity> {
        let mut entities = Vec::new();

        get_script_entities(&mut entities, self.init_script());
        for function in self.exec_script() {
            get_script_entities(&mut entities, function);
        }

        entities
    }
}