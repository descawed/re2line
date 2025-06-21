use std::fs::File;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};

use crate::aot::Entity;
use crate::app::{GameObject, RoomId};
use crate::character::CharacterPath;
use crate::record::{Recording, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Checkpoint {
    Aot(u8),
}

impl Checkpoint {
    pub fn matches(&self, state: &State, entities: &[Entity]) -> bool {
        match self {
            Self::Aot(aot) => {
                let Some(ref player) = state.characters()[0] else {
                    return false;
                };

                let object_type = player.object_type();
                let interaction_point = player.interaction_point();
                let floor = player.floor();
                let is_action_pressed = state.input_state_this_frame().is_action_pressed;

                for entity in entities {
                    if entity.id() != *aot {
                        continue;
                    }

                    if entity.is_triggered(object_type, player.center, interaction_point, floor, is_action_pressed) {
                        return true;
                    }
                }

                false
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoomFilter {
    pub room_id: RoomId,
    pub entrance_id: Option<RoomId>,
    pub exit_id: Option<RoomId>,
    pub checkpoints: Vec<Checkpoint>,
}

impl RoomFilter {
    pub const fn new(room_id: RoomId, entrance_id: Option<RoomId>, exit_id: Option<RoomId>, checkpoints: Vec<Checkpoint>) -> Self {
        Self {
            room_id,
            entrance_id,
            exit_id,
            checkpoints,
        }
    }

    pub const fn basic(room_id: RoomId) -> Self {
        Self::new(room_id, None, None, Vec::new())
    }

    pub const fn empty() -> Self {
        Self::basic(RoomId::zero())
    }

    fn get_runs(&self, recording_path: Rc<PathBuf>, recording: &mut Recording, entities: &[Entity], runs: &mut Vec<Run>) {
        let mut last_room_id = RoomId::zero();
        let mut checkpoints = self.checkpoints.iter();
        let mut next_checkpoint = checkpoints.next();
        let mut start_index = 0usize;
        let mut end_index = usize::MAX;

        recording.set_index(0);
        while let Some(state) = recording.current_state() {
            if state.room_id() != last_room_id || state.is_new_game_start() {
                // we just entered a new room
                let entrance_id = if state.is_new_game_start() {
                    // if this is the start of a new game, we always consider the entrance criteria to
                    // be fulfilled, because there's no way other way to have reached this room
                    self.entrance_id
                } else {
                    Some(last_room_id)
                };

                last_room_id = state.room_id();
                start_index = state.frame_index();
                checkpoints = self.checkpoints.iter();
                next_checkpoint = checkpoints.next();

                // go ahead and check our exit point
                let exit_id = if let Some(next_state) = recording.peek_next_room() {
                    end_index = next_state.frame_index();
                    if next_state.is_new_game_start() {
                        // if this is the last room of the run, we always consider the exit criteria to be fulfilled
                        self.exit_id
                    } else {
                        Some(next_state.room_id())
                    }
                } else {
                    // if this is the last room of the run, we always consider the exit criteria to be fulfilled
                    end_index = usize::MAX;
                    self.exit_id
                };

                if state.room_id() != self.room_id || (self.entrance_id.is_some() && entrance_id != self.entrance_id) || (self.exit_id.is_some() && exit_id != self.exit_id) {
                    // this room doesn't match our criteria, so we can skip it
                    recording.next_room();
                    continue;
                }
            }

            // check if we've fulfilled our next checkpoint criteria
            if let Some(checkpoint) = next_checkpoint {
                if checkpoint.matches(state, entities) {
                    next_checkpoint = checkpoints.next();
                }
            }

            if next_checkpoint.is_none() {
                // we've fulfilled all the checkpoint criteria; extract the run
                recording.set_index(end_index - 1);
                if let Some(route) = recording.get_path_for_character(0) {
                    runs.push(Run {
                        source_path: Rc::clone(&recording_path),
                        frame_index: start_index,
                        route,
                        included: true,
                    });
                }
            }

            recording.next();
        }
    }
}

#[derive(Debug)]
pub struct LoadedRecording {
    path: Rc<PathBuf>,
    recording: Recording,
}

impl LoadedRecording {
    pub const fn new(path: Rc<PathBuf>, recording: Recording) -> Self {
        Self {
            path,
            recording,
        }
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        let file = File::open(&path)?;
        let recording = Recording::read(file)?;

        Ok(Self::new(Rc::new(path), recording))
    }

    pub fn load_for_run(&mut self, run: &Run) -> Result<()> {
        if self.path != run.source_path {
            let file = File::open(run.source_path.as_path())?;
            self.recording = Recording::read(file)?;
            self.path = Rc::clone(&run.source_path);
        }

        self.recording.set_index(run.frame_index);
        Ok(())
    }
    
    pub fn is_active_run(&self, run: &Run) -> bool {
        self.path == run.source_path && run.range().contains(&self.recording.index())
    }
}

#[derive(Debug, Clone)]
pub struct Run {
    source_path: Rc<PathBuf>,
    frame_index: usize,
    route: CharacterPath,
    included: bool,
}

impl Run {
    pub const fn set_included(&mut self, included: bool) {
        self.included = included;
    }

    pub const fn is_included(&self) -> bool {
        self.included
    }

    pub const fn route(&self) -> &CharacterPath {
        &self.route
    }

    pub const fn len(&self) -> usize {
        self.route.frames()
    }

    pub fn range(&self) -> Range<usize> {
        self.frame_index..self.frame_index + self.route.frames()
    }
    
    pub fn identifier(&self) -> String {
        format!("{}:{}", self.source_path.file_name().unwrap().display(), self.frame_index)
    }
}

#[derive(Debug)]
pub struct Comparison {
    runs: Vec<Run>,
    loaded_recording: LoadedRecording,
    active_run_index: usize,
}

impl Comparison {
    pub fn load_runs(recording_paths: Vec<PathBuf>, filter: &RoomFilter, entities: &[Entity]) -> Result<Self> {
        let mut loaded = None;
        let mut runs = Vec::new();
        for recording_path in recording_paths {
            let mut recording = LoadedRecording::load(recording_path)?;
            filter.get_runs(Rc::clone(&recording.path), &mut recording.recording, entities, &mut runs);
            loaded = Some(recording);
        }

        let Some(mut loaded_recording) = loaded else {
            bail!("Must select at least one recording to compare");
        };

        if runs.is_empty() {
            bail!("No runs found");
        }

        runs.sort_by_key(Run::len);

        loaded_recording.load_for_run(&runs[0])?;

        Ok(Self {
            runs,
            loaded_recording,
            active_run_index: 0,       
        })
    }
    
    pub fn is_active_run(&self, run: &Run) -> bool {
        self.loaded_recording.is_active_run(run)
    }

    pub fn set_active_run(&mut self, index: usize) -> Result<()> {
        self.loaded_recording.load_for_run(self.runs.get(index).ok_or_else(|| anyhow!("Invalid run index {index}"))?)?;
        self.active_run_index = index;
        Ok(())
    }
    
    pub fn active_run(&self) -> &Run {
        &self.runs[self.active_run_index]   
    }

    pub fn fastest_time(&self) -> usize {
        // we've sorted the fastest run to be first
        self.runs.iter().skip_while(|run| !run.is_included()).next().map(Run::len).unwrap_or(0)
    }

    pub fn slowest_time(&self) -> usize {
        self.runs.iter().rev().skip_while(|run| !run.is_included()).next().map(Run::len).unwrap_or(0)
    }

    pub fn average_time(&self) -> usize {
        let mut total = 0;
        let mut count = 0usize;
        for run in &self.runs {
            if run.is_included() {
                total += run.len();
                count += 1;
            }
        }

        total / count
    }

    pub const fn recording(&self) -> &Recording {
        &self.loaded_recording.recording
    }

    pub const fn recording_mut(&mut self) -> &mut Recording {
        &mut self.loaded_recording.recording
    }

    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    pub const fn num_runs(&self) -> usize {
        self.runs.len()
    }
}