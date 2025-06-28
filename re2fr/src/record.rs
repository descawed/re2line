use re2shared::record::*;
use residat::common::*;
use residat::re2::{Character, CharacterPart, MAX_PARTS, NUM_CHARACTERS, NUM_OBJECTS};

use crate::game::Game;

#[derive(Debug, Clone)]
struct Part {
    translation: VECTOR,
    x_size: UFixed16,
    y_size: UFixed16,
    z_size: UFixed16,
    size_offset: UFixed16,
}

impl Part {
    pub fn from_part(part: &CharacterPart) -> Self {
        Self {
            translation: part.pos.clone(),
            x_size: part.x_size,
            y_size: part.y_size,
            z_size: part.z_size,
            size_offset: part.size_offset,
        }
    }
}

#[derive(Debug)]
pub struct CharacterState {
    flags: u32,
    state: [u8; 4],
    id: u8,
    transform: MATRIX,
    parts: [Option<Part>; MAX_PARTS],
    part_offset_x: Fixed16,
    part_offset_z: Fixed16,
    model_part_transforms: Vec<MATRIX>,
    motion_angle: Fixed16,
    motion: i16,
    x_size: UFixed16,
    z_size: UFixed16,
    floor: u8,
    velocity: SVECTOR,
    health: i16,
    type_: u8,
}

impl CharacterState {
    pub fn from_character(char: &Character) -> Self {
        let mut parts = [None, None, None, None];
        for (state_part, char_part) in parts.iter_mut().zip(char.parts()) {
            *state_part = Some(Part::from_part(char_part));
        }

        Self {
            flags: char.flags,
            state: char.state.clone(),
            id: char.id,
            transform: char.transform.clone(),
            parts,
            part_offset_x: char.parts[0].x_offset,
            part_offset_z: char.parts[0].z_offset,
            model_part_transforms: unsafe { char.model_parts() }.into_iter().map(|p| p.composite_transform.clone()).collect(),
            motion_angle: char.motion_angle,
            motion: char.motion,
            x_size: char.parts[0].x_size,
            z_size: char.parts[0].z_size,
            floor: char.floor,
            velocity: char.velocity.clone(),
            health: char.health,
            type_: (char.type_ & 0xff) as u8,
        }
    }
    
