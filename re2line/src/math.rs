use binrw::binrw;
use derive_more::{Add, AddAssign, From, Into, Neg, Sub, SubAssign};

#[binrw]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Add, AddAssign, From, Into, Neg, Sub, SubAssign)]
pub struct Fixed12(pub i16);

impl Fixed12 {
    pub const fn from_f32(f: f32) -> Self {
        Self((f * 4096.0) as i16)
    }

    pub const fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
    }
    
    pub const fn to_radians(&self) -> f32 {
        self.to_f32() * std::f32::consts::PI * 2.0
    }
    
    pub const fn to_degrees(&self) -> f32 {
        self.to_radians() * 180.0 / std::f32::consts::PI
    }
    
    pub const fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    pub const fn unsigned_abs(&self) -> UFixed12 {
        UFixed12(self.0.unsigned_abs())
    }
}

impl std::convert::From<f32> for Fixed12 {
    fn from(f: f32) -> Self {
        Self::from_f32(f)
    }
}

impl std::convert::From<Fixed12> for f32 {
    fn from(f: Fixed12) -> Self {
        f.to_f32()
    }
}

impl std::ops::Add<f32> for Fixed12 {
    type Output = f32;

    fn add(self, rhs: f32) -> Self::Output {
        self.to_f32() + rhs
    }
}

impl std::ops::Sub<f32> for Fixed12 {
    type Output = f32;

    fn sub(self, rhs: f32) -> Self::Output {
        self.to_f32() - rhs
    }
}

impl std::ops::Mul<Fixed12> for Fixed12 {
    type Output = Self;

    fn mul(self, rhs: Fixed12) -> Self::Output {
        let lhs_wide = self.0 as isize;
        let rhs_wide = rhs.0 as isize;

        Self(((lhs_wide * rhs_wide) >> 12) as i16)
    }
}

impl std::ops::Mul<f32> for Fixed12 {
    type Output = f32;

    fn mul(self, rhs: f32) -> Self::Output {
        self.to_f32() * rhs
    }
}

impl std::ops::Div<Fixed12> for Fixed12 {
    type Output = Self;

    fn div(self, rhs: Fixed12) -> Self::Output {
        Self((self.0 / rhs.0) << 12)
    }
}

impl std::ops::Div<f32> for Fixed12 {
    type Output = f32;

    fn div(self, rhs: f32) -> Self::Output {
        self.to_f32() / rhs
    }
}

impl PartialEq<f32> for Fixed12 {
    fn eq(&self, other: &f32) -> bool {
        self.to_f32().eq(other)
    }
}

impl PartialOrd<f32> for Fixed12 {
    fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
        self.to_f32().partial_cmp(other)
    }
}

impl std::ops::Shl<i32> for Fixed12 {
    type Output = Self;
    
    fn shl(self, rhs: i32) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl std::ops::Shr<i32> for Fixed12 {
    type Output = Self;

    fn shr(self, rhs: i32) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl std::fmt::Display for Fixed12 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[binrw]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Add, AddAssign, From, Into, Sub, SubAssign)]
pub struct UFixed12(pub u16);

impl UFixed12 {
    pub const fn from_f32(f: f32) -> Self {
        Self((f * 4096.0) as u16)
    }

    pub const fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
    }

    pub const fn sqrt(&self) -> Self {
        Self(self.0.isqrt())
    }
}

impl std::convert::From<f32> for UFixed12 {
    fn from(f: f32) -> Self {
        Self::from_f32(f)
    }
}

impl std::convert::From<UFixed12> for f32 {
    fn from(f: UFixed12) -> Self {
        f.to_f32()
    }
}

impl std::ops::Add<f32> for UFixed12 {
    type Output = f32;

    fn add(self, rhs: f32) -> Self::Output {
        self.to_f32() + rhs
    }
}

impl std::ops::Sub<f32> for UFixed12 {
    type Output = f32;

    fn sub(self, rhs: f32) -> Self::Output {
        self.to_f32() - rhs
    }
}

impl std::ops::Mul<UFixed12> for UFixed12 {
    type Output = Self;

    fn mul(self, rhs: UFixed12) -> Self::Output {
        let lhs_wide = self.0 as usize;
        let rhs_wide = rhs.0 as usize;

        Self(((lhs_wide * rhs_wide) >> 12) as u16)
    }
}

impl std::ops::Mul<f32> for UFixed12 {
    type Output = f32;

    fn mul(self, rhs: f32) -> Self::Output {
        self.to_f32() * rhs
    }
}

