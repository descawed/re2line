use std::ops::Mul;
use epaint::{Color32, Shape};
use re2shared::game::{AimZone, HitBounds, WeaponRange};

use crate::aot::Item;
use crate::app::{DrawParams, GameObject, ObjectType};
use crate::character::CharacterType;
use crate::math::{Fixed16, Fixed32, Vec2};
use crate::record::State;

// FIXME: we're not taking the y-axis into account

#[derive(Debug, Clone)]
pub struct WeaponAimRanges {
    pub low: WeaponRange,
    pub mid: WeaponRange,
    pub high: WeaponRange,
}

impl WeaponAimRanges {
    pub const fn new(low: WeaponRange, mid: WeaponRange, high: WeaponRange) -> Self {
        Self { low, mid, high }
    }
}

const KNIFE: WeaponAimRanges = WeaponAimRanges::new(
    WeaponRange::one(
        AimZone::Mid,
        HitBounds {
            x: 0x32,
            z: 0x1F4,
            x_size_half: 0x146,
            z_size_quarter: 0x79,
        },
    ),
    WeaponRange::one(
        AimZone::Mid,
        HitBounds {
            x: 0xFA,
            z: 0x320,
            x_size_half: 0x1AA,
            z_size_quarter: 0x79,
        },
    ),
    WeaponRange::one(
        AimZone::KnifeHigh,
        HitBounds {
            x: -500,
            z: 0x320,
            x_size_half: 0x146,
            z_size_quarter: 0x79,
        },
    ),
);

// also applies to SMG
const HANDGUN: WeaponAimRanges = WeaponAimRanges::new(
    WeaponRange::low(
        HitBounds::new(0x64, 0x3E8, 0x177),
        HitBounds::new(0x1004, 0x3E8, 0x1F4),
        HitBounds::new(0x1FA4, 0x9C4, 0x271),
    ),
    WeaponRange::mid(
        HitBounds::new(0xC8, 0x3E8, 0x177),
        HitBounds::new(0x1068, 0x3E8, 0x1F4),
        HitBounds::new(0x2008, 0x1D4C, 0x271),
    ),
    WeaponRange::high(
        HitBounds::new(0x64, 0x3E8, 0x177),
        HitBounds::new(0x1004, 0x3E8, 0x1F4),
        HitBounds::new(0x1FA4, 0x9C4, 0x271),
    ),
);

// applies to both regular magnum and custom magnum
const MAGNUM: WeaponAimRanges = WeaponAimRanges::new(
    WeaponRange::low(
        HitBounds::new(0x64, 0x3E8, 0x7D),
        HitBounds::new(0x1004, 0x3E8, 0x96),
        HitBounds::new(0x1FA4, 0x9C4, 0xAF),
    ),
    WeaponRange::mid(
        HitBounds::new(0xC8, 0x3E8, 0x7D),
        HitBounds::new(0x1068, 0x3E8, 0x96),
        HitBounds::new(0x1C20, 0x1D4C, 0xAF),
    ),
    WeaponRange::high(
        HitBounds::new(0x64, 0x3E8, 0x7D),
        HitBounds::new(0x1004, 0x3E8, 0x96),
        HitBounds::new(0x1FA4, 0x9C4, 0xAF),
    ),
);

const SHOTGUN: WeaponAimRanges = WeaponAimRanges::new(
    WeaponRange::low(
        HitBounds::new(0x64, 0x3E8, 0x1F4),
        HitBounds::new(0x1004, 0x3E8, 0x2EE),
        HitBounds::new(0x1FA4, 0x9C4, 0x3E8),
    ),
    WeaponRange::mid(
        HitBounds::new(0xC8, 0x3E8, 0x271),
        HitBounds::new(0x1068, 0x3E8, 0x36B),
        HitBounds::new(0x2008, 0x1D4C, 0x465),
    ),
    WeaponRange::high(
        HitBounds::new(0x64, 0x2EE, 0x1F4),
        HitBounds::new(0xC1C, 0x5DC, 0x2EE),
        HitBounds::new(0x238C, 0x9C4, 0x3E8),
    ),
);

const CUSTOM_SHOTGUN: WeaponAimRanges = WeaponAimRanges::new(
    WeaponRange::low(
        HitBounds::new(0x64, 0x3E8, 0x1F4),
        HitBounds::new(0x1004, 0x3E8, 0x36B),
        HitBounds::new(0x1FA4, 0x9C4, 0x465),
    ),
    WeaponRange::mid(
        HitBounds::new(0xC8, 0x3E8, 0x271),
        HitBounds::new(0x1068, 0x3E8, 0x3E8),
        HitBounds::new(0x2008, 0x1D4C, 0x4E2),
    ),
    WeaponRange::high(
        HitBounds::new(0x64, 0x2EE, 0x1F4),
        HitBounds::new(0xC1C, 0x3E8, 0x36B),
        HitBounds::new(0x1BBC, 0x9C4, 0x465),
    ),
);

