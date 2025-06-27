use anyhow::anyhow;
use residat::common::Vec2;
use residat::re2::{Collider, Instruction, Rdt};

use crate::aot::Entity;
use crate::app::Floor as FloorId;
use crate::app::WorldPos;
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
        Self::try_from(collider.collision_mask & 0x0f).unwrap()
    }
}
 
impl TryFrom<u16> for CollisionShape {
    type Error = anyhow::Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
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
                        WorldPos::rect(Vec2::new(floor.x, floor.z), Vec2::new(floor.width, floor.height), FloorId::Id(floor.level as u8)), collision::CapsuleType::None,
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
            let world_pos = WorldPos::new(
                Vec2::new(collider.x, collider.z),
                Vec2::new(collider.w, collider.h),
                FloorId::Mask(collider.floor),
                collider.collision_mask(),
                0,
            ).with_quadrant_mask(collider.quadrant_mask);
            
            colliders.push(match CollisionShape::from_collider(collider) {
                CollisionShape::Rectangle => collision::Collider::Rect(
                    collision::RectCollider::new(world_pos, collision::CapsuleType::None)
                ),
                CollisionShape::TriangleTopRight => collision::Collider::Triangle(
                    collision::TriangleCollider::new(world_pos, collision::TriangleType::TopRight),
                ),
                CollisionShape::TriangleTopLeft => collision::Collider::Triangle(
                    collision::TriangleCollider::new(world_pos, collision::TriangleType::TopLeft),
                ),
                CollisionShape::TriangleBottomRight => collision::Collider::Triangle(
                    collision::TriangleCollider::new(world_pos, collision::TriangleType::BottomRight),
                ),
                CollisionShape::TriangleBottomLeft => collision::Collider::Triangle(
                    collision::TriangleCollider::new(world_pos, collision::TriangleType::BottomLeft),
                ),
                CollisionShape::Diamond => collision::Collider::Diamond(
                    collision::DiamondCollider::new(world_pos),
                ),
                CollisionShape::Circle => collision::Collider::Ellipse(
                    collision::EllipseCollider::new(world_pos),
                ),
                CollisionShape::HorizontalCapsule => collision::Collider::Rect(
                    collision::RectCollider::new(world_pos, collision::CapsuleType::Horizontal),
                ),
                CollisionShape::VerticalCapsule => collision::Collider::Rect(
                    collision::RectCollider::new(world_pos, collision::CapsuleType::Vertical),
                ),
                CollisionShape::Ramp => collision::Collider::Rect(
                    collision::RectCollider::new(world_pos, collision::CapsuleType::None)
                        .with_special_rect_type(collision::SpecialRectType::Ramp),
                ),
                CollisionShape::HalfPipe => collision::Collider::Rect(
                    collision::RectCollider::new(world_pos, collision::CapsuleType::None)
                        .with_special_rect_type(collision::SpecialRectType::HalfPipe),
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