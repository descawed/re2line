use std::f32::consts::{PI, TAU};

use binrw::binrw;
use derive_more::{Add, AddAssign, From, Into, Neg, Sub, SubAssign};

mod tables;
use tables::*;

#[binrw]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Add, AddAssign, From, Into, Neg, Sub, SubAssign)]
pub struct Fixed16(pub i16);

impl Fixed16 {
    pub const fn from_f32(f: f32) -> Self {
        Self((f * 4096.0) as i16)
    }

    pub const fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
    }
    
    pub const fn to_radians(&self) -> f32 {
        self.to_f32() * TAU
    }
    
    pub const fn to_degrees(&self) -> f32 {
        self.to_radians() * 180.0 / PI
    }
    
    pub const fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    pub const fn unsigned_abs(&self) -> UFixed16 {
        UFixed16(self.0.unsigned_abs())
    }

    pub const fn to_32(&self) -> Fixed32 {
        Fixed32(self.0 as i32)
    }
}

impl std::convert::From<f32> for Fixed16 {
    fn from(f: f32) -> Self {
        Self::from_f32(f)
    }
}

impl std::convert::From<Fixed16> for f32 {
    fn from(f: Fixed16) -> Self {
        f.to_f32()
    }
}

impl std::ops::Add<f32> for Fixed16 {
    type Output = f32;

    fn add(self, rhs: f32) -> Self::Output {
        self.to_f32() + rhs
    }
}

impl std::ops::Sub<f32> for Fixed16 {
    type Output = f32;

    fn sub(self, rhs: f32) -> Self::Output {
        self.to_f32() - rhs
    }
}

impl std::ops::Mul<Fixed16> for Fixed16 {
    type Output = Self;

    fn mul(self, rhs: Fixed16) -> Self::Output {
        let lhs_wide = self.0 as isize;
        let rhs_wide = rhs.0 as isize;

        Self(((lhs_wide * rhs_wide) >> 12) as i16)
    }
}

impl std::ops::Mul<f32> for Fixed16 {
    type Output = f32;

    fn mul(self, rhs: f32) -> Self::Output {
        self.to_f32() * rhs
    }
}

impl std::ops::Div<Fixed16> for Fixed16 {
    type Output = Self;

    fn div(self, rhs: Fixed16) -> Self::Output {
        Self((self.0 << 12) / rhs.0)
    }
}

impl std::ops::Div<f32> for Fixed16 {
    type Output = f32;

    fn div(self, rhs: f32) -> Self::Output {
        self.to_f32() / rhs
    }
}

impl PartialEq<f32> for Fixed16 {
    fn eq(&self, other: &f32) -> bool {
        self.to_f32().eq(other)
    }
}

impl PartialOrd<f32> for Fixed16 {
    fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
        self.to_f32().partial_cmp(other)
    }
}

impl std::ops::Shl<i32> for Fixed16 {
    type Output = Self;
    
    fn shl(self, rhs: i32) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl std::ops::Shr<i32> for Fixed16 {
    type Output = Self;

    fn shr(self, rhs: i32) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl std::fmt::Display for Fixed16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[binrw]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Add, AddAssign, From, Into, Sub, SubAssign)]
pub struct UFixed16(pub u16);

impl UFixed16 {
    pub const fn from_f32(f: f32) -> Self {
        Self((f * 4096.0) as u16)
    }

    pub const fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
    }

    pub const fn to_32(&self) -> Fixed32 {
        Fixed32(self.0 as i32)
    }
}

impl std::convert::From<f32> for UFixed16 {
    fn from(f: f32) -> Self {
        Self::from_f32(f)
    }
}

impl std::convert::From<UFixed16> for f32 {
    fn from(f: UFixed16) -> Self {
        f.to_f32()
    }
}

impl std::ops::Add<f32> for UFixed16 {
    type Output = f32;

    fn add(self, rhs: f32) -> Self::Output {
        self.to_f32() + rhs
    }
}

impl std::ops::Sub<f32> for UFixed16 {
    type Output = f32;

    fn sub(self, rhs: f32) -> Self::Output {
        self.to_f32() - rhs
    }
}

