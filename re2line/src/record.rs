use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};
use std::ops::Range;
use std::time::Duration;

use anyhow::{bail, Result};
use binrw::BinReaderExt;
use re2shared::record::*;
use re2shared::rng::RollType;
use residat::common::*;
use residat::re2::{CharacterId, NUM_CHARACTERS, NUM_OBJECTS};

use crate::app::{Floor, GameObject, RoomId};
use crate::character::*;
use crate::rng::{RNG_SEQUENCE, ROLL_DESCRIPTIONS, RollDescription};

pub const FRAME_DURATION: Duration = Duration::from_micros(1000000 / 30);

const KEY_FORWARD: u32 = 0x01;
const KEY_RIGHT: u32 = 0x02;
const KEY_BACK: u32 = 0x04;
const KEY_LEFT: u32 = 0x08;
const KEY_ACTION: u32 = 0x80;
const KEY_AIM: u32 = 0x100;
const KEY_RUN_CANCEL: u32 = 0x200;

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundEnvironment(u8);

impl SoundEnvironment {
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
    
    const fn bit(self, bit: u8) -> bool {
        (self.0 & bit) != 0
    }
    
    pub const fn is_gunshot_audible(self) -> bool {
        self.bit(0x01)
    }
    
    pub const fn is_walking_footstep_audible(self) -> bool {
        self.bit(0x02)
    }
    
    pub const fn is_running_footstep_audible(self) -> bool {
        self.bit(0x04)
    }
    
    pub const fn is_knife_audible(self) -> bool {
        self.bit(0x08)
    }
    
    pub const fn is_aim_audible(self) -> bool {
        self.bit(0x10)
    }
    
    pub const fn is_silent(self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone)]
pub struct PlayerSound {
    pub age: usize,
    pub pos: Vec2,
    pub sounds: SoundEnvironment,
}

#[derive(Debug, Clone)]
pub struct RoomStats {
    pub num_frames: usize,
    pub total_time: Duration,
    pub num_rng_rolls: usize,
    pub rng_position: usize,
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
    pub is_action_pressed: bool,
    pub is_run_cancel_pressed: bool,
    pub is_aim_pressed: bool,
}