const SPARKSHOT: WeaponAimRanges = WeaponAimRanges::new(
    WeaponRange::one(
        AimZone::LowNear,
        HitBounds::new(0xC8, 0xDAC, 0x1F4),
    ),
    WeaponRange::one(
        AimZone::Mid,
        HitBounds::new(0xC8, 0xDAC, 0x1F4),
    ),
    WeaponRange::one(
        AimZone::HighNear,
        HitBounds::new(0xC8, 0xDAC, 0x1F4),
    ),
);

const GATLING_GUN: WeaponAimRanges = WeaponAimRanges::new(
    WeaponRange::low(
        HitBounds::new(0x64, 0x3E8, 0x1F4),
        HitBounds::new(0x1004, 0x3E8, 0x271),
        HitBounds::new(0x1FA4, 0x9C4, 0x271),
    ),
    WeaponRange::new(
        [AimZone::Mid, AimZone::LowNear, AimZone::HighNear],
        [
            HitBounds::new(0xC8, 0x1D4C, 0x1F4),
            HitBounds::new(0xC8, 0x1D4C, 0x1F4),
            HitBounds::new(0xC8, 0x1D4C, 0x1F4),
        ],
    ),
    WeaponRange::high(
        HitBounds::new(0x64, 0x3E8, 0x177),
        HitBounds::new(0x1004, 0x3E8, 0x1F4),
        HitBounds::new(0x2008, 0x9C4, 0x271),
    ),
);

pub const fn get_weapon_aim_ranges(item: Item) -> Option<&'static WeaponAimRanges> {
    // weapons that are omitted have special logic and don't use this range system
    Some(match item {
        Item::Knife => &KNIFE,
        Item::HandgunLeon => &HANDGUN,
        Item::HandgunClaire => &HANDGUN,
        Item::CustomHandgun => &HANDGUN,
        Item::Magnum => &MAGNUM,
        Item::CustomMagnum => &MAGNUM,
        Item::Shotgun => &SHOTGUN,
        Item::CustomShotgun => &CUSTOM_SHOTGUN,
        Item::ColtSaa => &HANDGUN,
        Item::Sparkshot => &SPARKSHOT,
        Item::SubMachinegun => &HANDGUN,
        Item::GatlingGun => &GATLING_GUN,
        Item::Beretta => &HANDGUN,
        _ => return None,
    })
}

#[derive(Debug, Clone)]
pub struct WeaponRangeVisualization {
    pub weapon: Item,
    pub pos: Vec2,
    pub angle: Fixed32,
    pub aim_range: [(Vec2, Vec2); 3],
}

impl WeaponRangeVisualization {
    fn convert_bounds(bounds: &HitBounds) -> (Vec2, Vec2) {
        (
            Vec2::new(Fixed16(bounds.x), Fixed16(bounds.z)),
            Vec2::new(Fixed16(bounds.x_size_half), Fixed16(bounds.z_size_quarter)),
        )
    }

    pub fn for_state(state: &State) -> Option<Self> {
        let player = state.characters()[0].as_ref()?;
        if player.describe_state() != "Weapon" {
            return None;
        }

        let weapon = player.equipped_item()?;
        let aim_ranges = get_weapon_aim_ranges(weapon)?;

        let input = state.input_state();
        let aim_range = if input.is_forward_pressed {
            &aim_ranges.high
        } else if input.is_backward_pressed {
            &aim_ranges.low
        } else {
            &aim_ranges.mid
        };

        if aim_range.is_empty() {
            return None;
        }

        // as each enemy is considered for a hit, the size of the bounds is adjusted by the enemy's
        // size. that would be unwieldy to display, and most rooms only have one type of enemy
        // anyway, so we'll just adjust the bounds by the size of the largest enemy.
        let mut x_size = Fixed32(0);
        let mut z_size = Fixed32(0);
        for character in state.characters() {
            let Some(character) = character else {
                continue;
            };
            if character.type_() != CharacterType::Enemy {
                continue;
            }

            x_size = character.size.x.max(x_size);
            z_size = character.size.z.max(z_size);
        }

        let x_size = x_size >> 2;
        let z_size = z_size >> 2;

        let mut bounds0 = Self::convert_bounds(&aim_range.hit_bounds[0]);
        if aim_range.hit_bounds[0].has_area() {
            bounds0.1.x += x_size;
            bounds0.1.z += z_size;
        }

        let mut bounds1 = Self::convert_bounds(&aim_range.hit_bounds[1]);
        if aim_range.hit_bounds[1].has_area() {
            bounds1.1.z += z_size;
        }

        let mut bounds2 = Self::convert_bounds(&aim_range.hit_bounds[2]);
        if aim_range.hit_bounds[2].has_area() {
            bounds2.1.z += z_size;
        }

        Some(Self {
            weapon,
            pos: player.center,
            angle: player.angle,
            aim_range: [bounds0, bounds1, bounds2],
        })
    }
    
