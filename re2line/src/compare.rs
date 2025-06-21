use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};

use crate::aot::Entity;
use crate::app::{GameObject, RoomId};
use crate::character::CharacterPath;
use crate::record::{Recording, State};

#[derive(Debug, Clone)]
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
    room_id: RoomId,
    entrance_id: RoomId,
    exit_id: RoomId,
    checkpoints: Vec<Checkpoint>,
}

impl RoomFilter {
    pub const fn new(room_id: RoomId, entrance_id: RoomId, exit_id: RoomId, checkpoints: Vec<Checkpoint>) -> Self {
        Self {
            room_id,
            entrance_id,
            exit_id,
            checkpoints,
        }
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
                    last_room_id
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
                        next_state.room_id()   
                    }
                } else {
                    // if this is the last room of the run, we always consider the exit criteria to be fulfilled
                    end_index = usize::MAX;
                    self.exit_id
                };
                
                if state.room_id() != self.room_id || entrance_id != self.entrance_id || exit_id != self.exit_id {
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
}

#[derive(Debug)]
pub struct Comparison {
    runs: Vec<Run>,
    loaded_recording: LoadedRecording,
}

impl Comparison {
    pub fn load_runs(recording_paths: Vec<PathBuf>, filter: RoomFilter, entities: &[Entity]) -> Result<Self> {
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
        })
    }
    
    pub fn set_active_run(&mut self, index: usize) -> Result<()> {
        self.loaded_recording.load_for_run(self.runs.get(index).ok_or_else(|| anyhow!("Invalid run index {index}"))?)
    }
}