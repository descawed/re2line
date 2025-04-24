use std::sync::LazyLock;

use enum_map::{EnumMap, enum_map};
use re2shared::rng::RollType;

pub const ZOMBIE_ONE_SHOT_STAGGER_THRESHOLD: u8 = 0x17;

const ZOMBIE_SPEED_INDEXES: [u8; 8] = [0, 2, 0, 2, 0, 2, 2, 0];

const INITIAL_SEED: u16 = 0x6ca4; // technically 0xd2706ca4, but the high 16 bits are ignored

pub const fn roll(seed: u16) -> u16 {
    let high = seed.overflowing_mul(2).0 >> 8;
    let low = seed.overflowing_add(high).0 & 0xff;
    (high << 8) | low
}

const RNG_SEQUENCE: [u16; 24312] = {
    let mut sequence = [0u16; 24312];
    sequence[0] = INITIAL_SEED;

    let mut value = roll(INITIAL_SEED);
    let mut i = 1;
    while value != INITIAL_SEED {
        sequence[i] = value;
        i += 1;
        value = roll(value);
    }

    sequence
};

const ZOMBIE_HEALTHS1: [i16; 16] = [
    0x46,     0x54,     0x76,     0x41,
    0x32,     0x55,     0x30,     0x41,
    0x28,     0x49,     0x45,     0x38,
    0x46,     0x37,     0x48,     0x37,
];

const ZOMBIE_HEALTHS2: [i16; 16] = [
     0x27,     0x0D,     0x1B,     0x27,
     0x36,     0x1B,     0x27,     0x55,
     0x1B,     0x27,     0x1B,     0x1B,
     0x27,     0x1B,     0x27,     0x1B,
];

const IVY_HEALTHS1: [i16; 16] = [
     0x61,     0x77,     0x61,     0x5A,
     0x61,     0x5A,     0x77,     0x61,
     0x5A,     0x5A,     0x59,     0x77,
     0x45,     0x5A,     0x59,     0x77,
];

const IVY_HEALTHS2: [i16; 16] = [
     0x59,     0x45,     0x59,     0x4F,
     0x63,     0x45,     0x4F,     0x3B,
     0x45,     0x3B,     0x3B,     0x45,
     0x3B,     0x3B,     0x3B,     0x45,
];

const IVY_TENTACLE_SETS: [[u8; 4]; 4] = [
    [2, 3, 5, 6],
    [0, 1, 4, 7],
    [0, 3, 6, 7],
    [1, 2, 6, 7],
];

pub const fn roll8(seed: u16) -> u8 {
    (roll(seed) & 0xff) as u8
}

// this works because we record the seed value before it's rolled
const fn roll_two(seed: u16) -> (u8, u8) {
    ((seed & 0xff) as u8, roll8(seed))
}

// this works because we only need the low byte of the previous seed
const fn roll_three(seed: u16) -> (u8, u8, u8) {
    let high = (seed >> 8) as u8;
    let low = (seed & 0xff) as u8;
    let prev = low.overflowing_sub(high).0;
    (prev, low, roll8(seed))
}

const fn roll_double(seed: u16, mask: u8) -> u8 {
    let (first, second) = roll_two(seed);
    (second >> (first & 3)) & mask
}

fn bool_text(value: bool) -> String {
    String::from(if value {
        "success"
    } else {
        "failure"
    })
}

fn zombie_stagger_threshold(seed: u16, base: u8) -> String {
    let threshold = (roll8(seed) & 0xf) + base;
    let description = if threshold <= ZOMBIE_ONE_SHOT_STAGGER_THRESHOLD {
        "one-shot stagger"
    } else {
        "two-shot stagger"
    };
    format!("{} ({})", threshold, description)
}

fn zombie_stagger(seed: u16) -> String {
    zombie_stagger_threshold(seed, 0x10)
}

fn zombie_stagger_hard(seed: u16) -> String {
    zombie_stagger_threshold(seed, 0x20)
}

fn zombie_stagger_reroll(seed: u16) -> String {
    bool_text(roll_double(seed, 0xff) % 3 == 0)
}

fn bit_one(seed: u16) -> String {
    bool_text(roll(seed) & 1 == 1)
}

fn not_bit_one(seed: u16) -> String {
    bool_text(roll(seed) & 1 == 0)
}

fn bit_two(seed: u16) -> String {
    bool_text(roll(seed) & 2 == 2)
}

fn not_bit_two(seed: u16) -> String {
    bool_text(roll(seed) & 2 == 0)
}

