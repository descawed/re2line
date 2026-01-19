use std::fs::File;
use std::path::Path;

use anyhow::Result;
use residat::common::Fixed32;
use residat::re2::{AnimationFrame, AnimationSet, CharacterId, Rdt, Item};

use crate::app::{GameObject, RoomId};
use crate::character::Character;
use crate::record::State;

const CAUTION_THRESHOLD: i16 = 100;
const DANGER_THRESHOLD: i16 = 20;

const WATER_SLOWDOWN_THRESHOLD: Fixed32 = Fixed32(-500);
const WATER_SLOWDOWN: Fixed32 = Fixed32(3 << 12);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationId {
    Model(usize),
    Weapon(usize),
    Room(usize),
}

// known animations for different scenarios
pub const ANIM_WALK: AnimationId = AnimationId::Weapon(0);
pub const ANIM_RUN: AnimationId = AnimationId::Weapon(1);
pub const ANIM_IDLE: AnimationId = AnimationId::Weapon(2);
pub const ANIM_WALK_CAUTION: AnimationId = AnimationId::Weapon(3);
pub const ANIM_RUN_CAUTION: AnimationId = AnimationId::Weapon(4);
pub const ANIM_IDLE_CAUTION: AnimationId = AnimationId::Weapon(5);
pub const ANIM_WALK_DANGER: AnimationId = AnimationId::Weapon(6);
pub const ANIM_RUN_DANGER: AnimationId = AnimationId::Weapon(7);
pub const ANIM_IDLE_DANGER: AnimationId = AnimationId::Weapon(8);
pub const ANIM_RAISE_WEAPON: AnimationId = AnimationId::Weapon(9);
pub const ANIM_SHOOT_FORWARD: AnimationId = AnimationId::Weapon(10);
pub const ANIM_AIM_FORWARD: AnimationId = AnimationId::Weapon(11);
pub const ANIM_SHOOT_HIGH: AnimationId = AnimationId::Weapon(12);
pub const ANIM_AIM_HIGH: AnimationId = AnimationId::Weapon(13);
pub const ANIM_SHOOT_LOW: AnimationId = AnimationId::Weapon(14);
pub const ANIM_AIM_LOW: AnimationId = AnimationId::Weapon(15);
pub const ANIM_RELOAD: AnimationId = AnimationId::Weapon(16);
// at least some weapons have an 18 but I don't know what it is yet. haven't seen any reference to 17.

pub const ANIM_BACK_UP: AnimationId = AnimationId::Model(0);
pub const ANIM_BACK_UP_THREATENED: AnimationId = AnimationId::Model(1);
pub const ANIM_DIE: AnimationId = AnimationId::Model(2);
pub const ANIM_QUICK_RECOIL: AnimationId = AnimationId::Model(3);
pub const ANIM_HIT_FROM_BEHIND: AnimationId = AnimationId::Model(4);
pub const ANIM_RECOIL_FROM_ATTACK: AnimationId = AnimationId::Model(5);
pub const ANIM_PICKUP_KNEEL: AnimationId = AnimationId::Model(6);
pub const ANIM_START_PUSH: AnimationId = AnimationId::Model(7);
pub const ANIM_PUSH: AnimationId = AnimationId::Model(8);
pub const ANIM_BACK_UP_DANGER: AnimationId = AnimationId::Model(9);

pub const ANIM_START_ASCEND_STAIRS: AnimationId = AnimationId::Room(0);
pub const ANIM_ASCEND_STAIRS: AnimationId = AnimationId::Room(1);
pub const ANIM_FINISH_ASCEND_STAIRS: AnimationId = AnimationId::Room(2);
pub const ANIM_START_DESCEND_STAIRS: AnimationId = AnimationId::Room(3);
pub const ANIM_DESCEND_STAIRS: AnimationId = AnimationId::Room(4);
pub const ANIM_FINISH_DESCEND_STAIRS: AnimationId = AnimationId::Room(5);
pub const ANIM_CLIMB_UP: AnimationId = AnimationId::Room(6);
pub const ANIM_JUMP_DOWN: AnimationId = AnimationId::Room(7);

