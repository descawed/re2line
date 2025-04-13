use std::ffi::c_void;
use std::fs::File;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::{OnceLock, Mutex};

use anyhow::{anyhow, Result};
use binrw::BinWriterExt;
use chrono::Local;
use hook86::asm;
use hook86::mem;
use hook86::patch::patch;
use log::LevelFilter;
use re2shared::record::RecordHeader;
use simplelog::{Config, WriteLogger};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

mod game;
use game::*;
mod record;
use record::*;

patch! {
    pub RngTrack = [
        call tracker // we'll also grab the caller address off the stack
        0x66 0xA1 imm32 seed // mov ax, [seed]
        ret
    ];
}

patch! {
    pub FrameTick = [
        0xA1 imm32 mov_address // mov eax,[mov_address]
        pushad
        call frame_tick
        popad
        ret
    ];
}

struct FlightRecorder {
    game: Game,
    tracker: GameTracker,
    file: Option<File>,
    rng_track: RngTrack,
    frame_tick: FrameTick,
    rng_calls: Vec<usize>,
}

impl FlightRecorder {
    pub fn apply_patches(&mut self) -> Result<()> {
        let version = self.game.version();

        let rng_track_thunk = self.rng_track.bind(track_rng as usize as mem::IntPtr, version.rng_seed as mem::IntPtr)?;

        let frame_hook_mov_address = unsafe { std::ptr::read_unaligned((version.frame_tick_patch + 1) as *const mem::IntPtr) };
        let frame_tick_thunk = self.frame_tick.bind(frame_hook_mov_address, frame_tick as usize as mem::IntPtr)?;

        let rng_track_call = {
            let c = asm::call(version.rng_roll_patch, rng_track_thunk as usize);
            // patched instruction is 6 bytes so we need to append a nop
            [c[0], c[1], c[2], c[3], c[4], asm::NOP]
        };
        let frame_tick_call = asm::call(version.frame_tick_patch, frame_tick_thunk as usize);

        unsafe {
            log::info!("Installing RNG tracker hook at {:08X}", version.rng_roll_patch);
            mem::patch(version.rng_roll_patch as *const c_void, &rng_track_call)?;

            log::info!("Installing frame tick hook at {:08X}", version.frame_tick_patch);
            mem::patch(version.frame_tick_patch as *const c_void, &frame_tick_call)?;
        }

        log::info!("Finished applying patches");
        Ok(())
    }

    pub fn record_frame(&mut self) -> Result<()> {
        if !self.game.is_in_game() {
            return Ok(());
        }

        let Some(ref mut file) = self.file else {
            log::warn!("Attempted to record frame when recording file was not open");
            return Ok(());
        };

        let mut frame_record = self.tracker.track_delta(&self.game);
        frame_record.num_rng_rolls = self.rng_calls.len() as u16;
        self.rng_calls.clear();
        file.write_le(&frame_record)?;
        Ok(())
    }

    pub fn close(&mut self) {
        self.file = None;
    }
}

// FIXME: can this value be moved? do I need Pin here somewhere?
static FLIGHT_RECORDER: OnceLock<Mutex<FlightRecorder>> = OnceLock::new();

extern "C" fn track_rng(_ecx: usize, _return: usize, caller: usize) {
    recorder().rng_calls.push(caller);
}

extern "C" fn frame_tick() {
    if let Err(e) = recorder().record_frame() {
        log::error!("Error recording frame: {e}");
    }
}

fn init_recorder() -> Result<()> {
    log::info!("Initializing recorder");

    let game = unsafe { Game::init() }?;
    let tracker = GameTracker::new(&game);

    // use the current timestamp in the filename to make it unique
    let now = Local::now();
    let filename = format!("re2fr_{}.bin", now.format("%Y-%m-%d_%H-%M-%S"));

    let mut file = File::create(filename)?;
    file.write_le(&RecordHeader::new())?;

    FLIGHT_RECORDER.set(Mutex::new(FlightRecorder {
        game,
        tracker,
        file: Some(file),
        rng_track: RngTrack::new(),
        frame_tick: FrameTick::new(),
        rng_calls: Vec::new(),
    })).map_err(|_| anyhow!("Flight recorder was already initialized"))
}

fn recorder() -> impl DerefMut<Target = FlightRecorder> {
    FLIGHT_RECORDER
        .get().expect("flight recorder should be initialized")
        .lock().expect("flight recorder lock should be acquired")
}

fn open_log(log_level: LevelFilter, log_path: impl AsRef<Path>) -> Result<()> {
    let log_file = File::create(log_path)?;
    WriteLogger::init(log_level, Config::default(), log_file)?;
    log::info!("Beginning re2fr log");
    hook86::crash::install_crash_loggers();
    Ok(())
}

fn main(reason: u32) -> Result<()> {
    if reason != DLL_PROCESS_ATTACH {
        if reason == DLL_PROCESS_DETACH {
            recorder().close();
        }
        return Ok(());
    }

    open_log(LevelFilter::Info, "re2fr.log")?;
    init_recorder()?;
    recorder().apply_patches()
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(_dll_module: HMODULE, reason: u32, _reserved: *const c_void) -> i32 {
    match main(reason) {
        Ok(_) => 1,
        Err(e) => {
            log::error!("Fatal error: {e}");
            log::logger().flush();
            0
        }
    }
}