fn and_three_zero(seed: u16) -> String {
    bool_text(roll(seed) & 3 == 0)
}

fn and_three_not_zero(seed: u16) -> String {
    bool_text(roll(seed) & 3 != 0)
}

fn and_two_zero(seed: u16) -> String {
    bool_text(roll(seed) & 2 == 0)
}

fn zombie_appearance(seed: u16) -> String {
    let (first, second, third) = roll_three(seed);
    format!("{}", (third as usize + ((second as usize) << (first as usize & 3))) % 3 + 1)
}

fn zombie_appearance2(seed: u16) -> String {
    format!("{}", roll8(seed) % 3 + 1)
}

fn health<T: Into<usize>>(index: T, values: &[i16]) -> String {
    let index = index.into();
    format!("{} (index {})", values[index], index)
}

fn zombie_health(seed: u16) -> String {
    health(roll_double(seed, 0xf), &ZOMBIE_HEALTHS1)
}

fn zombie_health_alt(seed: u16) -> String {
    health(roll_double(seed, 0xf), &ZOMBIE_HEALTHS2)
}

fn zombie_health2(seed: u16) -> String {
    health(roll8(seed) & 0xf, &ZOMBIE_HEALTHS1)
}

fn ivy_health1(seed: u16) -> String {
    health(roll8(seed) & 0xf, &IVY_HEALTHS1)
}

fn ivy_health2(seed: u16) -> String {
    health(roll8(seed) & 0xf, &IVY_HEALTHS2)
}

fn ivy_health_bonus(seed: u16) -> String {
    format!("{}", roll8(seed) & 3)
}

fn ivy_tentacle_set(seed: u16) -> String {
    let index = (roll8(seed) & 3) as usize;
    let set = &IVY_TENTACLE_SETS[index];
    format!("{:?} (index {})", set, index)
}

fn ivy_ambush(seed: u16) -> String {
    format!("{}", roll8(seed) & 3)
}

fn zombie_knockdown93(seed: u16) -> String {
    bool_text(roll_double(seed, 0xf) != 2)
}

fn zombie_knockdown_speed(seed: u16) -> String {
    format!("{}", roll8(seed) & 3)
}

fn zombie_knockdown87(seed: u16) -> String {
    bool_text(roll_double(seed, 7) != 0)
}

fn zombie_speed(seed: u16) -> String {
    let index = roll_double(seed, 7) as usize;
    String::from(if ZOMBIE_SPEED_INDEXES[index] == 0 {
        "fast"
    } else {
        "slow"
    })
}

fn licker_health(seed: u16) -> String {
    format!("index {}", (roll8(seed) & 0xf) * 2)
}

fn licker_jump37(seed: u16) -> String {
    bool_text(roll8(seed) & 7 < 3)
}

fn licker_jump25(seed: u16) -> String {
    bool_text(roll8(seed) & 7 < 2)
}

fn licker_jump62(seed: u16) -> String {
    bool_text(roll8(seed) & 7 < 5)
}

fn licker_jump_or_lick(seed: u16) -> String {
    String::from(if roll8(seed) & 7 < 6 {
        "jump"
    } else {
        "lick"
    })
}

#[derive(Debug)]
pub struct RollDescription {
    description: &'static str,
    result_formatter: Option<fn(u16) -> String>,
}

impl RollDescription {
    pub const fn simple(description_template: &'static str) -> Self {
        RollDescription {
            description: description_template,
            result_formatter: None,
        }
    }

    pub const fn new(description_template: &'static str, result_formatter: fn(u16) -> String) -> Self {
        RollDescription {
            description: description_template,
            result_formatter: Some(result_formatter),
        }
    }
    
    pub fn describe(&self, seed: u16, subject: Option<&str>) -> String {
        let description = if let Some(subject) = subject {
            format!("{} {}", subject, self.description)
        } else {
            self.description.to_string()
        };
        
        if let Some(formatter) = &self.result_formatter {
            format!("{}: {}", description, formatter(seed))
        } else {
            description
        }
    }
}

