#![allow(dead_code)]

use std::collections::HashMap;
use std::ops::Range;

use super::*;

const MIN_FRAMES: usize = 115;
const MAX_HANDGUN_DAMAGE: usize = 16;
const HANDGUN_QUICK_SHOT_FRAMES: usize = 11; // assumes optimal quick shooting
const MAX_BUS_SHOTS_SAVED: usize = 7 - 2;
const MAX_FRAMES: usize = MIN_FRAMES + MAX_BUS_SHOTS_SAVED * HANDGUN_QUICK_SHOT_FRAMES;
const ZOMBIE6_STATE_CHANGE: usize = 76 - 8; // 8 init frames
const ZOMBIE4_STATE_CHANGE: usize = ZOMBIE6_STATE_CHANGE + 20;
const ZOMBIE5_STATE_CHANGE: usize = ZOMBIE4_STATE_CHANGE + 50;
const ZOMBIE3_STATE_CHANGE: usize = ZOMBIE5_STATE_CHANGE + 20;
const ZOMBIE_INIT_FRAMES: usize = 1;
const ZOMBIE_RISE_FRAMES: usize = 55;
const FRAME_TIME: f32 = 1.0 / 30.0;

#[derive(Debug, Clone)]
struct BusScenario {
    pub frame_index: usize,
    pub rng_index: usize,
    pub shots: (usize, usize),
}

