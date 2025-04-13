#![allow(non_camel_case_types)]

use binrw::binrw;

pub const FRAMES_PER_SECOND: u64 = 60;
pub const NUM_CHARACTERS: usize = 34;

#[repr(C)]
#[binrw]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SVECTOR {
    pub vx: i16,
    pub vy: i16,
    pub vz: i16,
    pub pad: i16,
}

#[repr(C)]
#[binrw]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VECTOR {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[repr(C)]
#[binrw]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MATRIX {
    pub m: [i16; 9],
    pub pad: u16,
    pub t: VECTOR,
}

#[repr(C)]
#[derive(Debug)]
pub struct Character {
    pub flags: u32,                 // 000
    pub state: [u8; 4],             // 004
    pub id: u8,                     // 005
    pub unk_09: [u8; 0x1b],         // 009
    pub transform: MATRIX,          // 024
    pub pos_short: SVECTOR,         // 044
    pub base_pos_short: SVECTOR,    // 04C
    pub unk_54: [u8; 0x22],         // 054
    pub motion_angle: i16,          // 076
    pub unk_78: [u8; 0xc],          // 078
    pub base_pos: VECTOR,           // 084
    pub x_size: u16,                // 090
    pub z_size: u16,                // 092
    pub unk_94: [u8; 0x72],         // 094
    pub floor: u8,                  // 106
    pub unk_107: [u8; 7],           // 107
    pub type_: u16,                 // 10E
    pub collision_state: u32,       // 110
    pub colliders_hit: usize,       // 114
    pub next_x: i16,                // 118
    pub next_z: i16,                // 11A
    pub unk_11c: [u8; 0x28],        // 11C
    pub velocity: SVECTOR,          // 144
    pub unk_14c: [u8; 0xa],         // 14C
    pub health: i16,                // 156
    pub motion: i16,                // 158
    pub unk_15a: [u8; 0xa2],        // 15A
    pub prev_state: [u8; 4],        // 1FC
}

#[cfg(test)]
mod tests {
    use std::mem::offset_of;
    use super::*;

    #[test]
    fn test_size() {
        assert_eq!(size_of::<SVECTOR>(), 8);
        assert_eq!(size_of::<VECTOR>(), 12);
        assert_eq!(size_of::<MATRIX>(), 32);
    }

    #[test]
    fn test_layout() {
        assert_eq!(offset_of!(Character, prev_state), 0x1fc);
    }
}