const STAND_TURN_RATES: [Fixed32; 3] = [Fixed32(0x50), Fixed32(0x30), Fixed32(0x20)];
const WALK_TURN_RATES: [Fixed32; 3] = [Fixed32(0x28), Fixed32(0x20), Fixed32(0x10)];
const RUN_TURN_RATES: [Fixed32; 3] = [Fixed32(0x38), Fixed32(0x30), Fixed32(0x18)];
const BACK_UP_TURN_RATES: [Fixed32; 3] = [Fixed32(0x28), Fixed32(0x10), Fixed32(0x10)];
const AIM_TURN_RATE: Fixed32 = Fixed32(0x40);
const SHOOT_TURN_RATE: Fixed32 = Fixed32(8); // doesn't apply to the knife

const IDLE_ANIMATIONS: [AnimationId; 3] = [ANIM_IDLE, ANIM_IDLE_CAUTION, ANIM_IDLE_DANGER];
const WALK_ANIMATIONS: [AnimationId; 3] = [ANIM_WALK, ANIM_WALK_CAUTION, ANIM_WALK_DANGER];
const RUN_ANIMATIONS: [AnimationId; 3] = [ANIM_RUN, ANIM_RUN_CAUTION, ANIM_RUN_DANGER];
const BACK_UP_ANIMATIONS: [AnimationId; 3] = [ANIM_BACK_UP, ANIM_BACK_UP_DANGER, ANIM_BACK_UP_DANGER];

#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    animation_id: Option<AnimationId>,
    frame_index: usize,
    room_id: RoomId,
    character_id: CharacterId,
    weapon_id: Item,
    model_animations: AnimationSet,
    weapon_animations: AnimationSet,
}

impl AnimationPlayer {
    pub const fn empty() -> Self {
        Self {
            animation_id: None,
            frame_index: 0,
            room_id: RoomId::zero(),
            character_id: CharacterId::Unknown,
            weapon_id: Item::Empty,
            model_animations: AnimationSet::empty(),
            weapon_animations: AnimationSet::empty(),
        }
    }

    fn clear_animation(&mut self) {
        self.animation_id = None;
        self.frame_index = 0;
    }

    fn load_model(&mut self, state: &State, game_dir: &Path) -> Result<()> {
        let model_path = game_dir.join(format!("pl{}/PLD/PL{:02X}.PLD", if state.is_claire_scenario() { '1' } else { '0' }, self.character_id as u8));
        let model_file = File::open(model_path)?;
        self.model_animations = AnimationSet::read_plw(model_file)?;

        if matches!(self.animation_id, Some(AnimationId::Model(_))) {
            self.clear_animation();
        }

        Ok(())
    }

    fn load_weapon(&mut self, state: &State, game_dir: &Path) -> Result<()> {
        let weapon_path = game_dir.join(format!("pl{}/PLD/PL{:02X}W{:02X}.PLW", if state.is_claire_scenario() { '1' } else { '0' }, self.character_id as u8, self.weapon_id as u16));
        let weapon_file = File::open(weapon_path)?;
        self.weapon_animations = AnimationSet::read_plw(weapon_file)?;

        if matches!(self.animation_id, Some(AnimationId::Weapon(_))) {
            self.clear_animation();
        }

        Ok(())
    }

