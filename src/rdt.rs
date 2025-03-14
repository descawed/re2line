use std::io::{Read, Seek, SeekFrom};

use anyhow::{anyhow, Context, Result};
use binrw::{binrw, BinReaderExt};

use crate::collision;
use crate::math::{Fixed12, UFixed12};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum CollisionShape {
    Rectangle,
    TriangleTopRight, // name indicates which quadrant the corner is in
    TriangleTopLeft,
    TriangleBottomRight,
    TriangleBottomLeft,
    Diamond,
    Circle,
    RoundedRectangle,
}

impl TryFrom<u32> for CollisionShape {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 | 9 | 10 | 11 | 12 | 13 | 14 | 15 => Ok(Self::Rectangle),
            1 => Ok(Self::TriangleTopRight),
            2 => Ok(Self::TriangleTopLeft),
            3 => Ok(Self::TriangleBottomRight),
            4 => Ok(Self::TriangleBottomLeft),
            5 => Ok(Self::Diamond),
            6 => Ok(Self::Circle),
            7 | 8 => Ok(Self::RoundedRectangle),
            _ => Err(anyhow!("Unknown collision shape type {}", value)),
        }
    }
}

#[binrw]
#[derive(Debug)]
struct RdtHeader {
    n_sprite: u8,
    n_cut: u8,
    o_model: u8,
    n_item: u8,
    n_door: u8,
    n_room_at: u8,
    reverb_lv: u8,
    n_sprite_max: u8,
    // section offsets
    sound_attr_offset: u32,
    sound_header1_offset: u32,
    sound_bank1_offset: u32,
    sound_header2_offset: u32,
    sound_bank2_offset: u32,
    ota_offset: u32,
    collision_offset: u32,
    camera_pos_offset: u32,
    camera_zone_offset: u32,
    light_offset: u32,
    model_offset: u32,
    floor_offset: u32,
    block_offset: u32,
    jp_message_offset: u32,
    other_message_offset: u32,
    camera_scroll_offset: u32,
    init_script_offset: u32,
    exec_script_offset: u32,
    sprite_id_offset: u32,
    sprite_data_offset: u32,
    sprite_texture_offset: u32,
    model_texture_offset: u32,
    animation_offset: u32,
}

#[binrw]
#[derive(Debug)]
struct Collider {
    x: Fixed12,
    z: Fixed12,
    w: UFixed12,
    h: UFixed12,
    packed: u32,
    floor: u32,
}

impl Collider {
    fn shape(&self) -> CollisionShape {
        // masking with 0x0f ensures this will never fail to match
        CollisionShape::try_from(self.packed & 0x0f).unwrap()
    }
}

#[binrw]
#[derive(Debug)]
struct Collision {
    cell_x: Fixed12,
    cell_z: Fixed12,
    count: u32,
    ceiling: i32,
    dummy: u32,
    #[br(count = count - 1)]
    colliders: Vec<Collider>,
}

#[binrw]
#[derive(Debug)]
struct Floor {
    x1: Fixed12,
    z1: Fixed12,
    x2: Fixed12,
    z2: Fixed12,
    x_size: Fixed12,
    z_size: Fixed12,
}

#[derive(Debug)]
pub struct Rdt {
    collision: Collision,
}

impl Rdt {
    pub fn read<T: Read + Seek>(mut f: T) -> Result<Self> {
        let header: RdtHeader = f.read_le().context("RDT header")?;
        f.seek(SeekFrom::Start(header.collision_offset as u64))?;

        Ok(Self {
            collision: f.read_le().context("RDT collision")?,
        })
    }

    pub fn get_center(&self) -> (Fixed12, Fixed12) {
        (self.collision.cell_x, self.collision.cell_z)
    }

    pub fn get_colliders(&self) -> Vec<Box<dyn collision::Collider>> {
        let mut colliders = Vec::with_capacity(self.collision.colliders.len());

        for collider in &self.collision.colliders {
            colliders.push(match collider.shape() {
                CollisionShape::Rectangle => Box::new(collision::RectCollider::new(collider.x, collider.z, collider.w, collider.h)) as Box<dyn collision::Collider>,
                CollisionShape::TriangleTopRight => Box::new(collision::TriangleCollider::new(
                    collider.x, collider.z, collider.w, collider.h,
                    [(1.0, 1.0), (1.0, 0.0), (0.0, 0.0)],
                )) as Box<dyn collision::Collider>,
                CollisionShape::TriangleTopLeft => Box::new(collision::TriangleCollider::new(
                    collider.x, collider.z, collider.w, collider.h,
                    [(0.0, 1.0), (0.0, 0.0), (1.0, 0.0)],
                )) as Box<dyn collision::Collider>,
                CollisionShape::TriangleBottomRight => Box::new(collision::TriangleCollider::new(
                    collider.x, collider.z, collider.w, collider.h,
                    [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
                )) as Box<dyn collision::Collider>,
                CollisionShape::TriangleBottomLeft => Box::new(collision::TriangleCollider::new(
                    collider.x, collider.z, collider.w, collider.h,
                    [(0.0, 1.0), (0.0, 0.0), (1.0, 1.0)],
                )) as Box<dyn collision::Collider>,
                CollisionShape::Diamond => Box::new(collision::DiamondCollider::new(collider.x, collider.z, collider.w, collider.h)) as Box<dyn collision::Collider>,
                CollisionShape::Circle => Box::new(collision::EllipseCollider::new(collider.x, collider.z, collider.w, collider.h)) as Box<dyn collision::Collider>,
                _ => continue,
            });
        }

        colliders
    }
}