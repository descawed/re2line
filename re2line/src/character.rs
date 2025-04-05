use crate::collision::EllipseCollider;
use crate::math::Fixed12;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum CharacterType {
    Player,
    Ally,
    Neutral,
    Enemy,
}

#[derive(Debug)]
pub struct Character {
    type_: CharacterType,
    shape: EllipseCollider,
    angle: Fixed12,
}