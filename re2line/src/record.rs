use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::time::Duration;

use anyhow::{Result, bail};
use binrw::BinReaderExt;
use re2shared::game::NUM_CHARACTERS;
use re2shared::record::*;

use crate::app::RoomId;
use crate::character::*;
use crate::math::*;

pub const FRAME_DURATION: Duration = Duration::from_micros(1000000 / 30);

#[derive(Debug, Clone)]
pub struct State {
    frame_index: usize,
    room_index: usize,
    room_id: RoomId,
    characters: [Option<Character>; NUM_CHARACTERS],
}

impl State {
    pub const fn empty() -> Self {
        Self {
            // usize::MAX enables us to roll over to zero when we apply the first change set, which
            // represents the first frame
            frame_index: usize::MAX,
            room_index: usize::MAX,
            room_id: RoomId::new(0, 0, 0),
            characters: [const { None }; NUM_CHARACTERS],
        }
    }

    pub fn make_next_state(&self, record: &FrameRecord) -> Self {
        let mut room_id = self.room_id;
        for change in &record.game_changes {
            match change {
                GameField::StageIndex(stage_index) => room_id.stage = *stage_index,
                GameField::RoomIndex(room_index) => room_id.room = *room_index,
                GameField::Scenario(scenario) => room_id.player = *scenario,
                _ => (),
            }
        }

        let mut characters = self.characters.clone();
        for diff in &record.character_diffs {
            let character = &mut characters[diff.index as usize];
            for change in &diff.changes {
                if matches!(change, CharacterField::Removed) {
                    *character = None;
                    break;
                }

                if character.is_none() {
                    *character = Some(Character::empty(CharacterId::Unknown));
                }

                let character = character.as_mut().unwrap();
                match change {
                    CharacterField::State(state) => character.state.copy_from_slice(state),
                    CharacterField::Id(id) => character.id = match CharacterId::try_from(*id).ok() {
                        Some(id) => id,
                        None => {
                            println!("Unknown character ID: {}", id);
                            CharacterId::Unknown
                        }
                    },
                    CharacterField::Transform(matrix) => {
                        character.set_pos(matrix.t.x as i16, matrix.t.z as i16);
                    },
                    CharacterField::MotionAngle(_) => (), // TODO: figure out difference between Motion and MotionAngle
                    CharacterField::Motion(angle) => character.angle = Fixed12(*angle),
                    CharacterField::Size(width, height) => {
                        character.set_size(*width, *height);
                    }
                    CharacterField::Floor(floor) => character.floor = *floor,
                    CharacterField::Velocity(velocity) => {
                        character.velocity = Vec2::new(velocity.vx, velocity.vz);
                    }
                    CharacterField::Health(health) => character.set_health(*health),
                    CharacterField::Removed => unreachable!(),
                }
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
            characters,
        }
    }

    pub const fn room_id(&self) -> RoomId {
        self.room_id
    }

    pub fn characters(&self) -> &[Option<Character>] {
        &self.characters
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
        let size = f.seek(SeekFrom::End(0))?;
        f.seek(SeekFrom::Start(0))?;

        let header: RecordHeader = f.read_le()?;
        if header.version != RECORD_VERSION {
            bail!("Unsupported record version {}", header.version);
        }

        let mut state = State::empty();
        let mut frames: Vec<FrameRecord> = Vec::new();
        let mut checkpoints: Vec<State> = Vec::new();
        let mut max_room_size = 0usize;
        while f.seek(SeekFrom::Current(0))? < size {
            let frame = f.read_le()?;
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

    pub fn current_room(&self) -> &[State] {
        &self.states
    }

    pub fn next(&mut self) -> Option<&State> {
        self.set_index(self.index + 1)
    }

    pub fn set_index(&mut self, index: usize) -> Option<&State> {
        self.index = index;
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

    pub fn index(&self) -> usize {
        self.index
    }
}