impl std::ops::Mul<UFixed16> for UFixed16 {
    type Output = Self;

    fn mul(self, rhs: UFixed16) -> Self::Output {
        let lhs_wide = self.0 as usize;
        let rhs_wide = rhs.0 as usize;

        Self(((lhs_wide * rhs_wide) >> 12) as u16)
    }
}

impl std::ops::Mul<f32> for UFixed16 {
    type Output = f32;

    fn mul(self, rhs: f32) -> Self::Output {
        self.to_f32() * rhs
    }
}

impl std::ops::Div<UFixed16> for UFixed16 {
    type Output = Self;

    fn div(self, rhs: UFixed16) -> Self::Output {
        Self((self.0 << 12) / rhs.0)
    }
}

impl std::ops::Div<f32> for UFixed16 {
    type Output = f32;

    fn div(self, rhs: f32) -> Self::Output {
        self.to_f32() / rhs
    }
}

impl PartialEq<f32> for UFixed16 {
    fn eq(&self, other: &f32) -> bool {
        self.to_f32().eq(other)
    }
}

impl PartialEq<Fixed16> for UFixed16 {
    fn eq(&self, other: &Fixed16) -> bool {
        if other.0 < 0 {
            return false;
        }
        
        other.0 as u16 == self.0
    }
}

impl PartialEq<UFixed16> for Fixed16 {
    fn eq(&self, other: &UFixed16) -> bool {
        if self.0 < 0 {
            return false;
        }
        
        self.0 as u16 == other.0
    }   
}

impl PartialOrd<f32> for UFixed16 {
    fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
        self.to_f32().partial_cmp(other)
    }
}

impl PartialOrd<Fixed16> for UFixed16 {
    fn partial_cmp(&self, other: &Fixed16) -> Option<std::cmp::Ordering> {
        if other.0 < 0 {
            return Some(std::cmp::Ordering::Greater);
        }

        (other.0 as u16).partial_cmp(&self.0)
    }
}

impl PartialOrd<UFixed16> for Fixed16 {
    fn partial_cmp(&self, other: &UFixed16) -> Option<std::cmp::Ordering> {
        if self.0 < 0 {
            return Some(std::cmp::Ordering::Less);
        }

        (self.0 as u16).partial_cmp(&other.0)
    }
}

impl std::ops::Neg for UFixed16 {
    type Output = Fixed16;

    fn neg(self) -> Self::Output {
        Fixed16(-(self.0 as i16))
    }
}

impl std::ops::Add<UFixed16> for Fixed16 {
    type Output = Self;

    fn add(self, rhs: UFixed16) -> Self::Output {
        Self((self.0 as i32 + rhs.0 as i32) as i16)
    }
}

impl std::ops::Sub<UFixed16> for Fixed16 {
    type Output = Self;

    fn sub(self, rhs: UFixed16) -> Self::Output {
        Self((self.0 as i32 - rhs.0 as i32) as i16)
    }
}

impl std::ops::Shl<i32> for UFixed16 {
    type Output = Self;

    fn shl(self, rhs: i32) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl std::ops::Shr<i32> for UFixed16 {
    type Output = Self;

    fn shr(self, rhs: i32) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl std::fmt::Display for UFixed16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// val should be pre-shifted left 12
const fn sqrt(val: i32) -> u32 {
    if val == 0 || val == -1 {
        return 0;
    }

    let zeros = val.abs().leading_zeros() & !1;
    let index = if zeros < 0x18 {
        val as usize >> (0x18 - zeros)
    } else {
        (val as usize) << (zeros - 0x18)
    };

    (SQRT_TABLE[index] << ((0x1f - zeros) >> 1)) >> 12
}

#[binrw]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Add, AddAssign, From, Into, Neg, Sub, SubAssign)]
pub struct Fixed32(pub i32);

impl Fixed32 {
    pub const fn from_f32(f: f32) -> Self {
        Self((f * 4096.0) as i32)
    }

    pub const fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
    }

    pub const fn to_radians(&self) -> f32 {
        self.to_f32() * TAU
    }

    pub const fn to_degrees(&self) -> f32 {
        self.to_radians() * 180.0 / PI
    }

    pub const fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    pub const fn sqrt(&self) -> Self {
        Self(sqrt(self.0 << 12) as i32)
    }

    pub const fn sin(&self) -> Self {
        let val = self.0 & 0x7ff;
        let sign = if (self.0 & 0x800) != 0 {
            -1
        } else {
            1
        };

        let index = if val < 0x400 {
            val as usize
        } else {
            SINE_TABLE.len() - (val - 0x400) as usize - 1
        };

        Self((SINE_TABLE[index] as i32) * sign)
    }

    pub const fn cos(&self) -> Self {
        Self(self.0 + 0x400).sin()
    }
    
    pub fn atan(&self) -> Self {
        // note: this is NOT perfectly accurate to the game because the game uses the x87 fpatan
        // instruction, which is not available on x64. however, the difference should be
        // extremely small, and emulating the fpatan instruction would be a lot of work.
        let atan = (self.0 as f64 / 4096.0).atan();
        // the game multiplies by the reciprocal of 3.14, so we'll use that value rather than
        // the PI constant
        let angle = (atan * 2048.0) / 3.14;
        Self(angle as i32)
    }
    
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }
    
