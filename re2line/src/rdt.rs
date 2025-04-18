use std::io::{Cursor, Read, Seek, SeekFrom};

use anyhow::{anyhow, Context, Result};
use binrw::{binrw, BinReaderExt};

use crate::aot::Entity;
use crate::collision;
use crate::math::{Fixed12, UFixed12};
use crate::script::Instruction;

const CORNER_RADIUS: f32 = Fixed12(2200).to_f32();

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
            // FIXME: 11 and 13 are not rectangles
            0 | 9 | 10 | 11 | 12 | 13 | 14 | 15 => Ok(Self::Rectangle),
            1 => Ok(Self::TriangleTopRight),
            2 => Ok(Self::TriangleTopLeft),
            3 => Ok(Self::TriangleBottomRight),
            4 => Ok(Self::TriangleBottomLeft),
            5 => Ok(Self::Diamond),
            6 => Ok(Self::Circle),
            // FIXME: these two types are not identical
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

impl RdtHeader {
    fn init_script_size(&self) -> usize {
        if self.init_script_offset == 0 {
            return 0;
        }

        let next_offset = if self.exec_script_offset > 0 {
            self.exec_script_offset
        } else if self.sprite_id_offset > 0 {
            self.sprite_id_offset
        } else if self.sprite_data_offset > 0 {
            self.sprite_data_offset
        } else if self.sprite_texture_offset > 0 {
            self.sprite_texture_offset
        } else if self.model_texture_offset > 0 {
            self.model_texture_offset
        } else if self.animation_offset > 0 {
            self.animation_offset
        } else {
            return 0;
        };

        (next_offset - self.init_script_offset) as usize
    }

    fn exec_script_size(&self) -> usize {
        if self.exec_script_offset == 0 {
            return 0;
        }

        let next_offset = if self.sprite_id_offset > 0 {
            self.sprite_id_offset
        } else if self.sprite_data_offset > 0 {
            self.sprite_data_offset
        } else if self.sprite_texture_offset > 0 {
            self.sprite_texture_offset
        } else if self.model_texture_offset > 0 {
            self.model_texture_offset
        } else if self.animation_offset > 0 {
            self.animation_offset
        } else {
            return 0;
        };

        (next_offset - self.exec_script_offset) as usize
    }
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

impl Default for Collision {
    fn default() -> Self {
        Self {
            cell_x: Fixed12(0),
            cell_z: Fixed12(0),
            count: 0,
            ceiling: 0,
            dummy: 0,
            colliders: Vec::new(),
        }
    }
}

#[binrw]
#[derive(Debug)]
struct Floor {
    x: Fixed12,
    z: Fixed12,
    width: UFixed12,
    height: UFixed12,
    unknown: u16,
    level: u16,
}

#[binrw]
#[derive(Debug)]
struct FloorData {
    num_floors: u16,
    #[br(count = num_floors)]
    floors: Vec<Floor>,
    unknown: u16,
}

#[derive(Debug)]
pub struct Rdt {
    collision: Collision,
    floors: Vec<Floor>,
    init_script: Vec<Instruction>,
    exec_script: Vec<Vec<Instruction>>,
}

impl Rdt {
    pub fn read<T: Read + Seek>(mut f: T) -> Result<Self> {
        let file_size = f.seek(SeekFrom::End(0))?;
        f.seek(SeekFrom::Start(0))?;

        let header: RdtHeader = f.read_le().context("RDT header")?;

        let collision = if header.collision_offset == 0 {
            Collision::default()
        } else {
            f.seek(SeekFrom::Start(header.collision_offset as u64))?;
            f.read_le().context("RDT collision")?
        };

        let floors = if header.floor_offset == 0 {
            Vec::new()
        } else {
            f.seek(SeekFrom::Start(header.floor_offset as u64))?;

            let floor_data: FloorData = f.read_le().context("RDT floor data")?;
            floor_data.floors
        };

        let init_script = if header.init_script_offset == 0 {
            Vec::new()
        } else {
            f.seek(SeekFrom::Start(header.init_script_offset as u64))?;
            let script_size = header.init_script_size();
            let mut buf = vec![0u8; script_size];
            f.read_exact(&mut buf)?;

            let mut script = Vec::new();
            let mut reader = Cursor::new(buf);
            while reader.position() < script_size as u64 {
                script.push(reader.read_le::<Instruction>()?);
                // the size calculation may not be reliable, so if we see the end-of-function
                // instruction, we'll go ahead and bail
                if matches!(script.last().unwrap(), Instruction::EvtEnd(_)) {
                    break;
                }
            }

            script
        };

        let exec_script = if header.exec_script_offset == 0 {
            Vec::new()
        } else {
            let exec_script_offset = header.exec_script_offset as u64;
            f.seek(SeekFrom::Start(exec_script_offset))?;

            let mut script_size = header.exec_script_size();
            if exec_script_offset + script_size as u64 > file_size {
                script_size = (file_size - exec_script_offset) as usize;
            }

            if script_size == 0 {
                Vec::new()
            } else {
                let mut buf = vec![0u8; script_size];
                f.read_exact(&mut buf)?;

                let mut reader = Cursor::new(buf);

                let offset: u16 = reader.read_le()?;
                let num_functions = (offset >> 1) as usize;

                let mut offsets = Vec::with_capacity(num_functions + 1);
                offsets.push(offset as u64);
                while offsets.len() < num_functions {
                    let offset = reader.read_le::<u16>()? as u64;
                    offsets.push(offset);
                }
                offsets.push(script_size as u64);

                let mut script = Vec::with_capacity(num_functions);
                for pair in offsets.windows(2) {
                    let offset = pair[0];
                    let next_offset = pair[1];

                    reader.seek(SeekFrom::Start(offset))?;

                    let mut function = Vec::new();
                    while reader.position() < next_offset {
                        function.push(reader.read_le::<Instruction>()?);
                        // the size calculation may not be reliable, so if we see the end-of-function
                        // instruction, we'll go ahead and bail
                        if matches!(function.last().unwrap(), Instruction::EvtEnd(_)) {
                            break;
                        }
                    }

                    script.push(function);
                }

                script
            }
        };

        Ok(Self {
            collision,
            floors,
            init_script,
            exec_script,
        })
    }

