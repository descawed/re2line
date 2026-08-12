use hook86::patch::Hook;
use re2shared::game::GameVersion;

#[derive(Debug)]
pub struct TasController {
    game: &'static GameVersion,
    hooks: Vec<Hook>,
    is_paused: bool,
    frame_counter: usize,
}

impl TasController {
    pub const fn new(game: &'static GameVersion, hooks: Vec<Hook>) -> Self {
        Self {
            game,
            hooks,
            is_paused: false,
            frame_counter: 0,
        }
    }

    pub const fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub const fn pause(&mut self) {
        self.is_paused = true;
    }

    pub const fn resume(&mut self) {
        self.is_paused = false;
    }
    
    pub const fn frame_counter(&self) -> usize {
        self.frame_counter
    }
    
    pub fn frame_tick(&mut self) {
        self.frame_counter += 1;
    }

    pub fn shutdown(&mut self) {
        // hooks will be uninstalled on drop
        self.hooks.clear();
    }
}

unsafe impl Send for TasController {}