    fn select_animation<'a, 'b: 'a>(&'a self, character: &Character, state: &State, rdt: &'b Rdt) -> &'a [AnimationFrame] {
        match self.animation_id {
            Some(AnimationId::Model(index)) => &self.model_animations.animations()[index],
            Some(AnimationId::Weapon(index)) => &self.weapon_animations.animations()[index],
            Some(AnimationId::Room(index)) => {
                let room_animations = rdt.animation_sets();

                let test_bit = if state.is_4th_survivor() {
                    0x80000000u32
                } else if state.is_ex_battle() {
                    match character.id.base_id() {
                        CharacterId::Leon | CharacterId::Chris | CharacterId::Hunk | CharacterId::Tofu | CharacterId::Sherry => 1,
                        CharacterId::Claire | CharacterId::Ada => 2,
                        _ => 0,
                    }
                } else {
                    1
                };

                for animation_set in room_animations {
                    if animation_set.character_mask() & test_bit != 0 {
                        return &animation_set.animations()[index];
                    }
                }

                &[]
            }
            None => &[],
        }
    }

    pub fn update(&mut self, character: &mut Character, state: &State, rdt: &Rdt, game_dir: &Path) -> Result<()> {
        if self.room_id != state.room_id() {
            self.room_id = state.room_id();
            if matches!(self.animation_id, Some(AnimationId::Room(_))) {
                self.clear_animation();
            }
        }

        if self.character_id != character.id {
            self.character_id = character.id;
            self.load_model(state, game_dir)?;
        }

        let weapon = character.equipped_item().unwrap_or(Item::Empty);
        if self.weapon_id != weapon {
            self.weapon_id = weapon;
            self.load_weapon(state, game_dir)?;
        }

        let move_type = if character.current_health() > CAUTION_THRESHOLD {
            0usize
        } else if character.current_health() > DANGER_THRESHOLD {
            1usize
        } else {
            2usize
        };

        let inputs = state.input_state();

        let (animation_id, turn_rate) = match character.state {
            [0x01, 0x00, _, _] => (Some(IDLE_ANIMATIONS[move_type]), Fixed32(0)),
            [0x01, 0x01, _, _] => (Some(WALK_ANIMATIONS[move_type]), WALK_TURN_RATES[move_type]),
            [0x01, 0x02, _, _] => (Some(RUN_ANIMATIONS[move_type]), RUN_TURN_RATES[move_type]),
            // intentionally ignoring the possibility of the "threatened" animation here as we have minimal reactivity to enemies anyway
            [0x01, 0x03, _, _] => (Some(BACK_UP_ANIMATIONS[move_type]), BACK_UP_TURN_RATES[move_type]),
            [0x01, 0x04, _, _] => (None, STAND_TURN_RATES[move_type]),
            // FIXME: motion in the aiming state does not appear to follow the normal rules and needs special handling
            // [0x01, 0x05, _, _]
            // animation velocity is not applied for item pick up
            // [0x01, 0x06, _, _]
            // FIXME: we don't track the necessary field to know whether we're going up or down the stairs
            // [0x01, 0x07, 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07, _] => ,
            // FIXME: motion during climbing and pushing is not based on the animation
            // [0x01, 0x08, _, _] =>
            // [0x01, 0x09, _, _] =>
            // [0x01, 0x0A, _, _] =>
            _ => (None, Fixed32(0)),
        };

        if inputs.is_right_pressed {
            character.angle += turn_rate;
        }
        if inputs.is_left_pressed {
            character.angle -= turn_rate;
        }

        if animation_id.is_none() {
            self.clear_animation();
            return Ok(());
        }

        if self.animation_id == animation_id {
            self.frame_index += 1;
        } else {
            self.animation_id = animation_id;
            self.frame_index = 0;
        }

        let animation = self.select_animation(character, state, rdt);
        if animation.is_empty() {
            self.clear_animation();
            return Ok(());
        }

        // need to use local due to reference shenanigans
        let frame_index = self.frame_index % animation.len();
        let mut velocity = animation[frame_index].speed();
        if frame_index > 0 {
            velocity -= animation[frame_index - 1].speed();
        }

        self.frame_index = frame_index;

        let floor_y = character.floor().y().expect("character should be on a specific floor");
        if character.water_level() < floor_y + WATER_SLOWDOWN_THRESHOLD {
            character.velocity.x -= character.velocity.x / WATER_SLOWDOWN;
        }

        character.velocity = velocity.xz();

        Ok(())
    }
}