impl BusScenario {
    const fn num_shots(&self) -> usize {
        self.shots.0 + self.shots.1
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Appearance {
    YellowShirt = 1,
    BlackShirt = 2,
    PoliceOfficer = 3,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Animation {
    SharpEnd = 18,
    HeadsDown = 19,
    Rapid = 20,
}

impl Animation {
    const fn num_frames(self) -> usize {
        match self {
            Animation::SharpEnd => 65,
            Animation::HeadsDown => 86,
            Animation::Rapid => 36,
        }
    }

    const fn is_spray_frame(self, i: usize) -> bool {
        match self {
            Animation::SharpEnd => matches!(i, 18 | 43),
            Animation::HeadsDown => matches!(i, 19 | 43 | 68),
            Animation::Rapid => matches!(i, 10 | 20),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ZombieCrowdAppearance(Appearance, Appearance, Appearance, Appearance);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ZombieCrowd(Appearance, Appearance, Appearance, Appearance, Animation, Animation, Animation, Animation);

impl ZombieCrowd {
    const fn appearance(&self) -> ZombieCrowdAppearance {
        ZombieCrowdAppearance(self.0, self.1, self.2, self.3)
    }
}

#[derive(Debug, Clone)]
struct BusManipResult {
    pub rng_start_index: usize,
    pub crowd_composition: ZombieCrowd,
    pub expected_shots: (usize, usize),
    pub manip_start_frame: usize,
    pub manip_end_frame: usize,
    pub manipulated_shots: (usize, usize),
    pub all_results: Vec<BusScenario>,
}

impl BusManipResult {
    const fn frames_saved(&self) -> usize {
        let expected_shots = self.expected_shots.0 + self.expected_shots.1;
        let manipulated_shots = self.manipulated_shots.0 + self.manipulated_shots.1;
        let wait_frames = self.manip_start_frame - MIN_FRAMES;
        (expected_shots - manipulated_shots) * HANDGUN_QUICK_SHOT_FRAMES - wait_frames
    }

    const fn time_window(&self) -> f32 {
        // +1 because the end frame is inclusive
        (self.manip_end_frame - self.manip_start_frame + 1) as f32 * FRAME_TIME
    }
}

fn frames_to_time(frames: usize) -> String {
    // the game appears to have a bug(?) where it counts 31 frames per second and each second barrier
    // (e.g. 1.00, 2.00, etc.) is held for 2 frames
    let (seconds, sub_second) = if frames % 30 == 0 {
        (frames.div_ceil(31), 0.0f32)
    } else {
        (frames / 31, (frames % 31) as f32 / 30.0)
    };
    let sub_second = (sub_second * 100.0) as usize;
    format!("{}.{:02}", seconds, sub_second)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ZombieState {
    Init,
    Eating,
    Rising,
    Mobile,
}

#[derive(Debug, Clone)]
struct Zombie {
    pub appearance: Appearance,
    pub animation: Animation,
    pub eating_frames: usize,
    pub anim_frames_remaining: usize,
    pub state_frames_remaining: usize,
    pub state: ZombieState,
}

impl Zombie {
    const fn new(appearance: Appearance, animation: Animation, eating_frames: usize) -> Self {
        Self {
            appearance,
            animation,
            eating_frames,
            anim_frames_remaining: animation.num_frames(),
            state_frames_remaining: ZOMBIE_INIT_FRAMES,
            state: ZombieState::Init,
        }
    }

    fn tick_animation(&mut self, rng_index: &mut usize) {
        self.anim_frames_remaining -= 1;
        if self.anim_frames_remaining == 0 {
            self.animation = animation(rng_index);
            self.anim_frames_remaining = self.animation.num_frames();
        }
        
        let anim_frame_index = self.animation.num_frames() - self.anim_frames_remaining;
        if self.animation.is_spray_frame(anim_frame_index) {
            *rng_index += 1;
        }
    }
    
    fn try_moan(&mut self, rng_index: &mut usize) {
        if !bit(*rng_index, 1) {
            *rng_index += 1;
            if !bit(*rng_index, 1) {
                *rng_index += 1;
            }
        }
        
        *rng_index += 1;
    }

    fn tick(&mut self, rng_index: &mut usize) {
        // FIXME: do we need to evaluate the first frame of the next state on the same tick?
        //  this works for now but might not if we take long enough for a zombie to get into the mobile state
        match self.state {
            ZombieState::Init => {
                if self.state_frames_remaining > 0 {
                    self.state_frames_remaining -= 1;
                } else {
                    self.state = ZombieState::Eating;
                    self.state_frames_remaining = self.eating_frames;
                }
            }
            ZombieState::Eating => {
                if self.state_frames_remaining > 0 {
                    self.state_frames_remaining -= 1;
                    self.tick_animation(rng_index);
                } else {
                    self.state = ZombieState::Rising;
                    self.state_frames_remaining = ZOMBIE_RISE_FRAMES;
                    self.try_moan(rng_index);
                }
            }
            ZombieState::Rising => {
                if self.state_frames_remaining > 0 {
                    self.state_frames_remaining -= 1;
                } else {
                    self.state = ZombieState::Mobile;
                    // TODO: don't know what to do here yet
                    self.state_frames_remaining = usize::MAX;
                    *rng_index += 5;
                }
            }
            ZombieState::Mobile => (),
        }
    }
}

#[derive(Debug)]
struct GameEnvironment {
    pub rng_start_index: usize,
    pub initial_crowd: ZombieCrowd,
    pub zombies: [Zombie; 4],
    pub frames_elapsed: usize,
    pub rng_index: usize,
    pub bus_shots: Vec<BusScenario>,
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
            rng_start_index,
            initial_crowd: ZombieCrowd(
                zombie3_appearance, zombie4_appearance, zombie5_appearance, zombie6_appearance,
                zombie3_animation, zombie4_animation, zombie5_animation, zombie6_animation,
            ),
            zombies: [
                Zombie::new(zombie3_appearance, zombie3_animation, ZOMBIE3_STATE_CHANGE),
                Zombie::new(zombie4_appearance, zombie4_animation, ZOMBIE4_STATE_CHANGE),
                Zombie::new(zombie5_appearance, zombie5_animation, ZOMBIE5_STATE_CHANGE),
                Zombie::new(zombie6_appearance, zombie6_animation, ZOMBIE6_STATE_CHANGE),
            ],
            frames_elapsed: 6, // 6 frames elapse over the course of choosing appearances and animations
            rng_index: i,
            bus_shots: Vec::new(),
        }
    }

    fn tick(&mut self) {
        let start_index = self.rng_index;
        for zombie in &mut self.zombies {
            let mut rng_index = self.rng_index;
            zombie.tick(&mut rng_index);
            self.rng_index = rng_index;
        }
        
        /*if self.rng_index != start_index {
            println!("Frame {}: {} rolls", self.frames_elapsed + 2675, self.rng_index - start_index);
        }*/
        
        // always print the RNG for the first frame in the range even if the index hasn't changed
        if self.frames_elapsed >= MIN_FRAMES && (self.rng_index != start_index || self.frames_elapsed == MIN_FRAMES) {
            let bus_shots = get_bus_shots(self.rng_index);
            self.bus_shots.push(BusScenario {
                frame_index: self.frames_elapsed,
                rng_index: self.rng_index,
                shots: bus_shots,
            });
        }

        self.frames_elapsed += 1;
    }
    
    fn is_active(&self) -> bool {
        self.frames_elapsed < MAX_FRAMES
    }
    
    fn to_manip_result(mut self) -> BusManipResult {
        let result_shots = self.bus_shots.clone();
        let expected = self.bus_shots.remove(0);
        let mut manipulated_shots = expected.shots.clone();
        let mut result_num_shots = expected.num_shots();
        let mut frames_saved = 0usize;
        let mut start_frame = expected.frame_index;
        let mut end_frame = MAX_FRAMES;
        let mut found_range_end = false;
        for shots in self.bus_shots {
            let num_shots = shots.num_shots();
            if num_shots > result_num_shots {
                // after setting the current ideal number of shots, the next time the RNG changes to
                // a higher number of shots, use that as the end of the ideal range
                if !found_range_end {
                    end_frame = shots.frame_index - 1;
                    found_range_end = true;
                }
                continue;
            }
            
            if num_shots < result_num_shots {
                let wait_frames = shots.frame_index - expected.frame_index;
                let shot_frames_saved = (expected.num_shots() - num_shots) * HANDGUN_QUICK_SHOT_FRAMES;
                if shot_frames_saved > wait_frames {
                    let total_frames_saved = shot_frames_saved - wait_frames;
                    if total_frames_saved > frames_saved {
                        frames_saved = total_frames_saved;
                        start_frame = shots.frame_index;
                        result_num_shots = num_shots;
                        manipulated_shots = shots.shots;
                        found_range_end = false;
                    }
                }
            }
        }

        let mut result = BusManipResult {
            rng_start_index: self.rng_start_index,
            crowd_composition: self.initial_crowd,
            expected_shots: expected.shots,
            manip_start_frame: start_frame,
            manip_end_frame: end_frame,
            manipulated_shots,
            all_results: result_shots,
        };
        
        // cap the end frame at the last frame where we would still be saving time
        let max_end_frame = result.manip_start_frame + result.frames_saved();
        if result.manip_end_frame > max_end_frame {
            result.manip_end_frame = max_end_frame;
        }
        
        result
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

const fn get_shots_to_stagger(rng_index: usize) -> usize {
    let stagger_threshold = (rng(rng_index) & 0xf) + 0x10;
    if stagger_threshold <= ZOMBIE_ONE_SHOT_STAGGER_THRESHOLD as usize {
        1usize
    } else {
        2usize
    }
}

const fn get_bus_shots(bus_rng_index: usize) -> (usize, usize) {
    let mut i = bus_rng_index + 4; // skip standing zombie speed and health rolls
    
    let standing_shots = get_shots_to_stagger(i);

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
    
    for i in 500usize..560usize {
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

/// rng_index should be the RNG index upon entering Kendo's shop, before the 64 rolls in that
/// room are applied.
const fn simulate_gate_shot(rng_index: usize) -> usize {
    // skip over Kendo rolls and bball court rolls prior to the gate zombie's stagger threshold
    get_shots_to_stagger(rng_index + 64 + 62)
}

pub fn print_gate_shots() {
    for i in 150usize..300usize {
        println!("{}: {}", i, simulate_gate_shot(i));
    }
}

fn simulate_one_bus_manip(start: usize) -> BusManipResult {
    let mut env = GameEnvironment::new(start);
    while env.is_active() {
        env.tick();
    }
    
    env.to_manip_result()
}

pub fn simulate_bus_manip(start: usize, end: usize) {
    let mut all_results: Vec<BusManipResult> = Vec::new();
    let mut result_map: HashMap<ZombieCrowdAppearance, Vec<BusManipResult>> = HashMap::new();
    
    for i in start..end {
        let result = simulate_one_bus_manip(i);
        all_results.push(result.clone());
        
        let appearance = result.crowd_composition.appearance();
        let appearance_vec = result_map.entry(appearance).or_insert_with(Vec::new);
        appearance_vec.push(result);
    }
    
    all_results.sort_by_key(|result| result.crowd_composition);
    for result in all_results {
        let appearance = result.crowd_composition.appearance();
        let dupes = &result_map[&appearance];
        let num_dupes = dupes.len() - 1;
        /*if result.frames_saved() == 0 && dupes.iter().all(|r| r.frames_saved() == 0) {
            continue;
        }*/
        println!("{}: {:?}; frames saved: {}, window seconds: {}, window time range: {}-{}, num dupes: {}",
                 result.rng_start_index, result, result.frames_saved(), result.time_window(), frames_to_time(result.manip_start_frame), frames_to_time(result.manip_end_frame), num_dupes);
    }
}