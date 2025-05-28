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
    pub colliders_hit: u32,         // 114
    pub next_x: i16,                // 118
    pub next_z: i16,                // 11A
    pub unk_11c: [u8; 0x28],        // 11C
    pub velocity: SVECTOR,          // 144
    pub unk_14c: [u8; 0xa],         // 14C
    pub health: i16,                // 156
    pub motion: i16,                // 158
    pub unk_15a: [u8; 0x92],        // 15A
    pub weapon_hit_stage_frames: u8,// 1EC
    pub weapon_hit_stage_index: u8, // 1ED
    pub unk_1ee: [u8; 0x2],         // 1EE
    pub distance_to_target: u32,    // 1F0
    pub unk_1f4: u32,               // 1F4
    pub unk_1f8: u32,               // 1F8
    pub prev_state: [u8; 4],        // 1FC
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct HitBounds {
    pub x: i16,
    pub z: i16,
    pub x_size_half: i16,
    pub z_size_quarter: i16,
}

impl HitBounds {
    // most bounds don't have a z offset, so the convenience constructor will omit it for brevity
    pub const fn new(x: i16, x_size_half: i16, z_size_quarter: i16) -> Self {
        Self {
            x,
            z: 0,
            x_size_half,
            z_size_quarter,
        }
    }
    
    pub const fn zero() -> Self {
        Self {
            x: 0,
            z: 0,
            x_size_half: 0,
            z_size_quarter: 0,
        }
    }
    
    pub const fn has_area(&self) -> bool {
        self.x_size_half != 0 && self.z_size_quarter != 0
    }   
}

impl Default for HitBounds {
    fn default() -> Self {
        Self::zero()
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AimZone {
    None = 0x00,
    LowFar = 0x01,
    LowMid = 0x02,
    LowNear = 0x04,
    Mid = 0x08,
    HighNear = 0x10,
    HighMid = 0x20,
    HighFar = 0x40,
    KnifeHigh = 0x80, // don't really know how this works
}

impl Default for AimZone {
    fn default() -> Self {
        Self::None
    }   
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct WeaponRange {
    pub unk00: u8,
    pub aim_zones: [AimZone; 3],
    pub hit_bounds: [HitBounds; 3],
}

impl WeaponRange {
    pub const fn new(aim_zones: [AimZone; 3], hit_bounds: [HitBounds; 3]) -> Self {
        Self {
            unk00: 0,
            aim_zones,
            hit_bounds,
        }
    }
    
    pub const fn low(bounds0: HitBounds, bounds1: HitBounds, bounds2: HitBounds) -> Self {
        Self {
            unk00: 0,
            aim_zones: [AimZone::LowNear, AimZone::LowMid, AimZone::LowFar],
            hit_bounds: [bounds0, bounds1, bounds2],
        }
    }
    
    pub const fn mid(bounds0: HitBounds, bounds1: HitBounds, bounds2: HitBounds) -> Self {
        Self {
            unk00: 0,
            aim_zones: [AimZone::Mid, AimZone::Mid, AimZone::Mid],
            hit_bounds: [bounds0, bounds1, bounds2],
        }
    }
    
    pub const fn high(bounds0: HitBounds, bounds1: HitBounds, bounds2: HitBounds) -> Self {
        Self {
            unk00: 0,
            aim_zones: [AimZone::HighNear, AimZone::HighMid, AimZone::HighFar],
            hit_bounds: [bounds0, bounds1, bounds2],
        }
    }
    
    pub const fn one(aim_zone: AimZone, bounds: HitBounds) -> Self {
        Self {
            unk00: 0,
            aim_zones: [aim_zone, AimZone::None, AimZone::None],
            hit_bounds: [bounds, HitBounds::zero(), HitBounds::zero()],
        }   
    }
    
    pub const fn none() -> Self {
        Self {
            unk00: 0,
            aim_zones: [AimZone::None; 3],
            hit_bounds: [const { HitBounds::zero() }; 3],
        }
    }
    
    pub const fn is_empty(&self) -> bool {
        matches!(self.aim_zones, [AimZone::None, AimZone::None, AimZone::None]) || !(self.hit_bounds[0].has_area() || self.hit_bounds[1].has_area() || self.hit_bounds[2].has_area())
    }
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
        assert_eq!(offset_of!(Character, unk_94), 0x94);
        assert_eq!(offset_of!(Character, floor), 0x106);
        assert_eq!(offset_of!(Character, type_), 0x10e);
        assert_eq!(offset_of!(Character, unk_11c), 0x11c);
        assert_eq!(offset_of!(Character, motion), 0x158);
        assert_eq!(offset_of!(Character, unk_15a), 0x15a);
        assert_eq!(offset_of!(Character, distance_to_target), 0x1f0);
        assert_eq!(offset_of!(Character, prev_state), 0x1fc);
    }
}