    pub const fn is_positive(&self) -> bool {
        self.0 > 0
    }
    
    pub const fn is_negative(&self) -> bool {
        self.0 < 0
    }
}

impl std::convert::From<f32> for Fixed32 {
    fn from(f: f32) -> Self {
        Self::from_f32(f)
    }
}

impl std::convert::From<Fixed32> for f32 {
    fn from(f: Fixed32) -> Self {
        f.to_f32()
    }
}

impl std::convert::From<Fixed16> for Fixed32 {
    fn from(f: Fixed16) -> Self {
        Self(f.0 as i32)
    }
}

impl std::convert::From<UFixed16> for Fixed32 {
    fn from(f: UFixed16) -> Self {
        Self(f.0 as i32)
    }
}

impl std::ops::Add<f32> for Fixed32 {
    type Output = f32;

    fn add(self, rhs: f32) -> Self::Output {
        self.to_f32() + rhs
    }
}

impl std::ops::Sub<f32> for Fixed32 {
    type Output = f32;

    fn sub(self, rhs: f32) -> Self::Output {
        self.to_f32() - rhs
    }
}

impl std::ops::Mul<Fixed32> for Fixed32 {
    type Output = Self;

    fn mul(self, rhs: Fixed32) -> Self::Output {
        let lhs_wide = self.0 as isize;
        let rhs_wide = rhs.0 as isize;

        Self(((lhs_wide * rhs_wide) >> 12) as i32)
    }
}

impl std::ops::Mul<f32> for Fixed32 {
    type Output = f32;

    fn mul(self, rhs: f32) -> Self::Output {
        self.to_f32() * rhs
    }
}

impl std::ops::Div<Fixed32> for Fixed32 {
    type Output = Self;

    fn div(self, rhs: Fixed32) -> Self::Output {
        Self((self.0 << 12) / rhs.0)
    }
}

impl std::ops::Div<f32> for Fixed32 {
    type Output = f32;

    fn div(self, rhs: f32) -> Self::Output {
        self.to_f32() / rhs
    }
}

impl PartialEq<f32> for Fixed32 {
    fn eq(&self, other: &f32) -> bool {
        self.to_f32().eq(other)
    }
}

impl PartialOrd<f32> for Fixed32 {
    fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
        self.to_f32().partial_cmp(other)
    }
}

impl std::ops::Shl<i32> for Fixed32 {
    type Output = Self;

