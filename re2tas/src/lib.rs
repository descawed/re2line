use std::arch::naked_asm;
use std::ffi::c_void;
use std::fs::File;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{OnceLock, LazyLock, Mutex};

use anyhow::{anyhow, Result};
use hook86::dll::{dll_main, CallReason};
use hook86::mem;
use hook86::patch::Hook;
use log::LevelFilter;
use re2shared::game::GameVersion;
use simplelog::{Config, WriteLogger};
use tokio::sync::mpsc::error::TryRecvError;

mod api;
use api::{Api, Command, Request, Response};

mod tas;
use tas::TasController;

static FRAME_HOOK_MOVE_ADDRESS: AtomicU32 = AtomicU32::new(0);
static API: LazyLock<Mutex<Api>> = LazyLock::new(|| Mutex::new(Api::new()));
static CONTROLLER: OnceLock<Mutex<TasController>> = OnceLock::new();

#[unsafe(naked)]
unsafe extern "C" fn frame_tick_thunk() {
    naked_asm!(
        "mov eax,[{move_address}]",
        "mov eax,[eax]",
        "pushad",
        "call {frame_tick}",
        "popad",
        "ret",
        move_address = sym FRAME_HOOK_MOVE_ADDRESS,
        frame_tick = sym frame_tick,
    )
}

extern "C" fn frame_tick() {
    let mut api = api();
    loop {
        match api.try_recv() {
            Ok(Request { response_sender, command }) => {
                let response = handle_command(&command);
                if response_sender.send(response).is_err() {
                    log::error!("Failed to send response to command {}", command.describe());
                }
            }
            Err(TryRecvError::Empty) => {
                if !controller().is_paused() {
                    break;
                }
            },
            Err(TryRecvError::Disconnected) => {
                log::error!("API thread died; shutting down controller");
                controller().shutdown();
                break;
            }
        }

        // TODO: run message loop
    }
}

fn handle_command(command: &Command) -> Response {
    match command {
        Command::Pause => {
            controller().pause();
            Response::Success
        }
        Command::Resume => {
            controller().resume();
            Response::Success
        }
    }
}

fn open_log(log_level: LevelFilter, log_path: impl AsRef<Path>) -> Result<()> {
    let log_file = File::create(log_path)?;
    WriteLogger::init(log_level, Config::default(), log_file)?;
    log::info!("Beginning re2tas log");
    hook86::crash::install_crash_loggers();
    Ok(())
}

unsafe fn init() -> Result<()> {
    unsafe {
        let version = GameVersion::detect()?;

        let frame_hook_mov_address = std::ptr::read_unaligned((version.frame_tick_patch + 1) as *const mem::IntPtr);
        FRAME_HOOK_MOVE_ADDRESS.store(frame_hook_mov_address, Ordering::Release);
        let mut frame_hook = Hook::call(version.frame_tick_patch as *mut c_void, frame_tick_thunk as *const ()).expect_byte(0xA1); // mov eax
        frame_hook.install()?;

        CONTROLLER.set(Mutex::new(TasController::new(version, vec![frame_hook]))).map_err(|_| anyhow!("Controller was already initialized"))
    }
}

fn controller() -> impl DerefMut<Target = TasController> {
    CONTROLLER
        .get().expect("controller should be initialized")
        .lock().expect("controller lock should be acquired")
}

fn api() -> impl DerefMut<Target = Api> {
    API.lock().expect("API lock should be acquired")
}

#[dll_main(process)]
fn main(reason: CallReason) -> Result<()> {
    if let CallReason::ProcessDetach { is_process_exiting } = reason {
        // if the process is exiting, we don't need to do anything; the OS is shutting everything down anyway
        // if the controller isn't set then we haven't started anything up to be shut down
        if !is_process_exiting && CONTROLLER.get().is_some() {
            controller().shutdown();
            // we can't safely communicate with the API thread from DllMain, so we rely on whoever
            // freed the library while the process is still active to have done that
        }
        return Ok(());
    }

    open_log(LevelFilter::Info, "re2tas.log")?;

    unsafe { init() }?;
    log::info!("TAS controller initialized");
    Ok(())
}