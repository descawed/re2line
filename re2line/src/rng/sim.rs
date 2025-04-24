use std::collections::HashMap;
use std::ops::Range;

use super::*;

const MIN_FRAMES: usize = 105;
const MAX_FRAMES: usize = 180;
const MAX_HANDGUN_DAMAGE: usize = 16;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Appearance {
    YellowShirt = 1,
    BlackShirt = 2,
    PoliceOfficer = 3,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Animation {
    SharpEnd = 18,
    HeadsDown = 19,
    Rapid = 20,
}

impl Animation {
    const fn num_frames(self) -> usize {
        match self {
            Animation::SharpEnd => 67,
            Animation::HeadsDown => 88,
            Animation::Rapid => 38,
        }
    }

    const fn is_spray_frame(self, i: usize) -> bool {
        match self {
            Animation::SharpEnd => matches!(i, 20 | 45),
            Animation::HeadsDown => matches!(i, 21 | 45 | 70),
            Animation::Rapid => matches!(i, 12 | 22),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BusManipScenario {
    pub zombie3_appearance: Appearance,
    pub zombie4_appearance: Appearance,
    pub zombie5_appearance: Appearance,
    pub zombie6_appearance: Appearance,
    pub zombie3_animation: Animation,
    pub zombie4_animation: Animation,
    pub zombie5_animation: Animation,
    pub zombie6_animation: Animation,
    pub zombie6_moan: bool,
    pub zombie4_moan: bool,
}

#[derive(Debug, Clone)]
struct Zombie {
    pub appearance: Appearance,
    pub animation: Animation,
    pub anim_frames_remaining: usize,
    pub eating_frames_remaining: usize,
}

impl Zombie {
    const fn new(appearance: Appearance, animation: Animation, eating_frames: usize) -> Self {
        Self {
            appearance,
            animation,
            anim_frames_remaining: animation.num_frames(),
            eating_frames_remaining: eating_frames,
        }
    }

    fn tick(&mut self) {
        self.anim_frames_remaining -= 1;
        if self.anim_frames_remaining == 0 {
            self.anim_frames_remaining = self.animation.num_frames();
            self.animation = animation(&mut 0);
        }
    }
}

#[derive(Debug)]
struct GameEnvironment {
    pub zombies: [Zombie; 4],
    pub frames_elapsed: usize,
    pub rng_index: usize,
}

impl GameEnvironment {
    const fn new(rng_start_index: usize) -> Self {
        let mut i = rng_start_index;

        // skip all activity for the dead zombie
        i += 8;

        // skip health, speed, and stagger for remaining zombies
        i += 5;
        let zombie3_appearance = appearance(&mut i);

        i += 5;
        let zombie4_appearance = appearance(&mut i);

        i += 5;
        let zombie5_appearance = appearance(&mut i);

        i += 5;
        let zombie6_appearance = appearance(&mut i);

        let zombie3_animation = animation(&mut i);
        let zombie4_animation = animation(&mut i);
        let zombie5_animation = animation(&mut i);
        let zombie6_animation = animation(&mut i);

        Self {
            zombies: [
                Zombie::new(zombie3_appearance, zombie3_animation, 0),
                Zombie::new(zombie4_appearance, zombie4_animation, 0),
                Zombie::new(zombie5_appearance, zombie5_animation, 0),
                Zombie::new(zombie6_appearance, zombie6_animation, 0),
            ],
            frames_elapsed: 0,
            rng_index: i,
        }
    }

    const fn animation(&mut self) -> Animation {
        animation(&mut self.rng_index)
    }

    fn tick(&mut self) {
        self.frames_elapsed += 1;
        for zombie in &mut self.zombies {
            zombie.anim_frames_remaining -= 1;
            if zombie.anim_frames_remaining == 0 {
                zombie.anim_frames_remaining = zombie.animation.num_frames();
                zombie.animation = animation(&mut 0);
            }
        }
    }
}

const fn rng(i: usize) -> usize {
    (RNG_SEQUENCE[(i + 1) % RNG_SEQUENCE.len()] & 0xff) as usize
}

const fn bit(i: usize, bit: usize) -> bool {
    (rng(i) & bit) != 0
}

const fn appearance(i: &mut usize) -> Appearance {
    let j = *i;
    *i = j + 3;
    match (rng(j + 2) + (rng(j + 1) << (rng(j) & 3))) % 3 + 1 {
        1 => Appearance::YellowShirt,
        2 => Appearance::BlackShirt,
        3 => Appearance::PoliceOfficer,
        _ => unreachable!(),
    }
}

const fn animation(i: &mut usize) -> Animation {
    let j = *i;
    *i = j + 1;
    match ZOMBIE_EAT_ANIMATIONS[rng(j) & 7] {
        18 => Animation::SharpEnd,
        19 => Animation::HeadsDown,
        20 => Animation::Rapid,
        _ => unreachable!(),
    }
}

const fn get_bus_shots(bus_rng_index: usize) -> (usize, usize) {
    let mut i = bus_rng_index + 4; // skip standing zombie speed and health rolls
    
    let stagger_threshold = (rng(i) & 0xf) + 0x10;
    let standing_shots = if stagger_threshold <= ZOMBIE_ONE_SHOT_STAGGER_THRESHOLD as usize {
        1usize
    } else {
        2usize
    };

    // advance past stagger roll and standing zombie appearance rolls
    i += 3;

    // roll Misty health
    let health_index = (rng(i + 1) >> (rng(i) & 3)) & 0xf;
    let health_value = ZOMBIE_HEALTHS1[health_index] as usize;
    // +15 HP due to less than 4 enemies in the room, -50% due to being a crawling zombie
    let misty_health = (health_value + 15) / 2;
    let misty_shots = misty_health.div_ceil(MAX_HANDGUN_DAMAGE);

    (misty_shots, standing_shots)
}

pub fn simulate_bus_rng() {
    for i in 500usize..600usize {
        println!("{}: {:?}", i, get_bus_shots(i));
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct ZombieCrowd(Appearance, Appearance, Appearance, Appearance, Animation, Animation, Animation, Animation);

const fn get_crowd_composition(crowd_rng_index: usize) -> ZombieCrowd {
    let mut i = crowd_rng_index;

    // skip all activity for the dead zombie
    i += 8;

    // skip health, speed, and stagger for remaining zombies
    i += 5;
    let zombie3_appearance = appearance(&mut i);

    i += 5;
    let zombie4_appearance = appearance(&mut i);

    i += 5;
    let zombie5_appearance = appearance(&mut i);

    i += 5;
    let zombie6_appearance = appearance(&mut i);

    let zombie3_animation = animation(&mut i);
    let zombie4_animation = animation(&mut i);
    let zombie5_animation = animation(&mut i);
    let zombie6_animation = animation(&mut i);
    
    ZombieCrowd(
        zombie3_appearance, zombie4_appearance, zombie5_appearance, zombie6_appearance,
        zombie3_animation, zombie4_animation, zombie5_animation, zombie6_animation,
    )
}

pub fn simulate_pre_bus_rng() {
    let mut crowds = HashMap::new();
    
    for i in 400usize..550usize {
        let crowd = get_crowd_composition(i);
        println!("{}: {:?}", i, crowd);
        *crowds.entry(crowd).or_insert(0usize) += 1;
    }
    
    println!("\n\nCrowd types by prevalence:");
    let mut crowd_list = crowds.iter().map(|(crowd, count)| (*crowd, *count)).collect::<Vec<_>>();
    crowd_list.sort_by_key(|crowd| -(crowd.1 as isize));
    for (crowd, count) in crowd_list {
        println!("\t{:?}: {}", crowd, count);
    }
}

const MIN_RUN_LENGTH: usize = 6;

fn print_bit_runs(runs: &[Vec<Range<usize>>]) {
    for (bit_index, runs) in runs.iter().enumerate() {
        let bit_value = 1usize << bit_index;
        println!("{bit_value:08b}:\n");
        if let Some(longest_run) = runs.iter().max_by_key(|run| run.len()) {
            println!("\tLongest run {} elements: {:?}\n", longest_run.len(), longest_run);
        }
        println!("\tRuns:\n");
        for run in runs {
            println!("\t\t{} elements: {:?}\n", run.len(), run);
        }
    }
}

fn print_runs(header: &str, runs: &[Range<usize>]) {
    println!("{header}:");
    if let Some(longest_run) = runs.iter().max_by_key(|run| run.len()) {
        println!("\tLongest run {} elements: {:?}", longest_run.len(), longest_run);
    }
    println!("\tRuns:");
    for run in runs {
        println!("\t\t{} elements: {:?}", run.len(), run);
    }
}

pub fn find_runs() {
    // TODO: calculate prevalence
    
    println!("1-bit runs:\n");
    for bit_index in 0..8 {
        let mut runs = Vec::new();
        let bit_value = 1usize << bit_index;
        
        let mut i = 0usize;
        while i < RNG_SEQUENCE.len() {
            if !bit(i, bit_value) {
                i += 1;
                continue;
            }
            
            // the bit function will transparently wrap around at the end of the array, so we don't
            // need to worry about a bounds check here
            let mut j = i + 1;
            while bit(j, bit_value) {
                j += 1;
            }
            if j - i >= MIN_RUN_LENGTH {
                runs.push(i..j);
            }
            i = j;
        }
        
        print_runs(&format!("{bit_value:08b}"), &runs);
    }

    println!("0-bit runs:\n");
    for bit_index in 0..8 {
        let mut runs = Vec::new();
        let bit_value = 1usize << bit_index;

        let mut i = 0usize;
        while i < RNG_SEQUENCE.len() {
            if bit(i, bit_value) {
                i += 1;
                continue;
            }

            // the bit function will transparently wrap around at the end of the array, so we don't
            // need to worry about a bounds check here
            let mut j = i + 1;
            while !bit(j, bit_value) {
                j += 1;
            }
            if j - i >= MIN_RUN_LENGTH {
                runs.push(i..j);
            }
            i = j;
        }

        print_runs(&format!("{bit_value:08b}"), &runs);
    }
    
    let mut threes = Vec::new();
    let mut i = 0usize;
    while i < RNG_SEQUENCE.len() {
        if rng(i) % 3 != 0 {
            i += 1;
            continue;
        }
        
        let mut j = i + 1;
        while rng(j) % 3 == 0 {
            j += 1;
        }
        if j - i >= MIN_RUN_LENGTH {
            threes.push(i..j);
        }
        i = j;
    }

    print_runs("Mod 3", &threes);
}

/*fn simulate_bus_rng(start: usize) -> BusManipScenario {
    let env = GameEnvironment::new(start);
    // player is now mobile. we expect the player to take at least 3.5 seconds (105 frames) to reach
    // the bus.

    BusManipScenario {
        zombie3_appearance,
        zombie4_appearance,
        zombie5_appearance,
        zombie6_appearance,
        zombie3_animation,
        zombie4_animation,
        zombie5_animation,
        zombie6_animation,
        zombie6_moan: false,
        zombie4_moan: false,
    }
}*/