    fn shl(self, rhs: i32) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl std::ops::Shr<i32> for Fixed32 {
    type Output = Self;

    fn shr(self, rhs: i32) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl std::fmt::Display for Fixed32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Vec2 {
    pub x: Fixed32,
    pub z: Fixed32,
}

impl Vec2 {
    pub fn new<T, U>(x: T, z: U) -> Self
    where T: Into<Fixed32>,
          U: Into<Fixed32>
    {
        Self {
            x: x.into(),
            z: z.into(),
        }
    }

    pub const fn zero() -> Self {
        Self {
            x: Fixed32(0),
            z: Fixed32(0),
        }
    }

    pub const fn len(&self) -> Fixed32 {
        let x = self.x.0;
        let z = self.z.0;
        
        let (x_squared, x_overflow) = x.overflowing_mul(x);
        let (z_squared, z_overflow) = z.overflowing_mul(z);
        let (sum, sum_overflow) = x_squared.overflowing_add(z_squared);
        
        if x_overflow || z_overflow || sum_overflow {
            return Fixed32(i32::MAX);
        }
        
        Fixed32(sqrt(sum) as i32)
    }

    pub const fn rotate_y(&self, angle: Fixed32) -> Self {
        // this is the same matrix math used by the game but with extra steps that aren't relevant
        // to a rotation about the y-axis stripped out
        let x = self.x.0;
        let z = self.z.0;

        let cos_angle = angle.cos().0;
        let sin_angle = angle.sin().0;

        let m00 = cos_angle;
        let m02 = sin_angle;
        let m20 = -sin_angle;
        let m22 = cos_angle;

        let out_x = m02 * z + m00 * x;
        let out_z = m22 * z + m20 * x;

        Self {
            x: Fixed32(out_x >> 12),
            z: Fixed32(out_z >> 12),
        }
    }

    pub fn angle_between(&self, other: &Self) -> Fixed32 {
        let diff = *other - *self;

        Fixed32(if !diff.x.is_zero() {
            let ratio = diff.z / diff.x;
            let angle = ratio.atan();
            -(angle.0 + if diff.x.is_negative() { 0x800 } else { 0 }) & 0xfff
        } else {
            0x400 + if diff.z.is_positive() { 0x800 } else { 0 }
        })
    }

    pub fn saturating_sub(&self, rhs: impl Into<Self>) -> Self {
        let rhs = rhs.into();
        Self {
            x: Fixed32(self.x.0.saturating_sub(rhs.x.0)),
            z: Fixed32(self.z.0.saturating_sub(rhs.z.0)),
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::Add<(Fixed32, Fixed32)> for Vec2 {
    type Output = Self;

    fn add(self, rhs: (Fixed32, Fixed32)) -> Self::Output {
        Self {
            x: self.x + rhs.0,
            z: self.z + rhs.1,
        }
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Sub<(Fixed32, Fixed32)> for Vec2 {
    type Output = Self;

    fn sub(self, rhs: (Fixed32, Fixed32)) -> Self::Output {
        Self {
            x: self.x - rhs.0,
            z: self.z - rhs.1,
        }
    }
}

impl<T: Into<Fixed32>> std::ops::Mul<T> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        Self {
            x: self.x * rhs,
            z: self.z * rhs,
        }
    }
}

impl std::ops::Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            z: -self.z,
        }
    }
}

impl From<(Fixed32, Fixed32)> for Vec2 {
    fn from(v: (Fixed32, Fixed32)) -> Self {
        Self {
            x: v.0,
            z: v.1,
        }
    }
}

impl From<(Fixed16, Fixed16)> for Vec2 {
    fn from(v: (Fixed16, Fixed16)) -> Self {
        Self {
            x: v.0.to_32(),
            z: v.1.to_32(),
        }
    }
}

impl From<(UFixed16, UFixed16)> for Vec2 {
    fn from(v: (UFixed16, UFixed16)) -> Self {
        Self {
            x: v.0.to_32(),
            z: v.1.to_32(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt() {
        assert_eq!(sqrt(0x2a04c2a), 0x19ec);
        assert_eq!(Fixed32(0x4000).sqrt(), Fixed32(0x2000));
    }

    #[test]
    fn test_sin() {
        assert_eq!(Fixed32(0x400).sin(), Fixed32(0x1000));
        assert_eq!(Fixed32(0x13db).sin(), Fixed32(0xff9));
        assert_eq!(Fixed32(0xfdb).sin(), Fixed32(-232));
    }
    
    #[test]
    fn test_angle_between_points() {
        let zombie_pos = Vec2::new(-26346, -25364);
        let player_pos = Vec2::new(-25194, -24143);
        let angle = zombie_pos.angle_between(&player_pos);
        assert_eq!(angle, Fixed32(3565));
    }
}