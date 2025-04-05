use crate::game::{Character, Game, NUM_CHARACTERS, MATRIX, SVECTOR};

mod bin;
pub use bin::*;

#[derive(Debug)]
pub struct CharacterState {
    state: [u8; 4],
    id: u8,
    transform: MATRIX,
    motion_angle: i16,
    motion: i16,
    x_size: u16,
    z_size: u16,
    floor: u8,
    velocity: SVECTOR,
    health: i16,
}

impl CharacterState {
    pub fn from_character(char: &Character) -> Self {
        Self {
            state: char.state.clone(),
            id: char.id,
            transform: char.transform.clone(),
            motion_angle: char.motion_angle,
            motion: char.motion,
            x_size: char.x_size,
            z_size: char.z_size,
            floor: char.floor,
            velocity: char.velocity.clone(),
            health: char.health,
        }
    }

    pub fn full_delta(&self) -> Vec<CharacterField> {
        vec![
            CharacterField::State(self.state.clone()),
            CharacterField::Id(self.id),
            CharacterField::Transform(self.transform.clone()),
            CharacterField::MotionAngle(self.motion_angle),
            CharacterField::Motion(self.motion),
            CharacterField::Size(self.x_size, self.z_size),
            CharacterField::Floor(self.floor),
            CharacterField::Velocity(self.velocity.clone()),
            CharacterField::Health(self.health),
        ]
    }

    pub fn track_delta(&mut self, char: &Character) -> Vec<CharacterField> {
        let mut fields = Vec::with_capacity(MAX_CHARACTER_CHANGES);

        if self.state != char.state {
            self.state = char.state.clone();
            fields.push(CharacterField::State(char.state.clone()));
        }

        if self.id != char.id {
            self.id = char.id;
            fields.push(CharacterField::Id(char.id));
        }

        if self.transform != char.transform {
            self.transform = char.transform.clone();
            fields.push(CharacterField::Transform(char.transform.clone()));
        }

        if self.motion_angle != char.motion_angle {
            self.motion_angle = char.motion_angle;
            fields.push(CharacterField::MotionAngle(char.motion_angle));
        }

        if self.motion != char.motion {
            self.motion = char.motion;
            fields.push(CharacterField::Motion(char.motion));
        }

        if self.x_size != char.x_size || self.z_size != char.z_size {
            self.x_size = char.x_size;
            self.z_size = char.z_size;
            fields.push(CharacterField::Size(char.x_size, char.z_size));
        }

        if self.floor != char.floor {
            self.floor = char.floor;
            fields.push(CharacterField::Floor(char.floor));
        }

        if self.velocity != char.velocity {
            self.velocity = char.velocity.clone();
            fields.push(CharacterField::Velocity(char.velocity.clone()));
        }

        if self.health != char.health {
            self.health = char.health;
            fields.push(CharacterField::Health(char.health));
        }

        fields
    }
}

#[derive(Debug)]
pub struct GameState {
    rng: u32,
    keys_down: u32,
    keys_down_this_frame: u32,
    stage_index: u16,
    room_index: u16,
    stage_offset: u32,
}

impl GameState {
    pub fn from_game(game: &Game) -> Self {
        Self {
            rng: game.rng(),
            keys_down: game.keys_down(),
            keys_down_this_frame: game.keys_down_this_frame(),
            stage_index: game.stage_index(),
            room_index: game.room_index(),
            stage_offset: game.stage_offset(),
        }
    }

    pub fn track_delta(&mut self, game: &Game) -> Vec<GameField> {
        let mut fields = Vec::with_capacity(MAX_GAME_CHANGES);

        let rng = game.rng();
        let keys_down = game.keys_down();
        let keys_down_this_frame = game.keys_down_this_frame();
        let stage_index = game.stage_index();
        let room_index = game.room_index();
        let stage_offset = game.stage_offset();

        if self.rng != rng {
            self.rng = rng;
            fields.push(GameField::Rng(self.rng as u16));
        }

        if self.keys_down != keys_down {
            self.keys_down = keys_down;
            fields.push(GameField::KeysDown(self.keys_down));
        }

        if self.keys_down_this_frame != keys_down_this_frame {
            self.keys_down_this_frame = keys_down_this_frame;
            fields.push(GameField::KeysDownThisFrame(self.keys_down_this_frame));
        }

        if self.stage_index != stage_index {
            self.stage_index = stage_index;
            fields.push(GameField::StageIndex(self.stage_index as u8));
        }

        if self.room_index != room_index {
            self.room_index = room_index;
            fields.push(GameField::RoomIndex(self.room_index as u8));
        }

        if self.stage_offset != stage_offset {
            self.stage_offset = stage_offset;
            fields.push(GameField::StageOffset(self.stage_offset as u8));
        }

        fields
    }
}

#[derive(Debug)]
pub struct GameTracker {
    state: GameState,
    characters: [Option<CharacterState>; NUM_CHARACTERS],
}

impl GameTracker {
    pub fn new(game: &Game) -> Self {
        Self {
            state: GameState::from_game(game),
            characters: [const { None }; NUM_CHARACTERS],
        }
    }

    pub fn track_delta(&mut self, game: &Game) -> FrameRecord {
        let igt_seconds = game.igt_seconds();
        let igt_frames = game.igt_frames();

        let game_changes = self.state.track_delta(game);

        let mut character_diffs = Vec::with_capacity(NUM_CHARACTERS);
        for (i, (char, state)) in game.characters().zip(self.characters.iter_mut()).enumerate() {
            match (char, state.as_mut()) {
                (None, Some(_)) => {
                    character_diffs.push(CharacterDiff::removed(i));
                    *state = None;
                }
                (Some(char), None) => {
                    let char = unsafe { &*char };
                    let char_state = CharacterState::from_character(char);
                    character_diffs.push(CharacterDiff::new(i, char_state.full_delta()));
                    *state = Some(char_state);
                }
                (Some(char), Some(state)) => {
                    let char = unsafe { &*char };
                    let delta = state.track_delta(char);
                    if !delta.is_empty() {
                        character_diffs.push(CharacterDiff::new(i, delta));
                    }
                }
                _ => (),
            }
        }

        FrameRecord {
            igt_seconds,
            igt_frames,
            num_rng_rolls: 0,
            game_changes,
            character_diffs,
        }
    }
}