impl std::ops::Div<UFixed12> for UFixed12 {
    type Output = Self;

    fn div(self, rhs: UFixed12) -> Self::Output {
        Self((self.0 / rhs.0) << 12)
    }
}

impl std::ops::Div<f32> for UFixed12 {
    type Output = f32;

    fn div(self, rhs: f32) -> Self::Output {
        self.to_f32() / rhs
    }
}

impl PartialEq<f32> for UFixed12 {
    fn eq(&self, other: &f32) -> bool {
        self.to_f32().eq(other)
    }
}

impl PartialEq<Fixed12> for UFixed12 {
    fn eq(&self, other: &Fixed12) -> bool {
        if other.0 < 0 {
            return false;
        }
        
        other.0 as u16 == self.0
    }
}

impl PartialEq<UFixed12> for Fixed12 {
    fn eq(&self, other: &UFixed12) -> bool {
        if self.0 < 0 {
            return false;
        }
        
        self.0 as u16 == other.0
    }   
}

impl PartialOrd<f32> for UFixed12 {
    fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
        self.to_f32().partial_cmp(other)
    }
}

impl PartialOrd<Fixed12> for UFixed12 {
    fn partial_cmp(&self, other: &Fixed12) -> Option<std::cmp::Ordering> {
        if other.0 < 0 {
            return Some(std::cmp::Ordering::Greater);
        }

        (other.0 as u16).partial_cmp(&self.0)
    }
}

impl PartialOrd<UFixed12> for Fixed12 {
    fn partial_cmp(&self, other: &UFixed12) -> Option<std::cmp::Ordering> {
        if self.0 < 0 {
            return Some(std::cmp::Ordering::Less);
        }

        (self.0 as u16).partial_cmp(&other.0)
    }
}

impl std::ops::Neg for UFixed12 {
    type Output = Fixed12;

    fn neg(self) -> Self::Output {
        Fixed12(-(self.0 as i16))
    }
}

impl std::ops::Add<UFixed12> for Fixed12 {
    type Output = Self;

    fn add(self, rhs: UFixed12) -> Self::Output {
        Self((self.0 as i32 + rhs.0 as i32) as i16)
    }
}

impl std::ops::Sub<UFixed12> for Fixed12 {
    type Output = Self;

    fn sub(self, rhs: UFixed12) -> Self::Output {
        Self((self.0 as i32 - rhs.0 as i32) as i16)
    }
}

impl std::ops::Shl<i32> for UFixed12 {
    type Output = Self;

    fn shl(self, rhs: i32) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl std::ops::Shr<i32> for UFixed12 {
    type Output = Self;

    fn shr(self, rhs: i32) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl std::fmt::Display for UFixed12 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Vec2 {
    pub x: Fixed12,
    pub z: Fixed12,
}

impl Vec2 {
    pub fn new<T, U>(x: T, z: U) -> Self
    where T: Into<Fixed12>,
          U: Into<Fixed12>
    {
        Self {
            x: x.into(),
            z: z.into(),
        }
    }

    pub const fn zero() -> Self {
        Self {
            x: Fixed12(0),
            z: Fixed12(0),
        }
    }

    pub fn len(&self) -> UFixed12 {
        let x_abs = self.x.unsigned_abs();
        let z_abs = self.z.unsigned_abs();
        (x_abs * x_abs + z_abs * z_abs).sqrt()
    }

    pub fn saturating_add(&self, rhs: impl Into<Self>) -> Self {
        let rhs = rhs.into();
        Self {
            x: Fixed12(self.x.0.saturating_add(rhs.x.0)),
            z: Fixed12(self.z.0.saturating_add(rhs.z.0)),
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

impl std::ops::Add<(Fixed12, Fixed12)> for Vec2 {
    type Output = Self;

    fn add(self, rhs: (Fixed12, Fixed12)) -> Self::Output {
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

impl std::ops::Sub<(Fixed12, Fixed12)> for Vec2 {
    type Output = Self;

    fn sub(self, rhs: (Fixed12, Fixed12)) -> Self::Output {
        Self {
            x: self.x - rhs.0,
            z: self.z - rhs.1,
        }
    }
}

impl<T: Into<Fixed12>> std::ops::Mul<T> for Vec2 {
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

impl From<(Fixed12, Fixed12)> for Vec2 {
    fn from(v: (Fixed12, Fixed12)) -> Self {
        Self {
            x: v.0,
            z: v.1,
        }
    }
}