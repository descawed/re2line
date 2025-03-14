use binrw::binrw;
use derive_more::{Add, AddAssign, From, Into, Neg, Sub, SubAssign};

#[binrw]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Add, AddAssign, From, Into, Neg, Sub, SubAssign)]
pub struct Fixed12(i16);

impl Fixed12 {
    pub const fn from_f32(f: f32) -> Self {
        Self((f * 4096.0) as i16)
    }

    pub const fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
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

#[binrw]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Add, AddAssign, From, Into, Sub, SubAssign)]
pub struct UFixed12(u16);

impl UFixed12 {
    pub const fn from_f32(f: f32) -> Self {
        Self((f * 4096.0) as u16)
    }

    pub const fn to_f32(&self) -> f32 {
        self.0 as f32 / 4096.0
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

impl PartialOrd<f32> for UFixed12 {
    fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
        self.to_f32().partial_cmp(other)
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