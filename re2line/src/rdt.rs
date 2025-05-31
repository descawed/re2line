use std::io::{Cursor, Read, Seek, SeekFrom};

use anyhow::{anyhow, Context, Result};
use binrw::{binrw, BinReaderExt};

use crate::aot::Entity;
use crate::collision;
use crate::math::{Fixed16, UFixed16};
use crate::script::Instruction;

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

impl TryFrom<u32> for CollisionShape {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
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
    x: Fixed16,
    z: Fixed16,
    w: UFixed16,
    h: UFixed16,
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
    cell_x: Fixed16,
    cell_z: Fixed16,
    count: u32,
    ceiling: i32,
    dummy: u32,
    #[br(count = count - 1)]
    colliders: Vec<Collider>,
}

impl Default for Collision {
    fn default() -> Self {
        Self {
            cell_x: Fixed16(0),
            cell_z: Fixed16(0),
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
    x: Fixed16,
    z: Fixed16,
    width: UFixed16,
    height: UFixed16,
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

    pub fn get_center(&self) -> (Fixed16, Fixed16) {
        (self.collision.cell_x, self.collision.cell_z)
    }

    pub fn get_floors(&self) -> Vec<collision::Collider> {
        let mut floors = Vec::with_capacity(self.floors.len());

        for floor in &self.floors {
            floors.push(
                collision::Collider::Rect(
                    collision::RectCollider::new(
                        floor.x.to_32(), floor.z.to_32(), floor.width.to_32(), floor.height.to_32(), collision::CapsuleType::None
                    ).with_special_rect_type(collision::SpecialRectType::Floor)
                )
            );
        }

        floors
    }

    pub fn get_colliders(&self) -> Vec<collision::Collider> {
        let mut colliders = Vec::with_capacity(self.collision.colliders.len());

        for collider in &self.collision.colliders {
            colliders.push(match collider.shape() {
                CollisionShape::Rectangle => collision::Collider::Rect(collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), collision::CapsuleType::None)),
                CollisionShape::TriangleTopRight => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(),
                    collision::TriangleType::TopRight,
                )),
                CollisionShape::TriangleTopLeft => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(),
                    collision::TriangleType::TopLeft,
                )),
                CollisionShape::TriangleBottomRight => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(),
                    collision::TriangleType::BottomRight,
                )),
                CollisionShape::TriangleBottomLeft => collision::Collider::Triangle(collision::TriangleCollider::new(
                    collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(),
                    collision::TriangleType::BottomLeft,
                )),
                CollisionShape::Diamond => collision::Collider::Diamond(collision::DiamondCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32())),
                CollisionShape::Circle => collision::Collider::Ellipse(collision::EllipseCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32())),
                CollisionShape::HorizontalCapsule => collision::Collider::Rect(collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), collision::CapsuleType::Horizontal)),
                CollisionShape::VerticalCapsule => collision::Collider::Rect(collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), collision::CapsuleType::Vertical)),
                CollisionShape::Ramp => collision::Collider::Rect(
                    collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), collision::CapsuleType::None).with_special_rect_type(collision::SpecialRectType::Ramp),
                ),
                CollisionShape::HalfPipe => collision::Collider::Rect(
                    collision::RectCollider::new(collider.x.to_32(), collider.z.to_32(), collider.w.to_32(), collider.h.to_32(), collision::CapsuleType::None).with_special_rect_type(collision::SpecialRectType::HalfPipe),
                ),
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
    
    pub fn print_scripts(&self) {
        println!("Init script:");
        for instruction in &self.init_script {
            println!("\t{:?}", instruction);
        }
        
        for (i, function) in self.exec_script.iter().enumerate() {
            println!("\nExec function {}:", i);
            for instruction in function.as_slice() {
                println!("\t{:?}", instruction);
            }
        }
    }
}