    fn bounds_contains(&self, bounds: &(Vec2, Vec2), point: Vec2) -> bool {
        if bounds.1.is_zero() {
            return false;
        }
        
        let x_radius = bounds.1.x << 1;
        let z_radius = bounds.1.z << 2;
        
        let center = Vec2::new(bounds.0.x + x_radius, -(bounds.0.z + z_radius));
        let center = center.rotate_y(self.angle);
        let center = center + self.pos;
        
        let trans_point = point - center;
        let rel_point = trans_point.rotate_y(-self.angle);
        
        rel_point.x.abs() <= x_radius && rel_point.z.abs() <= z_radius
    }

    fn bounds_shape(&self, params: &DrawParams, bounds: &(Vec2, Vec2)) -> Option<Shape> {
        if bounds.1.is_zero() {
            return None;
        }

        // FIXME: the actual game code multiplies these vectors by the player's full transform,
        //  which we aren't currently storing. I don't know if it ever has x or z rotations in
        //  practice, but we should at least confirm.
        let p1 = Vec2::new(bounds.0.x, -(bounds.0.z + (bounds.1.z << 2)));
        let p1 = p1.rotate_y(self.angle);
        let p1 = p1 + self.pos;

        let x_size = Vec2::new(bounds.1.x << 2, 0).rotate_y(self.angle);
        let z_size = Vec2::new(0, bounds.1.z << 3).rotate_y(self.angle);
        
        // FIXME: not sure if this is correct
        let p2 = p1 + x_size;
        let p3 = p1 + z_size;
        let p4 = p2 + z_size;
        
        let gui_p1 = params.transform_point(p1);
        let gui_p2 = params.transform_point(p2);
        let gui_p3 = params.transform_point(p3);
        let gui_p4 = params.transform_point(p4);
        
        // TODO: detect if enemies are in the zone and add outline

        Some(Shape::Path(epaint::PathShape::convex_polygon(
            vec![gui_p1, gui_p3, gui_p4, gui_p2],
            params.fill_color,
            params.stroke,
        )))
    }
}

impl GameObject for WeaponRangeVisualization {
    fn object_type(&self) -> ObjectType {
        ObjectType::WeaponRange
    }

    fn contains_point(&self, point: Vec2) -> bool {
        // FIXME: not accurate to game logic
        self.aim_range.iter().any(|r| self.bounds_contains(r, point))
    }

    fn name(&self) -> String {
        format!("{} range", self.weapon.name())
    }

    fn name_prefix(&self, _index: usize) -> String {
        String::new()
    }
    
    fn description(&self) -> String {
        String::new()
    }

    fn details(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = Vec::new();

        groups.push((String::from("Weapon Range"), vec![
            format!("Weapon: {} ({})", self.weapon.name(), self.weapon as u16),
        ]));
        
        for (i, bounds) in self.aim_range.iter().enumerate() {
            groups.push((format!("Bounds {}", i), vec![
                format!("X: {}", bounds.0.x),
                format!("Z: {}", bounds.0.z),
                format!("X Size: {}", bounds.1.x),
                format!("Z Size: {}", bounds.1.z),
            ]));
        }
        
        groups
    }

    fn gui_shape(&self, params: &DrawParams, _state: &State) -> Shape {
        let mut shapes = Vec::new();

        let mut params = params.clone();
        for bounds in &self.aim_range {
            if let Some(shape) = self.bounds_shape(&params, bounds) {
                shapes.push(shape);
            }
            
            params.stroke.color = params.stroke.color.mul(Color32::from_rgba_premultiplied(0xa0, 0x80, 0x80, 0xff));
        }
        
        // draw near zones over far ones
        shapes.reverse();

        Shape::Vec(shapes)
    }
}