    pub fn get_center(&self) -> (Fixed12, Fixed12) {
        (self.collision.cell_x, self.collision.cell_z)
    }

    pub fn get_floors(&self) -> Vec<collision::Collider> {
        let mut floors = Vec::with_capacity(self.floors.len());

        for floor in &self.floors {
            floors.push(collision::Collider::Rect(collision::RectCollider::new(floor.x, floor.z, floor.width, floor.height, 0.0)));
        }

        floors
    }

    pub fn get_colliders(&self) -> Vec<collision::Collider> {
        let mut colliders = Vec::with_capacity(self.collision.colliders.len());

        for collider in &self.collision.colliders {
            colliders.push(match collider.shape() {
                CollisionShape::Rectangle => collision::Collider::Rect(collision::RectCollider::new(collider.x, collider.z, collider.w, collider.h, 0.0)),
                CollisionShape::TriangleTopRight => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x, collider.z, collider.w, collider.h,
                    [(1.0, 1.0), (1.0, 0.0), (0.0, 0.0)],
                )),
                CollisionShape::TriangleTopLeft => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x, collider.z, collider.w, collider.h,
                    [(0.0, 1.0), (0.0, 0.0), (1.0, 0.0)],
                )),
                CollisionShape::TriangleBottomRight => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x, collider.z, collider.w, collider.h,
                    [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
                )),
                CollisionShape::TriangleBottomLeft => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x, collider.z, collider.w, collider.h,
                    [(0.0, 1.0), (0.0, 0.0), (1.0, 1.0)],
                )),
                CollisionShape::Diamond => collision::Collider::Diamond(collision::DiamondCollider::new(collider.x, collider.z, collider.w, collider.h)),
                CollisionShape::Circle => collision::Collider::Ellipse(collision::EllipseCollider::new(collider.x, collider.z, collider.w, collider.h)),
                CollisionShape::RoundedRectangle => collision::Collider::Rect(collision::RectCollider::new(collider.x, collider.z, collider.w, collider.h, CORNER_RADIUS)),
            });
        }

        colliders
    }

    fn get_script_entities(vec: &mut Vec<Entity>, script: &[Instruction]) {
        for entity in script.iter().filter_map(Instruction::to_entity) {
            vec.push(entity);
        }
    }

    pub fn get_entities(&self) -> Vec<Entity> {
        let mut entities = Vec::new();

        Self::get_script_entities(&mut entities, &self.init_script);
        for function in &self.exec_script {
            Self::get_script_entities(&mut entities, function.as_slice());
        }

        entities
    }
}