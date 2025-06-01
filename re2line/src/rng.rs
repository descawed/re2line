use std::sync::LazyLock;

use enum_map::{EnumMap, enum_map};
use re2shared::rng::RollType;

pub mod sim;

pub const ZOMBIE_ONE_SHOT_STAGGER_THRESHOLD: u8 = 0x17;

const ZOMBIE_SPEED_INDEXES: [u8; 8] = [0, 2, 0, 2, 0, 2, 2, 0];
const ZOMBIE_SPEED_INDEXES2: [u8; 8] = [0, 2, 0, 2, 0, 2, 0, 2];

const INITIAL_SEED: u16 = 0x6ca4; // technically 0xd2706ca4, but the high 16 bits are ignored

pub const fn roll(seed: u16) -> u16 {
    let high = seed.overflowing_mul(2).0 >> 8;
    let low = seed.overflowing_add(high).0 & 0xff;
    ((high << 8) | low) & 0x7fff
}

pub const RNG_SEQUENCE: [u16; 24312] = {
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

const ZOMBIE_EAT_ANIMATIONS: [u8; 8] = [
    0x12, 0x13, 0x14, 0x12, 0x13, 0x14, 0x12, 0x13,
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

const SPIDER_HEALTHS1: [i16; 16] = [
     0x63,     0x63,     0x63,     0x63,
     0x77,     0x63,     0x63,     0x77,
     0x63,     0x63,     0x63,     0x77,
     0x63,     0x59,     0x63,     0x63,
];

const SPIDER_HEALTHS2: [i16; 16] = [
     0x4F,     0x59,     0x63,     0x4F,
     0x31,     0x59,     0x4F,     0x45,
     0x59,     0x45,     0x59,     0x59,
     0x63,     0x45,     0x31,     0x4F,
];

const IVY_TENTACLE_SETS: [[u8; 4]; 4] = [
    [2, 3, 5, 6],
    [0, 1, 4, 7],
    [0, 3, 6, 7],
    [1, 2, 6, 7],
];

const DOG_HEALTHS1: [i16; 16] = [
    0x77, 0x55, 0x55, 0x55,
    0x77, 0x46, 0x55, 0x55,
    0x46, 0x55, 0x3B, 0x46,
    0x3B, 0x55, 0x3B, 0x46,
];

const DOG_HEALTHS2: [i16; 16] = [
    0x39, 0x39, 0x52, 0x41,
    0x52, 0x41, 0x39, 0x3B,
    0x41, 0x39, 0x1E, 0x39,
    0x1E, 0x39, 0x1E, 0x45,
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

fn dog_health(seed: u16) -> String {
    health(roll8(seed) & 0xf, &DOG_HEALTHS1)
}

fn dog_health2(seed: u16) -> String {
    health(roll8(seed) & 0xf, &DOG_HEALTHS2)
}

fn zombie_animation_offset(seed: u16) -> String {
    format!("{}", roll8(seed) & 0x1f)
}

fn zombie_animation_offset16(seed: u16) -> String {
    format!("{}", roll8(seed) & 0xf)
}

fn one_in_32(seed: u16) -> String {
    bool_text(roll8(seed) & 0x1f == 0)
}

fn zombie_arm_raise_timer(seed: u16) -> String {
    format!("{}", ((roll8(seed) as usize) >> 3) + 100)
}

fn zombie_moan_choice(seed: u16) -> String {
    String::from(if roll8(seed) & 1 == 0 {
        "short"
    } else {
        "long"
    })
}

fn zombie_eating_animation(seed: u16) -> String {
    let index = (roll8(seed) & 7) as usize;
    format!("{} (index {})", ZOMBIE_EAT_ANIMATIONS[index], index)
}

fn ivy_health1(seed: u16) -> String {
    health(roll8(seed) & 0xf, &IVY_HEALTHS1)
}

fn ivy_health2(seed: u16) -> String {
    health(roll8(seed) & 0xf, &IVY_HEALTHS2)
}

fn health_bonus(seed: u16) -> String {
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

fn tentacle_animation_offset(seed: u16) -> String {
    format!("{}", roll8(seed) & 0xf)
}

fn tentacle_attach_angle(seed: u16) -> String {
    format!("{}", (roll8(seed) as u16) * 2)
}

fn dog_animation_offset1(seed: u16) -> String {
    format!("{}", roll8(seed) & 0x3f)
}

fn dog_animation_offset2(seed: u16) -> String {
    format!("{}", roll8(seed) & 0xf)
}

fn dog_animation_offset3(seed: u16) -> String {
    format!("{}", roll8(seed) & 0x1f)
}

fn spider_health1(seed: u16) -> String {
    health(roll8(seed) & 0xf, &SPIDER_HEALTHS1)
}

fn spider_health2(seed: u16) -> String {
    health(roll8(seed) & 0xf, &SPIDER_HEALTHS2)
}

fn spider_poison_3_in_32(seed: u16) -> String {
    bool_text(((1u32 << (roll8(seed) & 0xf)) & 0x340) != 0)
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

fn zombie_speed2(seed: u16) -> String {
    let index = (roll8(seed) & 7) as usize;
    String::from(if ZOMBIE_SPEED_INDEXES2[index] == 0 {
        "fast"
    } else {
        "slow"
    })
}

fn zombie_blood_spray(seed: u16) -> String {
    format!("{}", (roll8(seed) as u16) << 4)
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

fn handgun_crit(seed: u16) -> String {
    bool_text(roll_double(seed, 0xf) == 0)
}

fn spider_max_turn_time(seed: u16) -> String {
    format!("{}", (roll8(seed) + 10) & 0x3f)
}

fn spider_turn_direction(seed: u16) -> String {
    String::from(if roll8(seed) & 1 == 0 {
        "clockwise"
    } else {
        "counterclockwise"
    })
}

fn spider_max_face_time(seed: u16) -> String {
    format!("{}", (roll8(seed) & 0x1f) + 0x3c)
}

fn spider_max_pursue_time(seed: u16) -> String {
    format!("{}", roll8(seed) & 0x2e)
}

fn spider_max_leg_turn_time(seed: u16) -> String {
    format!("{}", (roll8(seed) & 0x1f) + 0x14)
}

fn spider_max_leg_attack_time(seed: u16) -> String {
    format!("{}", (roll8(seed) & 0x1f) + 10)
}

fn spider_max_idle_time(seed: u16) -> String {
    format!("{}", roll8(seed).overflowing_add(10).0 & 0x1f)
}

fn spider_pursue50(seed: u16) -> String {
    bool_text(roll8(seed) & 1 == 0)
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
        RollType::ZombieSpeed2 => RollDescription::new("rolled for speed", zombie_speed2),
        RollType::ZombieAnimationOffset => RollDescription::new("rolled for animation offset", zombie_animation_offset),
        RollType::ZombieAnimationOffset16 => RollDescription::new("rolled for animation offset", zombie_animation_offset16),
        RollType::ZombieShortMoan => RollDescription::new("rolled for short moan (3.125%)", one_in_32),
        RollType::ZombieLongMoan => RollDescription::new("rolled for long moan (3.125%)", one_in_32),
        RollType::ZombieMoanChoice => RollDescription::new("rolled for moan type (50/50)", zombie_moan_choice),
        RollType::ZombieArmRaiseTimer => RollDescription::new("rolled for arm raise timer", zombie_arm_raise_timer),
        RollType::ZombieEatingAnimation => RollDescription::new("rolled for eating animation", zombie_eating_animation),
        RollType::ZombieTryMoan => RollDescription::new("rolled to try moan (50%)", not_bit_one),
        RollType::ZombieLongMoan50 => RollDescription::new("rolled for long moan (50%)", bit_one),
        RollType::ZombieShortMoan50 => RollDescription::new("rolled for short moan (50%)", bit_one),
        RollType::ZombieEatBloodSpray => RollDescription::new("rolled for blood spray", zombie_blood_spray),
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
        RollType::LickerDrool => RollDescription::new("rolled to drool (3.125%)", one_in_32),
        RollType::IvyHealth1 => RollDescription::new("rolled for health", ivy_health1),
        RollType::IvyHealth2 => RollDescription::new("rolled for health", ivy_health2),
        RollType::HealthBonus => RollDescription::new("rolled for health bonus", health_bonus),
        RollType::IvyTentacleSet => RollDescription::new("rolled for tentacle set", ivy_tentacle_set),
        RollType::IvyAmbushTentacle => RollDescription::new("rolled to select ambush tentacle", ivy_ambush),
        RollType::TentacleAnimationOffset => RollDescription::new("rolled for animation offset", tentacle_animation_offset),
        RollType::TentacleAttachAngle => RollDescription::new("rolled for attachment angle", tentacle_attach_angle),
        RollType::SpiderHealth1 => RollDescription::new("rolled for health", spider_health1),
        RollType::SpiderHealth2 => RollDescription::new("rolled for health", spider_health2),
        RollType::SpiderPoison3In32 => RollDescription::new("rolled to poison (9.375%)", spider_poison_3_in_32),
        RollType::HandgunCrit => RollDescription::new("Handgun rolled to crit (6.25%)", handgun_crit),
        RollType::DogHealth1 => RollDescription::new("rolled for health", dog_health),
        RollType::DogHealth2 => RollDescription::new("rolled for health", dog_health2),
        RollType::DogAnimationOffset1 => RollDescription::new("rolled for animation offset", dog_animation_offset1),
        RollType::DogAnimationOffset2 => RollDescription::new("rolled for animation offset", dog_animation_offset2),
        RollType::DogAnimationOffset3 => RollDescription::new("rolled for animation offset", dog_animation_offset3),
        RollType::SpiderTurnTime => RollDescription::new("rolled for max turn time", spider_max_turn_time),
        RollType::SpiderTurnDirection => RollDescription::new("rolled for turn direction", spider_turn_direction),
        RollType::SpiderMaxFaceTime => RollDescription::new("rolled for max facing time", spider_max_face_time),
        RollType::SpiderMaxPursueTime => RollDescription::new("rolled for max pursuit time", spider_max_pursue_time),
        RollType::SpiderMaxLegTurnTime => RollDescription::new("rolled for max leg attack turn time", spider_max_leg_turn_time),
        RollType::SpiderMaxLegAttackTime => RollDescription::new("rolled for max leg attack time", spider_max_leg_attack_time),
        RollType::SpiderMaxIdleTime => RollDescription::new("rolled for max idle time", spider_max_idle_time),
        RollType::SpiderPursue50 => RollDescription::new("rolled to pursue (50%)", spider_pursue50),
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