impl InputState {
    pub const fn from_flags(flags: u32) -> Self {
        Self {
            is_forward_pressed: (flags & KEY_FORWARD) != 0,
            is_backward_pressed: (flags & KEY_BACK) != 0,
            is_left_pressed: (flags & KEY_LEFT) != 0,
            is_right_pressed: (flags & KEY_RIGHT) != 0,
            is_action_pressed: (flags & KEY_ACTION) != 0,
            is_run_cancel_pressed: (flags & KEY_RUN_CANCEL) != 0,
            is_aim_pressed: (flags & KEY_AIM) != 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    frame_index: usize,
    room_index: usize,
    room_id: RoomId,
    sounds: SoundEnvironment,
    characters: [Option<Character>; NUM_CHARACTERS],
    objects: [Option<Object>; NUM_OBJECTS],
    rng_value: u16,
    input_flags: u32,
    input_flags_this_frame: u32,
    is_new_game_start: bool,
}

impl State {
    pub const fn empty() -> Self {
        Self {
            // usize::MAX enables us to roll over to zero when we apply the first change set, which
            // represents the first frame
            frame_index: usize::MAX,
            room_index: usize::MAX,
            room_id: RoomId::new(0, 0, 0),
            sounds: SoundEnvironment::new(0),
            characters: [const { None }; NUM_CHARACTERS],
            objects: [const { None }; NUM_OBJECTS],
            rng_value: 0,
            input_flags: 0,
            input_flags_this_frame: 0,
            is_new_game_start: false,
        }
    }

    pub fn make_next_state(&self, record: &FrameRecord) -> Self {
        let mut room_id = self.room_id;
        let mut sounds = self.sounds;
        let mut rng_value = self.rng_value;
        let mut input_flags = self.input_flags;
        let mut input_flags_this_frame = self.input_flags_this_frame;
        let mut is_new_game_start = false;
        for change in &record.game_changes {
            match change {
                GameField::StageIndex(stage_index) => room_id.stage = *stage_index,
                GameField::RoomIndex(room_index) => room_id.room = *room_index,
                GameField::Scenario(scenario) => room_id.player = *scenario,
                GameField::SoundFlags(flags) => sounds = SoundEnvironment::new(*flags),
                GameField::Rng(rng) => rng_value = *rng,
                GameField::KeysDown(flags) => input_flags = *flags,
                GameField::KeysDownThisFrame(flags) => input_flags_this_frame = *flags,
                GameField::NewGame => is_new_game_start = true,
                _ => (),
            }
        }

        let mut characters = self.characters.clone();
        for diff in &record.character_diffs {
            let index = diff.index as usize;
            let character = &mut characters[index];
            for change in &diff.changes {
                if matches!(change, CharacterField::Removed) {
                    *character = None;
                    break;
                }

                if character.is_none() {
                    *character = Some(Character::empty(CharacterId::Unknown));
                }

                let character = character.as_mut().unwrap();
                character.set_index(index);
                match change {
                    CharacterField::State(state) => character.state.copy_from_slice(state),
                    CharacterField::Id(id) => character.id = match CharacterId::try_from(*id).ok() {
                        Some(id) => id,
                        None => {
                            eprintln!("Unknown character ID: {}", id);
                            CharacterId::Unknown
                        }
                    },
                    CharacterField::Transform(matrix) => {
                        character.set_pos(&matrix.t);
                        character.set_prev_pos(&matrix.t);
                    },
                    CharacterField::PartTranslation(i, vector) => {
                        if let Some(part) = character.parts_mut().get_mut(*i as usize) {
                            match part {
                                Some(part) => part.set_pos(vector),
                                None => *part = Some(Part::from_pos(vector.into())),
                            }
                        }
                    }
                    CharacterField::PartSize(i, x, y, z, offset) => {
                        if let Some(part) = character.parts_mut().get_mut(*i as usize) {
                            match part {
                                Some(part) => part.set_size(*x, *y, *z, *offset),
                                None => *part = Some(Part::from_size(Vec3::new(*x, *y, *z), *offset)),
                            }
                        }
                    }
                    CharacterField::PartOffset(x, z) => character.set_part_offset(Vec2::new(*x, *z)),
                    CharacterField::ModelPartTransform(i, matrix) => {
                        let pos = Vec2::new(matrix.t.x, matrix.t.z);
                        character.set_model_part_center(*i as usize, pos);
                    }
                    CharacterField::MotionAngle(angle) => character.angle = angle.to_32(),
                    CharacterField::Motion(_) => (), // seems like this might not be something useful?
                    CharacterField::Size(width, height) => {
                        character.set_size(*width, *height);
                    }
                    CharacterField::Floor(floor) => character.set_floor(Floor::Id(*floor)),
                    CharacterField::Velocity(velocity) => {
                        character.velocity = Vec2::new(velocity.vx, velocity.vz);
                    }
                    CharacterField::Health(health) => character.set_health(*health),
                    CharacterField::Removed => unreachable!(),
                    CharacterField::Type(type_) => character.type_ = *type_,
                    CharacterField::Flags(flags) => character.flags = *flags,
                }
            }

            if let (Some(new_character), Some(old_character)) = (character.as_mut(), self.characters[index].as_ref()) {
                new_character.set_prev_pos(old_character.center_3d());
                if let Some(Some(part)) = old_character.parts().get(0) {
                    new_character.set_prev_root_part_pos(part.pos());
                }
            }
        }

        let mut objects = self.objects.clone();
        for diff in &record.object_diffs {
            let index = diff.index as usize;
            let object = &mut objects[index];
            for change in &diff.changes {
                if matches!(change, CharacterField::Removed) {
                    *object = None;
                    break;
                }

                if object.is_none() {
                    *object = Some(Object::empty());
                }

                let object = object.as_mut().unwrap();
                object.set_index(index);
                match change {
                    CharacterField::Transform(matrix) => object.set_pos(&matrix.t),
                    CharacterField::PartTranslation(i, vector) => {
                        if let Some(part) = object.parts_mut().get_mut(*i as usize) {
                            match part {
                                Some(part) => part.set_pos(vector),
                                None => *part = Some(Part::from_pos(vector.into())),
                            }
                        }
                        
                        if *i == 0 {
                            object.update_gui_shape();
                            if object.prev_root_part_pos().is_zero() {
                                object.set_prev_root_part_pos(vector.into());
                            }
                        }
                    }
                    CharacterField::PartSize(i, x, y, z, offset) => {
                        if let Some(part) = object.parts_mut().get_mut(*i as usize) {
                            match part {
                                Some(part) => part.set_size(*x, *y, *z, *offset),
                                None => *part = Some(Part::from_size(Vec3::new(*x, *y, *z), *offset)),
                            }
                        }
                    }
                    CharacterField::Size(width, height) => object.set_size(*width, *height),
                    CharacterField::Floor(floor) => object.set_floor(Floor::Id(*floor)),
                    CharacterField::Flags(flags) => object.flags = *flags,
                    CharacterField::Removed => unreachable!(),
                    // don't care about these for objects
                    CharacterField::State(_) | CharacterField::Id(_) | CharacterField::MotionAngle(_)
                    | CharacterField::Motion(_) | CharacterField::Health(_) | CharacterField::Type(_)
                    | CharacterField::Velocity(_)
                    | CharacterField::ModelPartTransform(_, _) | CharacterField::PartOffset(_, _) => (),
                }
            }

            if let (Some(new_object), Some(old_object)) = (object.as_mut(), self.objects[index].as_ref()) {
                new_object.set_prev_root_part_pos(old_object.center_3d());
            }
        }

        let frame_index = if self.frame_index < usize::MAX {
            self.frame_index + 1
        } else {
            0
        };

        let room_index = if room_id == self.room_id && self.room_index < usize::MAX {
            self.room_index + 1
        } else {
            0
        };

        Self {
            frame_index,
            room_index,
            room_id,
            sounds,
            characters,
            objects,
            rng_value,
            input_flags,
            input_flags_this_frame,
            is_new_game_start,
        }
    }

    pub const fn room_id(&self) -> RoomId {
        self.room_id
    }

    pub fn characters(&self) -> &[Option<Character>] {
        &self.characters
    }

    pub fn objects(&self) -> &[Option<Object>] {
        &self.objects
    }
    
    pub fn player_sounds(&self) -> Option<PlayerSound> {
        let (Some(player), false) = (self.characters[0].as_ref(), self.sounds.is_silent()) else {
            return None;
        };
        
        Some(PlayerSound {
            age: 0,
            pos: player.center(),
            sounds: self.sounds,
        })
    }
    
    pub const fn input_state(&self) -> InputState {
        InputState::from_flags(self.input_flags)
    }

    pub const fn input_state_this_frame(&self) -> InputState {
        InputState::from_flags(self.input_flags_this_frame)
    }

    pub const fn frame_index(&self) -> usize {
        self.frame_index
    }

    pub const fn is_new_game_start(&self) -> bool {
        self.is_new_game_start
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollCategory {
    Character(u8),
    NonCharacter,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct RngDescription {
    pub description: String,
    pub category: RollCategory,
    pub roll_type: Option<RollType>,
    pub start_value: u16,
}

impl RngDescription {
    pub const fn new(description: String, category: RollCategory, roll_type: Option<RollType>, start_value: u16) -> Self {
        Self { description, category, roll_type, start_value: start_value & 0x7fff }
    }

    pub const fn character(description: String, character_index: u8, roll_type: RollType, start_value: u16) -> Self {
        Self::new(description, RollCategory::Character(character_index), Some(roll_type), start_value)
    }
    
    pub const fn non_character(description: String, roll_type: RollType, start_value: u16) -> Self {
        Self::new(description, RollCategory::NonCharacter, Some(roll_type), start_value)
    }
    
    pub const fn unknown(description: String, start_value: u16) -> Self {
        Self::new(description, RollCategory::Unknown, None, start_value)
    }

    pub fn is_unknown(&self) -> bool {
        self.category == RollCategory::Unknown || self.roll_type.is_none()
    }

    pub fn rng_index(&self) -> usize {
        RNG_SEQUENCE.iter().position(|v| *v == self.start_value).unwrap()
    }

    fn adjacent_unique_value(&self, delta: isize) -> Option<(usize, isize, String)> {
        let rng_index = self.rng_index();
        let roll_description = &ROLL_DESCRIPTIONS[self.roll_type?];
        let value = roll_description.outcome(self.start_value)?;

        let num_rng_values = RNG_SEQUENCE.len() as isize;
        let mut next_index = (rng_index as isize + delta).rem_euclid(num_rng_values) as usize;
        let mut distance = delta;
        while next_index != rng_index  {
            let next_value = roll_description.outcome(RNG_SEQUENCE[next_index])?;
            if next_value != value {
                return Some((next_index, distance, next_value));
            }
            next_index = (next_index as isize + delta).rem_euclid(num_rng_values) as usize;
            distance += delta;
        }

        None
    }

    pub fn next_unique_value(&self) -> Option<(usize, isize, String)> {
        self.adjacent_unique_value(1)
    }

    pub fn prev_unique_value(&self) -> Option<(usize, isize, String)> {
        self.adjacent_unique_value(-1)
    }

    fn distribution_subset(&self, roll_description: &RollDescription, range_min: usize, range_max: usize, distribution: &mut HashMap<String, usize>) {
        for seed in &RNG_SEQUENCE[range_min..range_max] {
            let value = roll_description.outcome(*seed).unwrap();
            let count = distribution.entry(value).or_insert(0);
            *count += 1;
        }
    }

    pub fn distribution(&self, range_min: isize, range_max: isize) -> Vec<(String, usize)> {
        let mut distribution = HashMap::new();
        if self.category == RollCategory::Unknown {
            return Vec::new();
        }

        let roll_description = &ROLL_DESCRIPTIONS[self.roll_type.unwrap()];
        let rng_index = self.rng_index();

        let min = if range_min < -(rng_index as isize) {
            let wrap_around = range_min.abs() - rng_index as isize;
            let wrap_start = (RNG_SEQUENCE.len() as isize - wrap_around) as usize;
            self.distribution_subset(roll_description, wrap_start, RNG_SEQUENCE.len(), &mut distribution);

            0isize
        } else {
            rng_index as isize + range_min
        };

        let mut max = (rng_index as isize + range_max) as usize + 1;
        if max > RNG_SEQUENCE.len() {
            let wrap_end = max - RNG_SEQUENCE.len();
            self.distribution_subset(roll_description, 0, wrap_end, &mut distribution);

            max = RNG_SEQUENCE.len();
        }

        self.distribution_subset(roll_description, min as usize, max, &mut distribution);

        let mut distribution = distribution.into_iter().collect::<Vec<_>>();
        distribution.sort_by_key(|v| (v.1, v.0.clone()));
        distribution.reverse();

        distribution
    }
    
    fn values_subset(&self, roll_description: &RollDescription, range_min: usize, range_max: usize, values: &mut Vec<(usize, String)>) {
        for i in range_min..range_max {
            let seed = &RNG_SEQUENCE[i];
            values.push((i, roll_description.outcome(*seed).unwrap()));
        }
    }

    pub fn values_in_range(&self, range_min: isize, range_max: isize) -> Vec<(usize, String)> {
        let mut values = Vec::new();
        if self.is_unknown() {
            return values;
        }

        let roll_description = &ROLL_DESCRIPTIONS[self.roll_type.unwrap()];
        let rng_index = self.rng_index();

        let min = if range_min < -(rng_index as isize) {
            let wrap_around = range_min.abs() - rng_index as isize;
            let wrap_start = (RNG_SEQUENCE.len() as isize - wrap_around) as usize;
            self.values_subset(roll_description, wrap_start, RNG_SEQUENCE.len(), &mut values);

            0isize
        } else {
            rng_index as isize + range_min
        };

        let max = (rng_index as isize + range_max) as usize + 1;
        self.values_subset(roll_description, min as usize, max.min(RNG_SEQUENCE.len()), &mut values);
        
        if max > RNG_SEQUENCE.len() {
            let wrap_end = max - RNG_SEQUENCE.len();
            self.values_subset(roll_description, 0, wrap_end, &mut values);
        }

        values
    }

    pub fn options(&self) -> &[&'static str] {
        if self.is_unknown() {
            return &[];
        }

        ROLL_DESCRIPTIONS[self.roll_type.unwrap()].options()
    }
}

#[derive(Debug, Clone)]
pub struct FrameRng {
    pub frame_index: usize,
    pub timestamp: String,
    pub rng_descriptions: Vec<RngDescription>,
}

impl FrameRng {
    pub const fn new(frame_index: usize, timestamp: String) -> Self {
        Self {
            frame_index,
            timestamp,
            rng_descriptions: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Recording {
    frames: Vec<FrameRecord>,
    states: Vec<State>,
    checkpoints: Vec<State>, // one checkpoint per room transition
    index: usize,
    range: Range<usize>,
}

impl Recording {
    pub fn read(mut f: impl Read + Seek + BinReaderExt) -> Result<Self> {
        // reading the entire file into memory and then parsing it is SIGNIFICANTLY faster than
        // parsing directly from disk
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;

        let size = buf.len() as u64;
        let mut f = Cursor::new(buf);

        let header: RecordHeader = f.read_le()?;
        if header.version == 0 || header.version > RECORD_VERSION {
            bail!("Unsupported record version {}", header.version);
        }

        let mut state = State::empty();
        let mut frames: Vec<FrameRecord> = Vec::new();
        let mut checkpoints: Vec<State> = Vec::new();
        let mut max_room_size = 0usize;
        while f.stream_position()? < size {
            let frame = match header.version {
                1 => {
                    let frame_v1: FrameRecordV1 = f.read_le()?;
                    frame_v1.into()
                }
                2 => f.read_le()?,
                _ => unreachable!(),
            };
            state = state.make_next_state(&frame);
            if state.room_index >= max_room_size {
                max_room_size = state.room_index + 1;
            }
            if state.room_index == 0 {
                checkpoints.push(state.clone());
            }
            frames.push(frame);
        }

        let mut recording = Self {
            frames,
            index: 0,
            states: Vec::with_capacity(max_room_size),
            checkpoints,
            range: 0..0,
        };
        // initialize state
        recording.set_index(0);

        Ok(recording)
    }

    pub fn frames(&self) -> &[FrameRecord] {
        &self.frames
    }

    pub fn current_frame(&self) -> Option<&FrameRecord> {
        self.frames.get(self.index)
    }

    pub fn current_state(&self) -> Option<&State> {
        if !self.range.contains(&self.index) {
            return None;
        }

        let room_index = self.index - self.range.start;
        self.states.get(room_index)
    }

    pub fn peek_next_room(&self) -> Option<&State> {
        for checkpoint in &self.checkpoints {
            if self.index < checkpoint.frame_index {
                return Some(checkpoint);
            }
        }

        None
    }

    pub fn next_room(&mut self) -> Option<&State> {
        let mut next_index = None;
        for checkpoint in &self.checkpoints {
            if self.index < checkpoint.frame_index {
                next_index = Some(checkpoint.frame_index);
                break;
            }
        }

        self.set_index(next_index.unwrap_or(self.frames.len()))
    }

    pub fn next(&mut self) -> Option<&State> {
        self.set_index(self.index + 1)
    }

    pub fn prev(&mut self) -> Option<&State> {
        if self.index > 0 {
            self.set_index(self.index - 1)
        } else {
            None
        }
    }

    pub fn set_index(&mut self, index: usize) -> Option<&State> {
        self.index = index;
        if index > self.frames.len() {
            self.index = self.frames.len();
        }

        if !self.range.contains(&index) {
            let mut last_state = None;
            let mut end_index = None;
            for checkpoint in &self.checkpoints {
                if index < checkpoint.frame_index {
                    end_index = Some(checkpoint.frame_index);
                    break;
                }
                last_state = Some(checkpoint);
            }

            let Some(mut state) = last_state.map(Clone::clone) else {
                return None;
            };

            let start_index = state.frame_index;
            let end_index = end_index.unwrap_or(self.frames.len());
            self.range = start_index..end_index;

            self.states.clear();
            self.states.push(state.clone());
            for change in &self.frames[start_index + 1..end_index] {
                state = state.make_next_state(change);
                self.states.push(state.clone());
            }
        }

        self.current_state()
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn room_range(&self) -> &Range<usize> {
        &self.range
    }
    
    pub fn get_rng_descriptions(&self) -> Vec<FrameRng> {
        let mut frames = Vec::new();
        let end = self.index.min(self.frames.len() - 1);
        for i in self.range.start..=end {
            let frame_record = &self.frames[i];
            let state = &self.states[i - self.range.start];
            
            let mut frame_rng = FrameRng::new(i, frame_record.time());
            for change in &frame_record.game_changes {
                match change {
                    GameField::RngRoll(address, value) => {
                        frame_rng.rng_descriptions.push(RngDescription::unknown(format!("{:08X} rolled on {:04X}", address, value), *value));
                    }
                    GameField::KnownRng { roll_type, start_value } => {
                        let description_data = &ROLL_DESCRIPTIONS[*roll_type];
                        frame_rng.rng_descriptions.push(RngDescription::non_character(description_data.describe(*start_value, None), *roll_type, *start_value));
                    }
                    GameField::CharacterRng { char_index, roll_type, start_value } => {
                        let description_data = &ROLL_DESCRIPTIONS[*roll_type];
                        let character_name = state.characters()
                            .get(*char_index as usize)
                            .and_then(|c| c.as_ref().map(Character::name))
                            .map(|n| format!("#{} {}", char_index, n));
                        frame_rng.rng_descriptions.push(
                            RngDescription::character(description_data.describe(*start_value, character_name.as_ref().map(String::as_str)), *char_index, *roll_type, *start_value)
                        );
                    }
                    _ => (),
                }
            }
            
            if !frame_rng.rng_descriptions.is_empty() {
                frames.push(frame_rng);
            }
        }
        
        frames
    }
    
    pub fn get_player_sounds(&self, max_age: usize) -> Vec<PlayerSound> {
        let mut sounds = Vec::new();
        let start = (self.index - max_age.min(self.index)).max(self.range.start);
        let end = self.index.min(self.frames.len() - 1);
        for i in start..=end {
            let state = &self.states[i - self.range.start];
            if let Some(mut sound) = state.player_sounds() {
                sound.age = self.index - i;
                sounds.push(sound);
            }
        }
        
        sounds
    }
    
    pub fn get_room_stats(&self) -> RoomStats {
        RoomStats {
            num_frames: self.range.len(),
            total_time: FRAME_DURATION * (self.range.len() as u32),
            num_rng_rolls: self.frames[self.range.start..self.range.end]
                .iter()
                .map(|frame| {
                    frame.game_changes
                        .iter()
                        .filter(|change| matches!(change, GameField::RngRoll(_, _) | GameField::KnownRng { .. } | GameField::CharacterRng { .. }))
                        .count()
                })
                .sum(),
            rng_position: RNG_SEQUENCE.iter().position(|r| *r == (self.states[0].rng_value & 0x7fff)).unwrap_or(0),
        }
    }
    
    pub fn get_path_for_character(&self, index: usize) -> Option<CharacterPath> {
        let character = self.current_state()?.characters().get(index)?.as_ref()?;
        let current_index = self.index - self.range.start;
        let mut start_index = current_index;
        while start_index > 0 && self.states[start_index - 1].characters()[index].as_ref().map(|c| c.id) == Some(character.id) {
            start_index -= 1;
        }
        
        let mut points = Vec::with_capacity(current_index - start_index + 1);
        for i in start_index..=current_index {
            let Some(state_char) = self.states[i].characters()[index].as_ref() else {
                continue;
            };
            
            points.push(state_char.center());
        }
        
        Some(CharacterPath::new(points, character.id, character.floor()))
    }

    pub fn timeline(&self) -> Vec<Vec<(String, &State)>> {
        let mut timeline = Vec::new();
        let mut current_run = Vec::new();
        for state in &self.checkpoints {
            if state.is_new_game_start && !current_run.is_empty() {
                timeline.push(current_run);
                current_run = Vec::new();
            }

            let timestamp = self.frames[state.frame_index].time();
            current_run.push((timestamp, state));
        }

        if !current_run.is_empty() {
            timeline.push(current_run);
        }

        timeline
    }
}