pub static ROLL_DESCRIPTIONS: LazyLock<EnumMap<RollType, RollDescription>> = LazyLock::new(|| {
    enum_map! {
        RollType::Script => RollDescription::simple("Script rolled RNG"),
        RollType::ZombieStaggerThreshold => RollDescription::new("rolled for stagger threshold (16-31)", zombie_stagger),
        RollType::ZombieStaggerThresholdHard => RollDescription::new("rolled for stagger threshold (32-47)", zombie_stagger_hard),
        RollType::ZombieStaggerThresholdReroll => RollDescription::new("rolled to re-roll stagger threshold (33%)", zombie_stagger_reroll),
        RollType::ZombieAppearance => RollDescription::new("rolled for random appearance", zombie_appearance),
        RollType::ZombieAppearance2 => RollDescription::new("rolled for random appearance", zombie_appearance2),
        RollType::AltZombieAppearance => RollDescription::new("rolled for random appearance (50%)", bit_one),
        RollType::AltZombieAppearance2 => RollDescription::new("rolled for random appearance (50%)", not_bit_one),
        RollType::ZombieHealth => RollDescription::new("rolled for health", zombie_health),
        RollType::ZombieHealthAlt => RollDescription::new("rolled for health", zombie_health_alt),
        RollType::ZombieHealth2 => RollDescription::new("rolled for health", zombie_health2),
        RollType::ZombieLunge50 => RollDescription::new("rolled to lunge (50%)", not_bit_one),
        RollType::ZombieLunge50NotZero => RollDescription::new("rolled to lunge (50%)", bit_one),
        RollType::ZombieLunge25 => RollDescription::new("rolled to lunge (25%)", and_three_zero),
        RollType::DestinationBlock => RollDescription::simple("rolled for destination block"),
        RollType::ZombieRaiseArms => RollDescription::new("rolled to raise arms (25% + player must be within 5000 units)", and_three_zero),
        RollType::ZombieKnockdown25 => RollDescription::new("rolled to fall down (25%)", and_three_zero),
        RollType::ZombieKnockdown93 => RollDescription::new("rolled to fall down (93.75%)", zombie_knockdown93),
        RollType::ZombieKnockdownSpeed => RollDescription::new("rolled for knockdown speed", zombie_knockdown_speed),
        RollType::ZombieKnockdown87 => RollDescription::new("rolled to fall down (87.5%)", zombie_knockdown87),
        RollType::ZombieSpeed => RollDescription::new("rolled for speed", zombie_speed),
        RollType::LickerHealth => RollDescription::new("rolled for health", licker_health),
        RollType::LickerJump25 => RollDescription::new("rolled to jump (25%)", licker_jump25),
        RollType::LickerJump37 => RollDescription::new("rolled to jump (37.5%)", licker_jump37),
        RollType::LickerJump62 => RollDescription::new("rolled to jump (62.5%)", licker_jump62),
        RollType::LickerLick50 => RollDescription::new("rolled to lick (50%)", and_two_zero),
        RollType::LickerConsiderAttack => RollDescription::new("rolled to consider attacking (75%)", and_three_not_zero),
        RollType::LickerSlash25 => RollDescription::new("rolled to slash (25%)", and_three_zero),
        RollType::LickerSlash50 => RollDescription::new("rolled to slash (50%)", not_bit_one),
        RollType::LickerThreatened50 => RollDescription::new("rolled to transition to threatened (50%)", bit_one),
        RollType::LickerLickOrJump50 => RollDescription::new("rolled to attack (lick/jump) or not (50%)", bit_two),
        RollType::LickerJump75Lick25 => RollDescription::new("rolled to jump (75%) or lick (25%)", licker_jump_or_lick),
        RollType::LickerRecoil25 => RollDescription::new("rolled to recoil (25%)", and_three_zero),
        RollType::LickerJump50LowHealth => RollDescription::new("rolled to jump (50% + player must have <= 100 HP)", bit_one),
        RollType::IvyHealth1 => RollDescription::new("rolled for health", ivy_health1),
        RollType::IvyHealth2 => RollDescription::new("rolled for health", ivy_health2),
        RollType::IvyHealthBonus => RollDescription::new("rolled for health bonus", ivy_health_bonus),
        RollType::IvyTentacleSet => RollDescription::new("rolled for tentacle set", ivy_tentacle_set),
        RollType::IvyAmbushTentacle => RollDescription::new("rolled to select ambush tentacle", ivy_ambush),
        RollType::Partial => RollDescription::simple("Partial roll in a larger series"),
        RollType::Invalid => RollDescription::simple("Invalid roll"),
    }
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence() {
        let mut seed = INITIAL_SEED;
        for _i in 0..100000 {
            seed = roll(seed);
            if seed == INITIAL_SEED {
                // println!("RNG loops after {i} iterations");
                return;
            }
        }
        panic!("RNG did not loop");
    }
}