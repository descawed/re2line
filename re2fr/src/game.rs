use std::time::Duration;

use anyhow::{Result, bail};
use hook86::mem::ByteSearcher;

mod types;
pub use types::*;

const RDT_STRING: &[u8] = b"Pl0\\Rdt\\room1000.rdt\0";
const FRAMES_PER_SECOND: u64 = 60;
const STAGE_CHARS: &str = "123456789ABCDEFG";

pub const NUM_CHARACTERS: usize = 34;

#[derive(Debug)]
pub struct GameVersion {
    pub version_name: &'static str,
    pub rdt_path_template: usize,
    pub char_array: usize,
    pub rng_seed: usize,
    pub igt_seconds: usize,
    pub igt_frames: usize,
    pub stage_index: usize,
    pub room_index: usize,
    pub stage_offset: usize,
    pub dummy_char: usize,
    pub keys_down: usize,
    pub keys_down_this_frame: usize,
    pub game_flags: usize,
    pub frame_tick_patch: usize,
    pub rng_roll_patch: usize,
}

const GAME_VERSIONS: [GameVersion; 1] = [
    GameVersion {
        version_name: "sourcenext11",
        rdt_path_template: 0x0053ab98,
        char_array: 0x0098a10c,
        rng_seed: 0x00988610,
        igt_seconds: 0x00680588,
        igt_frames: 0x0068058c,
        stage_index: 0x0098eb14,
        room_index: 0x0098eb16,
        stage_offset: 0x0098e798,
        dummy_char: 0x0098e544,
        keys_down: 0x00988604,
        keys_down_this_frame: 0x00988608,
        game_flags: 0x00989ed0,
        //frame_tick_patch: 0x0044229b,
        frame_tick_patch: 0x004c3c70,
        rng_roll_patch: 0x004b2a91,
    },
];

#[derive(Debug)]
pub struct Game {
    version: &'static GameVersion,
    characters: *const *const Character,
    dummy_char: *const Character,
    rng_seed: *const u32,
    keys_down: *const u32,
    keys_down_this_frame: *const u32,
    igt_seconds: *const u32,
    igt_frames: *const u8,
    stage_index: *const u16,
    room_index: *const u16,
    stage_offset: *const u32,
    game_flags: *const u32,
}

impl Game {
    pub unsafe fn init() -> Result<Self> {
        // find the address of the RDT string in memory
        let [Some(rdt_path_addr)] = ByteSearcher::find_bytes_anywhere(&[RDT_STRING], None) else {
            bail!("Could not identify RE2 version: failed to find RDT string");
        };

        log::debug!("Checking for version match");
        let rdt_path_addr = rdt_path_addr as usize;
        for version in &GAME_VERSIONS {
            log::debug!("Checking version {}", version.version_name);
            if version.rdt_path_template != rdt_path_addr {
                continue;
            }

            log::info!("Found RE2 version: {}", version.version_name);
            let characters = version.char_array as *const *const Character;
            let dummy_char = version.dummy_char as *const Character;
            let rng_seed = version.rng_seed as *const u32;
            let keys_down = version.keys_down as *const u32;
            let keys_down_this_frame = version.keys_down_this_frame as *const u32;
            let igt_seconds = version.igt_seconds as *const u32;
            let igt_frames = version.igt_frames as *const u8;
            let stage_index = version.stage_index as *const u16;
            let room_index = version.room_index as *const u16;
            let stage_offset = version.stage_offset as *const u32;
            let game_flags = version.game_flags as *const u32;

            return Ok(Self {
                version,
                characters,
                dummy_char,
                rng_seed,
                keys_down,
                keys_down_this_frame,
                igt_seconds,
                igt_frames,
                stage_index,
                room_index,
                stage_offset,
                game_flags,
            });
        }

        bail!("Unsupported RE2 version (RDT address {:08X})", rdt_path_addr);
    }

    pub fn version(&self) -> &'static GameVersion {
        self.version
    }

    pub fn rng(&self) -> u32 {
        unsafe {
            *self.rng_seed
        }
    }

    pub fn keys_down(&self) -> u32 {
        unsafe {
            *self.keys_down
        }
    }

    pub fn keys_down_this_frame(&self) -> u32 {
        unsafe {
            *self.keys_down_this_frame
        }
    }

    pub fn igt_seconds(&self) -> u32 {
        unsafe {
            *self.igt_seconds
        }
    }

    pub fn igt_frames(&self) -> u8 {
        unsafe {
            *self.igt_frames
        }
    }

    pub fn stage_index(&self) -> u16 {
        unsafe {
            *self.stage_index
        }
    }

    pub fn room_index(&self) -> u16 {
        unsafe {
            *self.room_index
        }
    }

    pub fn stage_offset(&self) -> u32 {
        unsafe {
            *self.stage_offset
        }
    }

    pub fn is_claire(&self) -> bool {
        unsafe {
            *self.game_flags & 0x80000000 != 0
        }
    }

    pub fn is_b_scenario(&self) -> bool {
        unsafe {
            *self.game_flags & 0x40000000 != 0
        }
    }

    pub fn room_id(&self) -> String {
        let (stage_index, room_index) = unsafe {
            (*self.stage_index as usize + (*self.stage_offset as usize & 0xff), *self.room_index)
        };
        let stage_char = STAGE_CHARS.chars().nth(stage_index).unwrap();
        let player_id = if self.is_claire() { '1' } else { '0' };
        format!("{}{:02X}{}", stage_char, room_index, player_id)
    }

    fn is_char_valid(&self, char: *const Character) -> bool {
        !char.is_null() && char != self.dummy_char
    }

    pub fn is_in_game(&self) -> bool {
        unsafe {
            self.is_char_valid(*self.characters)
        }
    }

    pub fn igt(&self) -> Duration {
        unsafe {
            Duration::from_secs(*self.igt_seconds as u64) + Duration::from_millis(*self.igt_frames as u64 * 1000 / FRAMES_PER_SECOND)
        }
    }

    pub fn characters(&self) -> impl Iterator<Item = Option<*const Character>> {
        unsafe {
            (0..NUM_CHARACTERS).map(|i| {
                let char = *self.characters.add(i);
                self.is_char_valid(char).then_some(char)
            })
        }
    }
}

unsafe impl Send for Game {}