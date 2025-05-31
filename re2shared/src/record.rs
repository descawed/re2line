use binrw::binrw;

use crate::game::{FRAMES_PER_SECOND, MATRIX, SVECTOR};
use crate::rng::RollType;

pub const RECORD_VERSION: u16 = 1;
pub const MAX_CHARACTER_CHANGES: usize = 10;

#[binrw]
#[derive(Debug, Clone)]
pub enum CharacterField {
    #[brw(magic = 0u8)] State([u8; 4]),
    #[brw(magic = 1u8)] Id(u8),
    #[brw(magic = 2u8)] Transform(MATRIX),
    #[brw(magic = 3u8)] MotionAngle(i16),
    #[brw(magic = 4u8)] Motion(i16),
    #[brw(magic = 5u8)] Size(u16, u16),
    #[brw(magic = 6u8)] Floor(u8),
    #[brw(magic = 7u8)] Velocity(SVECTOR),
    #[brw(magic = 8u8)] Health(i16),
    #[brw(magic = 9u8)] Removed,
    #[brw(magic = 10u8)] Type(u8),
}

#[binrw]
#[derive(Debug, Clone)]
pub enum GameField {
    #[brw(magic = 0u8)] KeysDown(u32),
    #[brw(magic = 1u8)] KeysDownThisFrame(u32),
    #[brw(magic = 2u8)] StageIndex(u8),
    #[brw(magic = 3u8)] RoomIndex(u8),
    #[brw(magic = 4u8)] Rng(u16),
    #[brw(magic = 5u8)] StageOffset(u8),
    #[brw(magic = 6u8)] Scenario(u8),
    #[brw(magic = 7u8)]
    CharacterRng {
        char_index: u8,
        roll_type: RollType,
        start_value: u16,
    },
    #[brw(magic = 8u8)]
    KnownRng {
        roll_type: RollType,
        start_value: u16,
    },
    #[brw(magic = 9u8)]
    ScriptRng(u16),
    #[brw(magic = 10u8)]
    RngRoll(u32, u16),
    #[brw(magic = 11u8)]
    SoundFlags(u8),
    #[brw(magic = 12u8)]
    NewGame,
}

#[binrw]
#[derive(Debug)]
pub struct CharacterDiff {
    pub index: u8,
    #[bw(calc = changes.len() as u8)]
    num_changes: u8,
    #[br(count = num_changes)]
    pub changes: Vec<CharacterField>,
}

impl CharacterDiff {
    pub fn new(index: usize, changes: Vec<CharacterField>) -> Self {
        Self {
            index: index as u8,
            changes,
        }
    }

    pub fn removed(index: usize) -> Self {
        Self {
            index: index as u8,
            changes: vec![CharacterField::Removed],
        }
    }
}

#[binrw]
#[derive(Debug)]
pub struct FrameRecord {
    pub igt_seconds: u32,
    pub igt_frames: u8,
    pub num_rng_rolls: u16,

    #[bw(calc = game_changes.len() as u8)]
    num_game_changes: u8,
    #[br(count = num_game_changes)]
    pub game_changes: Vec<GameField>,

    #[bw(calc = character_diffs.len() as u8)]
    num_character_diffs: u8,
    #[br(count = num_character_diffs)]
    pub character_diffs: Vec<CharacterDiff>,
}

impl FrameRecord {
    pub fn time(&self) -> String {
        let minutes = self.igt_seconds / 60;
        let seconds = self.igt_seconds % 60;
        let frames = ((self.igt_frames as f32 / FRAMES_PER_SECOND as f32) * 100.0) as u32;
        format!("{:02}:{:02}:{:02}", minutes, seconds, frames)
    }
}

#[binrw]
#[brw(magic = b"RE2R")]
#[derive(Debug)]
pub struct RecordHeader {
    pub version: u16,
}

impl RecordHeader {
    pub const fn new() -> Self {
        Self {
            version: RECORD_VERSION,
        }
    }
}