    const fn model_parts_needed(&self) -> &'static [usize] {
        match self.id {
            32 => &[4], // dog
            49 => &[6, 11], // G2
            _ => &[],
        }
    }

    pub fn full_delta(&self) -> Vec<CharacterField> {
        let mut delta = vec![
            CharacterField::State(self.state.clone()),
            CharacterField::Id(self.id),
            CharacterField::Transform(self.transform.clone()),
            CharacterField::MotionAngle(self.motion_angle),
            CharacterField::Motion(self.motion),
            CharacterField::Size(self.x_size, self.z_size),
            CharacterField::Floor(self.floor),
            CharacterField::Velocity(self.velocity.clone()),
            CharacterField::Health(self.health),
            CharacterField::Type(self.type_),
            CharacterField::Flags(self.flags),
        ];

        for (i, part) in self.parts.iter().enumerate() {
            let Some(part) = part else {
                continue;
            };

            delta.push(CharacterField::PartTranslation(i as u8, part.translation.clone()));
            delta.push(CharacterField::PartSize(i as u8, part.x_size, part.y_size, part.z_size, part.size_offset));
        }
        
        for &i in self.model_parts_needed() {
            if i >= self.model_part_transforms.len() {
                log::warn!("Missing model part transform {} for character {} (num transforms: {})", i, self.id, self.model_part_transforms.len());
                continue;
            }
            
            delta.push(CharacterField::ModelPartTransform(i as u8, self.model_part_transforms[i].clone()));
        }
        
        delta
    }

    pub fn track_delta(&mut self, char: &Character) -> Vec<CharacterField> {
        let mut fields = Vec::with_capacity(MAX_CHARACTER_CHANGES);
        
        if self.flags != char.flags {
            self.flags = char.flags;
            fields.push(CharacterField::Flags(char.flags));       
        }

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

        let char_parts = char.parts();
        for (i, state_part) in self.parts.iter_mut().enumerate() {
            if i >= char_parts.len() {
                if state_part.is_some() {
                    *state_part = None;
                    fields.push(CharacterField::PartTranslation(i as u8, VECTOR::zero()));
                    fields.push(CharacterField::PartSize(i as u8, UFixed16(0), UFixed16(0), UFixed16(0), UFixed16(0)));
                }
                continue;
            }

            let char_part = &char_parts[i];
            
            match state_part {
                Some(state_part) => {
                    if state_part.translation != char_part.pos {
                        state_part.translation = char_part.pos.clone();
                        fields.push(CharacterField::PartTranslation(i as u8, char_part.pos.clone()));
                    }
                    
                    if state_part.x_size != char_part.x_size || state_part.y_size != char_part.y_size || state_part.z_size != char_part.z_size || state_part.size_offset != char_part.size_offset {
                        state_part.x_size = char_part.x_size;
                        state_part.y_size = char_part.y_size;
                        state_part.z_size = char_part.z_size;
                        state_part.size_offset = char_part.size_offset;
                        fields.push(CharacterField::PartSize(i as u8, char_part.x_size, char_part.y_size, char_part.z_size, char_part.size_offset));
                    }
                }
                None => {
                    *state_part = Some(Part::from_part(char_part));
                    fields.push(CharacterField::PartTranslation(i as u8, char_part.pos.clone()));
                    fields.push(CharacterField::PartSize(i as u8, char_part.x_size, char_part.y_size, char_part.z_size, char_part.size_offset));   
                }
            }
        }
        
        if self.part_offset_x != char.parts[0].x_offset || self.part_offset_z != char.parts[0].z_offset {
            self.part_offset_x = char.parts[0].x_offset;
            self.part_offset_z = char.parts[0].z_offset;
            fields.push(CharacterField::PartOffset(char.parts[0].x_offset, char.parts[0].z_offset));       
        }
        
        let model_parts_needed = self.model_parts_needed();
        for (i, model_part) in unsafe { char.model_parts() }.iter().enumerate() {
            if i >= self.model_part_transforms.len() {
                log::warn!("Unexpected model part transform {} for character {} (num transforms expected: {})", i, self.id, self.model_part_transforms.len());
                // we'll warn once but then just expand the transform array to account for the extra part(s)
                self.model_part_transforms.resize(i + 1, MATRIX::zero());
            }
            
            if self.model_part_transforms[i] != model_part.composite_transform {
                self.model_part_transforms[i] = model_part.composite_transform.clone();
                if model_parts_needed.contains(&i) {
                    fields.push(CharacterField::ModelPartTransform(i as u8, model_part.composite_transform.clone()));
                }
            }
        }

        if self.motion_angle != char.motion_angle {
            self.motion_angle = char.motion_angle;
            fields.push(CharacterField::MotionAngle(char.motion_angle));
        }

        // stop tracking this for now as it doesn't immediately appear to be useful
        /*if self.motion != char.motion {
            self.motion = char.motion;
            fields.push(CharacterField::Motion(char.motion));
        }*/

        if self.x_size != char.parts[0].x_size || self.z_size != char.parts[0].z_size {
            self.x_size = char.parts[0].x_size;
            self.z_size = char.parts[0].z_size;
            fields.push(CharacterField::Size(char.parts[0].x_size, char.parts[0].z_size));
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

        let type_ = (char.type_ & 0xff) as u8;
        if self.type_ != type_ {
            self.type_ = type_;
            fields.push(CharacterField::Type(type_));
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
    scenario: u8,
    sound_flags: u8,
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
            scenario: if game.is_claire() { 1 } else { 0 },
            sound_flags: game.sound_flags(),
        }
    }

    pub fn track_delta(&mut self, game: &Game) -> Vec<GameField> {
        let mut fields = Vec::new();

        let rng = game.rng();
        let keys_down = game.keys_down();
        let keys_down_this_frame = game.keys_down_this_frame();
        let stage_index = game.stage_index();
        let room_index = game.room_index();
        let stage_offset = game.stage_offset();
        let scenario = if game.is_claire() { 1 } else { 0 };
        let sound_flags = game.sound_flags();

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

        if self.scenario != scenario {
            self.scenario = scenario;
            fields.push(GameField::Scenario(self.scenario));
        }
        
        if self.sound_flags != sound_flags {
            self.sound_flags = sound_flags;
            fields.push(GameField::SoundFlags(self.sound_flags));       
        }

        fields
    }
}

#[derive(Debug)]
pub struct GameTracker {
    state: GameState,
    characters: [Option<CharacterState>; NUM_CHARACTERS],
    objects: [Option<CharacterState>; NUM_OBJECTS],
}

impl GameTracker {
    pub fn new(game: &Game) -> Self {
        Self {
            state: GameState::from_game(game),
            characters: [const { None }; NUM_CHARACTERS],
            objects: [const { None }; NUM_OBJECTS],       
        }
    }
    
    fn track_char_change(i: usize, char: Option<*const Character>, state: &mut Option<CharacterState>, character_diffs: &mut Vec<CharacterDiff>) {
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

    pub fn track_delta(&mut self, game: &Game) -> FrameRecord {
        let igt_seconds = game.igt_seconds();
        let igt_frames = game.igt_frames();

        let game_changes = self.state.track_delta(game);

        let mut character_diffs = Vec::with_capacity(NUM_CHARACTERS);
        for (i, (char, state)) in game.characters().zip(self.characters.iter_mut()).enumerate() {
            Self::track_char_change(i, char, state, &mut character_diffs);
        }
        
        let mut object_diffs = Vec::with_capacity(NUM_OBJECTS);
        for (i, (char, state)) in game.objects().zip(self.objects.iter_mut()).enumerate() {
            Self::track_char_change(i, char, state, &mut object_diffs);       
        }

        FrameRecord {
            igt_seconds,
            igt_frames,
            num_rng_rolls: 0,
            game_changes,
            character_diffs,
            object_diffs,
        }
    }
}