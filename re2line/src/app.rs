use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs::File;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::io::BufReader;
use std::str::FromStr;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use eframe::{Frame, Storage};
use egui::{Color32, Context, Key, RichText, TextBuffer, Ui, ViewportCommand};
use egui::layers::ShapeIdx;
use egui::widgets::color_picker::Alpha;
use egui_plot::{Line, Plot};
use epaint::{Stroke, StrokeKind};
use re2script::ScriptFormatter;
use re2shared::record::FrameRecord;
use re2shared::rng::RollType;
use residat::common::{Fixed32, UFixed16, Vec2};
use residat::re2::{CharacterId, Rdt, RdtSection, NUM_CHARACTERS, NUM_OBJECTS};
use rfd::FileDialog;

use crate::aot::{Entity, EntityForm, NUM_AOTS};
use crate::character::{Character, Object, PositionedAiZone, WeaponRangeVisualization};
use crate::collision::Collider;
use crate::compare::{Checkpoint, Comparison, RoomFilter};
use crate::draw::{VAlign, text_box};
use crate::rdt::RdtExt;
use crate::record::{PlayerSound, Recording, RngDescription, RollCategory, State, FRAME_DURATION};
use crate::rng::{RNG_SEQUENCE, ROLL_DESCRIPTIONS};

mod config;
mod game;
mod layer;

use config::Config;
pub use config::RoomId;
pub use game::{DrawParams, Floor, GameObject, ObjectType, WorldPos};
use layer::Layer;

pub const APP_NAME: &str = "re2line";

const DETAIL_MAX_ROWS: usize = 4;
const FAST_FORWARD: isize = 30;
const MAX_SOUND_AGE: usize = 100;

const INPUT_MARGIN: f32 = 2.0;
const INPUT_SIZE: f32 = 30.0;
const INPUT_OFFSET: f32 = INPUT_SIZE + INPUT_MARGIN;

const TEXT_BOX_DARK: Color32 = Color32::from_rgb(0x30, 0x30, 0x30);
const TEXT_BOX_LIGHT: Color32 = Color32::from_rgb(0xe0, 0xe0, 0xe0);
const UNFOCUSED_FADE: f32 = 0.25;

const TOOLTIP_HOVER_SECONDS: f32 = 1.0;

const COMPARISON_PATH_WIDTH: f32 = 0.0125;
const COMPARISON_PATH_EMPHASIS_WIDTH: f32 = 0.025;

trait UiExt {
    fn draw_game_object<O: GameObject>(&self, object: &O, params: &DrawParams, state: &State) -> ShapeIdx;

    fn draw_game_tooltip<O: GameObject>(&self, object: &O, params: &DrawParams, state: &State, index: usize) -> ShapeIdx;
}

impl UiExt for Ui {
    fn draw_game_object<O: GameObject>(&self, object: &O, params: &DrawParams, state: &State) -> ShapeIdx {
        self.painter().add(object.gui_shape(params, state))
    }

    fn draw_game_tooltip<O: GameObject>(&self, object: &O, params: &DrawParams, state: &State, index: usize) -> ShapeIdx {
        self.painter().add(object.gui_tooltip(params, state, self, &object.name_prefix(index)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedObject {
    None,
    Entity(usize),
    Collider(usize),
    Floor(usize),
    Object(usize),
    Character(usize),
    AiZone(usize),
}

impl SelectedObject {
    const fn for_object_type(object_type: ObjectType, index: usize) -> Self {
        if object_type.is_character() {
            Self::Character(index)
        } else if object_type.is_ai_zone() {
            Self::AiZone(index)
        } else if matches!(object_type, ObjectType::Object) {
            Self::Object(index)
        } else if object_type.is_aot() {
            Self::Entity(index)
        } else if object_type.is_collider() {
            Self::Collider(index)
        } else if object_type.is_floor() {
            Self::Floor(index)
        } else {
            Self::None
        }
    }

    fn matches<O: GameObject>(&self, object: &O, index: usize) -> bool {
        if matches!(self, Self::None) {
            return false;
        }

        *self == Self::for_object_type(object.object_type(), index)
    }
}

impl Default for SelectedObject {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserTab {
    Game,
    Room,
    Settings,
    Rng,
    Recording,
    Comparison,
}

impl BrowserTab {
    const fn list() -> [BrowserTab; 6] {
        [BrowserTab::Game, BrowserTab::Room, BrowserTab::Comparison, BrowserTab::Recording, BrowserTab::Rng, BrowserTab::Settings]
    }

    const fn name(&self) -> &'static str {
        match self {
            Self::Game => "Game",
            Self::Room => "Room",
            Self::Settings => "Settings",
            Self::Rng => "RNG",
            Self::Recording => "Recording",
            Self::Comparison => "Comparison",
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct CharacterSettings {
    pub show: bool,
    pub show_tooltip: bool,
    pub show_ai: bool,
    pub show_path: bool,
    pub show_rng_rolls: bool,
}

impl CharacterSettings {
    pub const fn config_default(config: &Config) -> Self {
        Self {
            show: true,
            show_tooltip: config.default_show_character_tooltips,
            show_ai: true,
            show_path: false,
            show_rng_rolls: true,
        }
    }

    pub const fn show_tooltip(&self) -> bool {
        self.show && self.show_tooltip
    }

    pub const fn show_ai(&self) -> bool {
        self.show && self.show_ai
    }

    pub const fn show_path(&self) -> bool {
        self.show && self.show_path
    }
    
    pub const fn show_rng_rolls(&self) -> bool {
        self.show_rng_rolls
    }
}

impl Default for CharacterSettings {
    fn default() -> Self {
        Self {
            show: true,
            show_tooltip: true,
            show_ai: true,
            show_path: false,
            show_rng_rolls: true,
        }
    }
}

pub struct App {
    center: Vec2,
    colliders: Layer<Collider>,
    objects: Layer<Object>,
    characters: Layer<Character>,
    ai_zones: Layer<PositionedAiZone>,
    entities: Layer<Entity>,
    floors: Layer<Collider>,
    pan: egui::Vec2,
    selected_object: SelectedObject,
    hover_object: SelectedObject,
    hover_pos: Option<egui::Pos2>,
    config: Config,
    tab: BrowserTab,
    leon_rooms: Vec<(PathBuf, RoomId)>,
    claire_rooms: Vec<(PathBuf, RoomId)>,
    need_title_update: bool,
    active_recording: Option<Recording>,
    is_recording_playing: bool,
    last_play_tick: Instant,
    character_settings: HashMap<(RoomId, CharacterId, usize), CharacterSettings>,
    pointer_game_pos: Option<Vec2>,
    current_rdt: Option<Rdt>,
    error_message: Option<String>,
    compare_filter: RoomFilter,
    is_compare_filter_window_open: bool,
    comparison: Option<Comparison>,
    show_comparison_paths: bool,
    rng_distribution_range_min: isize,
    rng_distribution_range_max: isize,
    rng_distribution_binary: bool,
    rng_selected_outcomes: HashSet<&'static str>,
    rng_selected_roll_type: Option<RollType>,
    rng_selected_index: usize,
    rng_run_threshold: f64,
    rng_run_window_size: usize,
    is_rng_explore_window_open: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            center: Vec2::zero(),
            colliders: Layer::new(),
            objects: Layer::new(),
            characters: Layer::new(),
            ai_zones: Layer::new(),
            entities: Layer::new(),
            floors: Layer::new(),
            pan: egui::Vec2::ZERO,
            selected_object: SelectedObject::None,
            hover_object: SelectedObject::None,
            hover_pos: None,
            config: Config::get()?,
            tab: BrowserTab::Game,
            leon_rooms: Vec::new(),
            claire_rooms: Vec::new(),
            need_title_update: false,
            active_recording: None,
            is_recording_playing: false,
            last_play_tick: Instant::now(),
            character_settings: HashMap::new(),
            pointer_game_pos: None,
            current_rdt: None,
            error_message: None,
            compare_filter: RoomFilter::empty(),
            is_compare_filter_window_open: false,
            comparison: None,
            show_comparison_paths: true,
            rng_distribution_range_min: -100,
            rng_distribution_range_max: 100,
            rng_distribution_binary: false,
            rng_selected_outcomes: HashSet::new(),
            rng_selected_roll_type: None,
            rng_selected_index: 0,
            rng_run_threshold: 2.0 / 3.0 * 100.0,
            rng_run_window_size: 10,
            is_rng_explore_window_open: false,
        })
    }

    const fn scale(&self) -> f32 {
        self.config.zoom_scale
    }

    fn toggle_play_recording(&mut self) {
        if self.active_recording().is_none() {
            return;
        }

        self.is_recording_playing = !self.is_recording_playing;
        self.last_play_tick = Instant::now();
    }

    fn visit_layer_objects<O: GameObject, T, F: Fn(usize, &O) -> Option<T>>(&self, layer: &Layer<O>, visitor: F, asc: bool) -> Option<T> {
        if asc {
            for (i, object) in layer.visible_objects(&self.config) {
                if let Some(value) = visitor(i, object) {
                    return Some(value);
                }
            }
        } else {
            for (i, object) in layer.visible_objects_desc(&self.config) {
                if let Some(value) = visitor(i, object) {
                    return Some(value);
                }
            }
        }

        None
    }

    fn is_ai_zone_visible(&self, ai_zone: &PositionedAiZone) -> bool {
        if !self.config.should_show(ai_zone.object_type()) {
            return false;
        }

        match (self.get_character(ai_zone.character_index), self.get_character_settings(ai_zone.character_index)) {
            (Some(character), Some(settings)) => {
                self.config.should_show(character.object_type()) && settings.show_ai()
            }
            _ => false,
        }
    }

    fn check_selected_object<O: GameObject>(object: &O, pos: Vec2, selection_value: SelectedObject) -> Option<SelectedObject> {
        object.contains_point(pos).then_some(selection_value)
    }

    fn check_selected_ai_zone(&self, zone: &PositionedAiZone, pos: Vec2, index: usize) -> Option<SelectedObject> {
        if self.is_ai_zone_visible(zone) {
            Self::check_selected_object(zone, pos, SelectedObject::AiZone(index))
        } else {
            None
        }
    }

    fn select_object(&self, pos: Vec2, include_ai_zones: bool) -> SelectedObject {
        let selection = self.visit_layer_objects(
            &self.characters,
            |_, o| Self::check_selected_object(o, pos, SelectedObject::Character(o.index())),
            false,
        );
        if let Some(object) = selection {
            return object;
        }

        if include_ai_zones {
            let selection = self.visit_layer_objects(
                &self.ai_zones,
                |i, o| self.check_selected_ai_zone(o, pos, i),
                false,
            );
            if let Some(object) = selection {
                return object;
            }
        }

        self.visit_layer_objects(&self.objects, |_, o| Self::check_selected_object(o, pos, SelectedObject::Object(o.index())), false)
            .or_else(|| self.visit_layer_objects(&self.entities, |i, o| Self::check_selected_object(o, pos, SelectedObject::Entity(i)), false))
            .or_else(|| self.visit_layer_objects(&self.colliders, |i, o| Self::check_selected_object(o, pos, SelectedObject::Collider(i)), false))
            .or_else(|| self.visit_layer_objects(&self.floors, |i, o| Self::check_selected_object(o, pos, SelectedObject::Floor(i)), false))
            .unwrap_or_default()
    }

    fn click_select(&mut self, pos: Vec2) {
        self.selected_object = self.select_object(pos, false);
    }

    fn hover_select(&mut self, pos: Vec2) {
        self.hover_object = self.select_object(pos, true);
    }
    
    fn screen_pos_to_game_pos(&self, pos: egui::Pos2, viewport: egui::Rect) -> Vec2 {
        let viewport_center = viewport.center().to_vec2();
        let view_relative = (pos + self.pan - viewport_center) / self.scale();
        Vec2::new(Fixed32::from_f32(view_relative.x) + self.center.x, -(Fixed32::from_f32(view_relative.y) - self.center.z))
    }
    
    fn set_pointer_game_pos(&mut self, pos: Option<egui::Pos2>, viewport: egui::Rect) {
        let Some(pos) = pos else {
            self.pointer_game_pos = None;
            return;
        };
        
        self.pointer_game_pos = Some(self.screen_pos_to_game_pos(pos, viewport));
    }

    fn handle_input(&mut self, ctx: &Context) {
        let egui_wants_kb_input = ctx.wants_keyboard_input();
        ctx.input(|i| {
            if i.pointer.middle_down() && !i.pointer.button_pressed(egui::PointerButton::Middle) {
                self.pan -= i.pointer.delta();
            }
            
            let viewport = i.screen_rect();
            self.set_pointer_game_pos(i.pointer.latest_pos(), viewport);
            
            if i.pointer.primary_pressed() {
                // select object that was clicked on
                if self.pointer_game_pos.is_none() {
                    // if we didn't find the pointer pos from latest_pos(), try again with interact_pos()
                    self.set_pointer_game_pos(i.pointer.interact_pos(), viewport);
                }
                if let Some(game_pos) = self.pointer_game_pos {
                    self.click_select(game_pos);
                }
            }

            if i.pointer.time_since_last_movement() >= TOOLTIP_HOVER_SECONDS {
                if let Some(hover_pos) = i.pointer.hover_pos() {
                    self.hover_select(self.screen_pos_to_game_pos(hover_pos, viewport));
                    self.hover_pos = Some(hover_pos);
                }
            } else {
                self.hover_object = SelectedObject::None;
                self.hover_pos = None;
            }

            self.config.zoom_scale += i.smooth_scroll_delta.y * 0.05;

            if !egui_wants_kb_input {
                if i.key_pressed(Key::Space) {
                    self.toggle_play_recording();
                }

                if self.active_recording().is_some() {
                    if self.is_recording_playing {
                        // skip forward or back in chunks
                        if i.key_pressed(Key::ArrowRight) {
                            self.move_recording_frame(FAST_FORWARD);
                        } else if i.key_pressed(Key::ArrowLeft) {
                            self.move_recording_frame(-FAST_FORWARD);
                        }
                    } else {
                        // frame-by-frame
                        if i.key_pressed(Key::ArrowRight) {
                            self.next_recording_frame();
                        } else if i.key_pressed(Key::ArrowLeft) {
                            self.prev_recording_frame();
                        }
                    }
                }
            }
        });
    }

    fn calculate_origin(&mut self, ctx: &Context) -> egui::Pos2 {
        let viewport = ctx.input(egui::InputState::screen_rect);

        let window_center = viewport.center();
        egui::Pos2::new(
            self.center.x * self.scale() - window_center.x,
            -(self.center.z * self.scale()) - window_center.y,
        ) + self.pan
    }

    fn clear_rdt(&mut self) {
        self.center = Vec2::zero();
        self.colliders.clear();
        self.entities.clear();
        self.floors.clear();
        self.pan = egui::Vec2::ZERO;
        self.selected_object = SelectedObject::None;
        self.hover_object = SelectedObject::None;
        self.need_title_update = true;
        self.current_rdt = None;
        self.compare_filter = RoomFilter::empty();

        // also pause any active recording and clear its GUI objects
        self.is_recording_playing = false;
        self.characters.clear();
        self.objects.clear();
        self.ai_zones.clear();
    }

    fn set_rdt(&mut self, rdt: Rdt, id: RoomId) {
        self.center = rdt.center();
        self.colliders.set_objects(rdt.get_colliders());
        self.entities.set_objects(rdt.get_entities());
        self.floors.set_objects(rdt.get_floors());
        self.pan = egui::Vec2::ZERO;
        self.selected_object = SelectedObject::None;
        self.hover_object = SelectedObject::None;
        self.config.last_rdt = Some(id);
        self.need_title_update = true;
        self.current_rdt = Some(rdt);
        self.compare_filter = RoomFilter::basic(id);
    }

    pub fn try_resume(&mut self) -> Result<()> {
        if let Some(ref path) = self.config.rdt_folder {
            self.load_game_folder(path.clone())?;

            if let Some((id, Some(path))) = self.config.last_rdt.map(|id| (id, self.get_room_path(id).map(PathBuf::from))) {
                self.load_rdt(id, path)?;
            }
        }

        Ok(())
    }

    pub fn load_rdt(&mut self, id: RoomId, path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let rdt = Rdt::read(reader)?;

        self.set_rdt(rdt, id);

        Ok(())
    }

    pub fn load_room(&mut self, id: RoomId) -> Result<()> {
        let path = self.get_room_path(id).ok_or_else(|| anyhow!("Could not find room"))?;
        self.load_rdt(id, path.to_path_buf())
    }

    fn get_room_path(&self, id: RoomId) -> Option<&Path> {
        let room_list = if id.player == 0 {
            &self.leon_rooms
        } else {
            &self.claire_rooms
        };

        for (path, room_id) in room_list {
            if id == *room_id {
                return Some(path.as_path());
            }
        }

        None
    }

    fn is_game_loaded(&self) -> bool {
        !self.leon_rooms.is_empty() || !self.claire_rooms.is_empty()
    }

    fn get_entry_case_insensitive(dir: impl AsRef<Path>, name: &str) -> Result<Option<PathBuf>> {
        for entry in dir.as_ref().read_dir()? {
            let entry = entry?;
            let lc_name = entry.file_name().to_string_lossy().to_lowercase();
            if lc_name == name {
                return Ok(Some(entry.path()));
            }
        }

        Ok(None)
    }

    fn enumerate_rdts(dir: impl AsRef<Path>, rdt_list: &mut Vec<(PathBuf, RoomId)>) -> Result<()> {
        let rdt_dir = Self::get_entry_case_insensitive(dir, "rdt")?.ok_or_else(|| anyhow!("Could not find RDT folder"))?;

        for entry in rdt_dir.read_dir()? {
            let entry = entry?;
            let lc_name = entry.file_name().to_string_lossy().to_lowercase();
            if lc_name.starts_with("room") && lc_name.ends_with(".rdt") {
                let room_id = RoomId::from_str(&lc_name[4..8])?;
                rdt_list.push((entry.path(), room_id));
            }
        }

        // sort RDT list by ID
        rdt_list.sort_by(|a, b| a.1.cmp(&b.1));

        Ok(())
    }

    pub fn load_game_folder(&mut self, dir: PathBuf) -> Result<()> {
        self.leon_rooms.clear();
        self.claire_rooms.clear();

        for entry in dir.read_dir()? {
            let entry = entry?;
            let lc_name = entry.file_name().to_string_lossy().to_lowercase();
            match lc_name.as_str() {
                "pl0" => Self::enumerate_rdts(entry.path(), &mut self.leon_rooms)?,
                "pl1" => Self::enumerate_rdts(entry.path(), &mut self.claire_rooms)?,
                _ => (),
            }

            if !self.leon_rooms.is_empty() && !self.claire_rooms.is_empty() {
                break;
            }
        }

        if !self.is_game_loaded() {
            bail!("Invalid game directory could not find RDT files");
        }

        self.config.rdt_folder = Some(dir);

        if let Some(room_id) = self.config.last_rdt {
            // reload the room
            if let Err(e) = self.load_room(room_id) {
                self.show_error(format!("Failed to load room {room_id}: {e}"));
                self.clear_rdt();
            }
        } else {
            self.clear_rdt();
        }

        self.need_title_update = true;

        Ok(())
    }

    fn prompt_load_game(&mut self) -> Result<()> {
        let Some(folder) = FileDialog::new().pick_folder() else {
            return Ok(());
        };

        self.load_game_folder(folder)
    }

    fn load_recording(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(path)?;
        self.active_recording = Some(Recording::read(file)?);
        // remove any active comparison
        self.comparison = None;
        if self.tab == BrowserTab::Comparison {
            self.tab = BrowserTab::Recording;
        }
        // reset character display settings for new recording
        self.character_settings.clear();
        self.change_recording_frame(|r| r.set_index(0));

        Ok(())
    }

    fn prompt_load_recording(&mut self) -> Result<()> {
        let Some(path) = FileDialog::new().add_filter("RE2 recordings", &["bin"]).pick_file() else {
            return Ok(());
        };

        self.load_recording(path)
    }
    
    fn close_recording(&mut self) {
        self.active_recording = None;
        self.is_recording_playing = false;
        self.objects.clear();
        self.character_settings.clear();
        self.ai_zones.clear();
        self.characters.clear();
        if matches!(self.selected_object, SelectedObject::Character(_) | SelectedObject::Object(_)) {
            self.selected_object = SelectedObject::None;
        }

        if self.tab == BrowserTab::Recording {
            self.tab = BrowserTab::Room;
        }
    }
    
    fn close_comparison(&mut self) {
        self.comparison = None;
        self.is_recording_playing = false;
        self.objects.clear();
        self.character_settings.clear();
        self.ai_zones.clear();
        self.characters.clear();
        if matches!(self.selected_object, SelectedObject::Character(_) | SelectedObject::Object(_)) {
            self.selected_object = SelectedObject::None;
        }
        
        if self.tab == BrowserTab::Comparison {
            self.tab = BrowserTab::Room;
        }
    }

    fn active_recording(&self) -> Option<&Recording> {
        self.active_recording.as_ref().or_else(|| self.comparison.as_ref().map(Comparison::recording))
    }

    fn active_recording_mut(&mut self) -> Option<&mut Recording> {
        self.active_recording.as_mut().or_else(|| self.comparison.as_mut().map(Comparison::recording_mut))
    }
    
    fn decompile_scripts(&self) -> Result<String> {
        let Some(ref rdt) = self.current_rdt else {
            bail!("No RDT loaded");
        };
        
        let init_buf = rdt.raw(RdtSection::InitScript);
        let exec_buf = rdt.raw(RdtSection::ExecScript);
        
        let mut formatter = ScriptFormatter::new(true, false, 2, false);
        let init_func = formatter.parse_function(init_buf);
        let exec_script = formatter.parse_script(exec_buf)?;
        
        Ok(format!("{}\n\n{}", init_func, exec_script))
    }

    fn room_browser(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            if let Some(ref recording) = self.active_recording {
                let stats = recording.get_room_stats();

                ui.label(format!("Frames:\t{}", stats.num_frames));
                
                let seconds = stats.total_time.as_secs_f32();
                let minutes = (seconds / 60.0) as i32;
                let seconds = seconds % 60.0;
                ui.label(format!("Time:\t{:02}:{:05.2}", minutes, seconds));
                
                ui.label(format!("RNG rolls:\t{}", stats.num_rng_rolls));
                ui.label(format!("RNG position:\t{}", stats.rng_position));
            }
            
            if self.current_rdt.is_some() {
                if ui.button("Print scripts").clicked() {
                    match self.decompile_scripts() {
                        Ok(script) => println!("{script}"),
                        Err(e) => eprintln!("Failed to decompile scripts: {e}"),
                    }
                }
            }

            ui.separator();

            ui.collapsing("Floor", |ui| {
                for i in 0..self.floors.len() {
                    ui.selectable_value(&mut self.selected_object, SelectedObject::Floor(i), format!("Floor {}", i));
                }
            });

            ui.collapsing("Collision", |ui| {
                for i in 0..self.colliders.len() {
                    ui.selectable_value(&mut self.selected_object, SelectedObject::Collider(i), format!("Collider {}", i));
                }
            });

            ui.collapsing("Door", |ui| {
                let mut door_count = 0;
                for (i, entity) in self.entities.objects().iter().enumerate() {
                    if entity.object_type() != ObjectType::Door {
                        continue;
                    }

                    ui.selectable_value(&mut self.selected_object, SelectedObject::Entity(i), format!("Door {}", door_count));
                    door_count += 1;
                }
            });

            ui.collapsing("Item", |ui| {
                let mut item_count = 0;
                for (i, entity) in self.entities.objects().iter().enumerate() {
                    if entity.object_type() != ObjectType::Item {
                        continue;
                    }

                    ui.selectable_value(&mut self.selected_object, SelectedObject::Entity(i), format!("Item {}", item_count));
                    item_count += 1;
                }
            });

            ui.collapsing("AOT", |ui| {
                let mut aot_count = 0;
                for (i, entity) in self.entities.objects().iter().enumerate() {
                    if matches!(entity.object_type(), ObjectType::Door | ObjectType::Item) {
                        continue;
                    }

                    ui.selectable_value(&mut self.selected_object, SelectedObject::Entity(i), format!("AOT {}", aot_count));
                    aot_count += 1;
                }
            });

            if self.active_recording().is_some() {
                ui.collapsing("Objects", |ui| {
                    for object in self.objects.objects() {
                        let i = object.index();
                        ui.selectable_value(&mut self.selected_object, SelectedObject::Object(i), format!("Object {}", i));
                    }
                });
                
                ui.collapsing("Characters", |ui| {
                    for character in self.characters.objects() {
                        let i = character.index();
                        ui.selectable_value(&mut self.selected_object, SelectedObject::Character(i), format!("#{}: {}", i, character.name()));
                    }
                });
            }
        });
    }

    fn rdt_list(&mut self, is_leon: bool, ui: &mut Ui) {
        let mut room_to_load = None;

        let rdt_list = if is_leon {
            &self.leon_rooms
        } else {
            &self.claire_rooms
        };

        for (path, id) in rdt_list {
            let id = *id;
            let is_current_room = self.config.last_rdt == Some(id);
            if ui.selectable_label(is_current_room, format!("{}", id)).clicked() && !is_current_room {
                room_to_load = Some((path.clone(), id));
            }
        }

        if let Some((path, id)) = room_to_load {
            if let Err(e) = self.load_rdt(id, path) {
                self.show_error(format!("Failed to load room {id}: {e}"));
            }
        }
    }

    fn rdt_browser(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            ui.collapsing("Leon", |ui| {
                self.rdt_list(true, ui);
            });
            ui.collapsing("Claire", |ui| {
                self.rdt_list(false, ui);
            });
        });
    }

    fn frames_to_time(frames: usize) -> String {
        let duration = FRAME_DURATION * frames as u32;
        let seconds = duration.as_secs_f32();
        let minutes = (seconds / 60.0) as i32;
        let seconds = seconds % 60.0;
        format!("{:02}:{:05.2}", minutes, seconds)
    }

    fn comparison_browser(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            let Some(ref mut comparison) = self.comparison else {
                return;
            };

            let fastest_time = comparison.fastest_time();
            let slowest_time = comparison.slowest_time();
            let average_time = comparison.average_time();

            ui.label(format!("Runs: {}", comparison.num_runs()));
            ui.label(format!("Fastest: {} ({})", Self::frames_to_time(fastest_time), fastest_time));
            ui.label(format!("Slowest: {} ({})", Self::frames_to_time(slowest_time), slowest_time));
            ui.label(format!("Average: {} ({})", Self::frames_to_time(average_time), average_time));

            ui.add_space(2.5);

            let mut include_exclusions_in_statistics = comparison.include_exclusions_in_statistics();
            ui.checkbox(&mut include_exclusions_in_statistics, "Include exclusions in statistics");
            comparison.set_include_exclusions_in_statistics(include_exclusions_in_statistics);

            ui.checkbox(&mut self.show_comparison_paths, "Show paths");

            ui.horizontal(|ui| {
                if ui.button("Select all").clicked() {
                    for run in comparison.runs_mut() {
                        run.set_included(true);
                    }
                }

                if ui.button("Select none").clicked() {
                    for run in comparison.runs_mut() {
                        run.set_included(false);
                    }
                }
            });

            ui.separator();

            let mut selected_run = None;
            let active_run_index = comparison.active_run_index();
            for (i, run) in comparison.runs_mut().into_iter().enumerate() {
                let is_active = i == active_run_index;
                if ui.selectable_label(is_active, run.identifier()).clicked() && !is_active {
                    selected_run = Some(i);
                }

                let mut included = run.is_included();
                ui.checkbox(&mut included, "Include");
                run.set_included(included);

                ui.label(format!("  Time: {} ({})", Self::frames_to_time(run.len()), run.len()));
            }

            if let Some(i) = selected_run {
                match comparison.set_active_run(i) {
                    Ok(_) => self.update_from_state(),
                    Err(e) => self.show_error(format!("Failed to load run: {e}")),
                }
            }
        });
    }
    
    fn recording_browser(&mut self, ui: &mut Ui) {
        let mut selected_frame = None;
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            let Some(ref recording) = self.active_recording else {
                return;
            };
            
            for (i, run) in recording.timeline().into_iter().enumerate() {
                ui.collapsing(format!("Run #{}", i + 1), |ui| {
                    for (timestamp, state) in run {
                        let frame_index = state.frame_index();
                        let label = format!("{} - {} ({})", state.room_id(), timestamp, frame_index);
                        if ui.selectable_label(recording.room_range().contains(&frame_index), label).clicked() {
                            selected_frame = Some(frame_index);
                        }
                    }
                });
            }
        });
        
        if let Some(frame_index) = selected_frame {
            self.change_recording_frame(|r| r.set_index(frame_index));
        }
    }
    
    fn rng_browser(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            let Some(rng_descriptions) = self.active_recording().map(Recording::get_rng_descriptions) else {
                return;
            };
            
            ui.checkbox(&mut self.config.show_character_rng, "Show character rolls");
            ui.checkbox(&mut self.config.show_known_non_character_rng, "Show known non-character rolls");
            ui.checkbox(&mut self.config.show_unknown_rng, "Show unknown rolls");
            
            if self.config.show_character_rng {
                ui.collapsing("Characters", |ui| {
                    let mut set_all = None;
                    
                    ui.horizontal(|ui| {
                        if ui.button("Select all").clicked() {
                            set_all = Some(true);
                        }
                        
                        if ui.button("Select none").clicked() {
                            set_all = Some(false);
                        }
                    });
                    
                    if let Some(set_all) = set_all {
                        let indexes = self.characters.objects().iter().map(Character::index).collect::<Vec<_>>();
                        for i in indexes {
                            let Some(settings) = self.get_character_settings_mut(i) else {
                                continue;
                            };
                            settings.show_rng_rolls = set_all;
                        }
                    }
                    
                    let mut checkboxes = Vec::with_capacity(self.characters.len());
                    for character in self.characters.objects() {
                        let i = character.index();
                        let name = character.name();
                        checkboxes.push((i, format!("#{i}: {name}")));
                    }
                    
                    for (i, name) in checkboxes {
                        let Some(settings) = self.get_character_settings_mut(i) else {
                            continue;
                        };
                        ui.checkbox(&mut settings.show_rng_rolls, name);
                    }
                });
            }
            
            ui.separator();
            
            // show in reverse order so newest items are at the top
            for frame in rng_descriptions.into_iter().rev() {
                egui::CollapsingHeader::new(format!("{} ({}) | Rolls: {}", frame.timestamp, frame.frame_index, frame.rng_descriptions.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        for mut roll in frame.rng_descriptions.into_iter().rev() {
                            let show = match roll.category {
                                RollCategory::Character(i) => { 
                                    self.config.show_character_rng && self.get_character_settings(i as usize).map(|s| s.show_rng_rolls()).unwrap_or(true)
                                }
                                RollCategory::NonCharacter => self.config.show_known_non_character_rng,
                                RollCategory::Unknown => self.config.show_unknown_rng,
                            };
                            
                            if !show {
                                continue;
                            }
                            
                            ui.label(roll.description.take()).context_menu(|ui| {
                                ui.label(format!("RNG position: {}", roll.rng_index()));
                                if roll.category == RollCategory::Unknown {
                                    // we don't have any other info to show for unknown rolls
                                    return;
                                }
                                
                                if let Some((index, distance, value)) = roll.next_unique_value() {
                                    ui.label(format!("Next unique value: {value} (+{distance}, position {index})"));
                                }
                                
                                if let Some((index, distance, value)) = roll.prev_unique_value() {
                                    ui.label(format!("Previous unique value: {value} ({distance}, position {index})"));
                                }

                                if ui.button("Explore").clicked() {
                                    self.open_rng_explore_window(roll.roll_type.unwrap(), roll.rng_index());
                                    ui.close_menu();
                                }
                            });
                        }
                    });
            }
        });
    }

    fn settings_browser(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            ui.checkbox(&mut self.config.focus_current_selected_object, "Focus for current selection");
            ui.checkbox(&mut self.config.alternate_collision_colors, "Alternate collision colors");
            if ui.checkbox(&mut self.config.default_show_character_tooltips, "Show character tooltips by default").clicked() {
                // when this setting is changed, update all character settings to the new value
                for character_settings in self.character_settings.values_mut() {
                    character_settings.show_tooltip = self.config.default_show_character_tooltips;
                }
            }
            ui.checkbox(&mut self.config.show_sounds, "Show sounds");
            ui.separator();

            for (object_type, object_settings) in &mut self.config.object_settings {
                ui.label(RichText::new(object_type.name()).strong());
                ui.checkbox(&mut object_settings.show, "Show");
                egui::widgets::color_picker::color_picker_color32(ui, &mut object_settings.color, Alpha::OnlyBlend);
                ui.separator();
            }
        });
    }

    fn get_character(&self, index: usize) -> Option<&Character> {
        for character in self.characters.objects() {
            if character.index() == index {
                return Some(character);
            }
        }

        None
    }
    
    fn get_object(&self, index: usize) -> Option<&Object> {
        for object in self.objects.objects() {
            if object.index() == index {
                return Some(object);
            }
        }
        
        None
    }

    fn get_character_settings(&self, index: usize) -> Option<CharacterSettings> {
        let room_id = self.active_recording().and_then(Recording::current_state).map(State::room_id)?;
        let character_id = self.get_character(index)?.id;
        Some(self.character_settings.get(&(room_id, character_id, index)).copied().unwrap_or_else(|| CharacterSettings::config_default(&self.config)))
    }

    fn get_character_settings_mut(&mut self, index: usize) -> Option<&mut CharacterSettings> {
        let room_id = self.active_recording().and_then(Recording::current_state).map(State::room_id)?;
        let character_id = self.get_character(index)?.id;
        Some(self.character_settings.entry((room_id, character_id, index)).or_insert_with(|| CharacterSettings::config_default(&self.config)))
    }

    fn object_details(&mut self, ui: &mut Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            let description = match self.selected_object {
                SelectedObject::Floor(i) => self.floors[i].details(),
                SelectedObject::Entity(i) => self.entities[i].details(),
                SelectedObject::Collider(i) => self.colliders[i].details(),
                SelectedObject::Object(i) => match self.get_object(i) {
                    Some(object) => object.details(),
                    None => vec![],
                }
                SelectedObject::AiZone(i) => self.ai_zones[i].details(),
                SelectedObject::Character(i) => match self.get_character(i) {
                    Some(character) => character.details(),
                    None => vec![],
                },
                SelectedObject::None => return,
            };

            if description.is_empty() {
                return;
            }

            let mut groups = description.into_iter();
            let (mut group_name, fields) = groups.next().unwrap();
            let mut field_iter = fields.into_iter();
            let mut is_group_start = true;
            let mut is_group_end = false;

            ui.horizontal(|ui| {
                loop {
                    ui.vertical(|ui| {
                        if is_group_start {
                            ui.label(RichText::new(group_name.clone()).strong());
                            is_group_start = false;
                        } else {
                            ui.label("");
                        }

                        let mut num_rows = 0;
                        loop {
                            match field_iter.next() {
                                Some(field) => {
                                    ui.label(field);
                                    num_rows += 1;

                                    if num_rows >= DETAIL_MAX_ROWS {
                                        break;
                                    }
                                }
                                None => {
                                    is_group_end = true;
                                    while num_rows < DETAIL_MAX_ROWS {
                                        ui.label("");
                                        num_rows += 1;
                                    }
                                    break;
                                }
                            }
                        }
                    });

                    if is_group_end {
                        let Some(group) = groups.next() else {
                            break;
                        };

                        group_name = group.0;
                        field_iter = group.1.into_iter();
                        is_group_start = true;
                        is_group_end = false;

                        ui.separator();
                    }
                }

                if let SelectedObject::Character(i) = self.selected_object {
                    if let Some(settings) = self.get_character_settings_mut(i) {
                        // extra display options for characters
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Display").strong());
                            ui.checkbox(&mut settings.show, "Show character");
                            ui.checkbox(&mut settings.show_tooltip, "Show tooltip");
                            ui.checkbox(&mut settings.show_ai, "Show AI");
                        });
                        ui.vertical(|ui| {
                            ui.label("");
                            ui.checkbox(&mut settings.show_path, "Show path");
                        });
                    }
                }

                ui.shrink_height_to_current();
            });
        });
    }

    fn update_from_state(&mut self) {
        let Some(next_state) = self.active_recording().and_then(Recording::current_state) else {
            return;
        };
        let new_room_id = next_state.room_id();

        let mut ai_zones = Vec::with_capacity(NUM_CHARACTERS);
        let mut characters = Vec::with_capacity(NUM_CHARACTERS);

        for (i, character) in next_state.characters().iter().enumerate() {
            let Some(character) = character.as_ref() else {
                continue;
            };

            let mut character = character.clone();
            character.set_index(i);

            let character_ai_zones = character.ai_zones();

            characters.push(character);
            ai_zones.extend(character_ai_zones);
        }

        let mut objects = Vec::with_capacity(NUM_OBJECTS);
        for (i, object) in next_state.objects().iter().enumerate() {
            let Some(object) = object.as_ref() else {
                continue;
            };

            // we don't care about displaying arbitrary 3D objects
            if !object.is_pushable() {
                continue;
            }

            let mut object = object.clone();
            object.set_index(i);
            objects.push(object);
        }

        self.characters.set_objects(characters);
        self.ai_zones.set_objects(ai_zones);
        self.objects.set_objects(objects);

        if self.config.last_rdt != Some(new_room_id) {
            if let Err(e) = self.load_room(new_room_id) {
                self.show_error(format!("Failed to load room {new_room_id}: {e}"));
            }
        }
    }

    fn change_recording_frame<F>(&mut self, func: F)
    where F: FnOnce(&mut Recording) -> Option<&State>
    {
        self.last_play_tick = Instant::now();
        if self.active_recording_mut().and_then(func).is_none() {
            // pause once we reach the end of the recording
            self.is_recording_playing = false;
        } else {
            self.update_from_state();
        }
    }

    fn prev_recording_frame(&mut self) -> bool {
        if let Some(comparison) = self.comparison.as_mut() {
            let range = comparison.active_run().range();
            let index = comparison.recording().index();
            if index <= range.start {
                comparison.set_playback_index(0);
                return false;
            }

            comparison.retreat_playback();
        }

        self.change_recording_frame(Recording::prev);
        true
    }

    fn next_recording_frame(&mut self) -> bool {
        if let Some(comparison) = self.comparison.as_mut() {
            let range = comparison.active_run().range();
            let next_index = comparison.recording().index() + 1;
            comparison.advance_playback();
            if next_index >= range.end {
                return !comparison.is_playback_complete();
            }
        }

        self.change_recording_frame(Recording::next);
        true
    }

    fn set_recording_frame(&mut self, mut index: usize) {
        if let Some(comparison) = self.comparison.as_mut() {
            let range = comparison.active_run().range();
            comparison.set_playback_index(index.saturating_sub(range.start));
            if index < range.start {
                index = range.start;
            } else if index >= range.end {
                index = range.end - 1;
            }
        }

        self.change_recording_frame(|recording| recording.set_index(index));
    }
    
    fn move_recording_frame(&mut self, delta: isize) {
        let Some(index) = self.active_recording().map(Recording::index) else {
            return;
        };
        
        let new_index = (index as isize + delta).max(0) as usize;
        self.set_recording_frame(new_index);
    }
    
    fn fade_focus<O: GameObject>(&self, draw_params: &mut DrawParams, object: &O) {
        if self.config.focus_current_selected_object {
            let floor = match self.selected_object {
                SelectedObject::Floor(i) => self.floors[i].floor(),
                SelectedObject::Collider(i) => self.colliders[i].floor(),
                SelectedObject::Entity(i) => self.entities[i].floor(),
                SelectedObject::AiZone(i) => self.ai_zones[i].floor(),
                SelectedObject::Object(i) => match self.get_object(i) {
                    Some(object) => object.floor(),
                    None => return,
                }
                SelectedObject::Character(i) => match self.get_character(i) {
                    Some(character) => character.floor(),
                    None => return,
                }
                SelectedObject::None => return,
            };

            if !floor.matches(object.floor()) {
                draw_params.fade(UNFOCUSED_FADE);
            }
        }
    }
    
    fn adjust_draw_for_selection<O: GameObject>(&self, draw_params: &mut DrawParams, object: &O, index: usize) -> bool {
        if self.selected_object.matches(object, index) {
            draw_params.highlight();
            true
        } else {
            self.fade_focus(draw_params, object);
            false
        }
    }

    fn get_sound_text_box(sound: &PlayerSound, draw_params: &DrawParams, ui: &Ui) -> egui::Shape {
        let (x, y, _, _) = draw_params.transform(sound.pos.x, sound.pos.z, UFixed16(0), UFixed16(0));
        let pos = egui::Pos2::new(x, y);

        let age = 1.0 - (sound.age as f32 / MAX_SOUND_AGE as f32);

        let bg_color = draw_params.fill_color.gamma_multiply(age);
        let text_color = draw_params.stroke.color.gamma_multiply(age);

        let mut sounds = Vec::new();

        if sound.sounds.is_gunshot_audible() {
            sounds.push("🔫");
        }

        if sound.sounds.is_walking_footstep_audible() {
            sounds.push("👞");
        }

        if sound.sounds.is_running_footstep_audible() {
            sounds.push("👟");
        }

        if sound.sounds.is_knife_audible() {
            sounds.push("🔪");
        }

        if sound.sounds.is_aim_audible() {
            sounds.push("🎯");
        }

        let sound_string = sounds.join("\n");

        let (bg, text) = text_box(sound_string, pos, VAlign::Center, bg_color, text_color, ui);

        egui::Shape::Vec(vec![bg, text])
    }

    fn draw_key(ui: &mut Ui, text: &str, pos: egui::Pos2, is_pressed: bool) {
        let (bg_color, text_color) = if is_pressed {
            (TEXT_BOX_LIGHT, TEXT_BOX_DARK)
        } else {
            (TEXT_BOX_DARK, TEXT_BOX_LIGHT)
        };
        let shape = text_box(text, pos, VAlign::Center, bg_color, text_color, ui);
        ui.painter().add(egui::Shape::Vec(vec![shape.0, shape.1]));
    }

    fn title(&self) -> String {
        match (self.config.rdt_folder.as_ref(), self.config.last_rdt) {
            (Some(folder), Some(id)) => format!("{} - {} - {}", APP_NAME, id, folder.display()),
            (Some(folder), None) => format!("{} - {}", APP_NAME, folder.display()),
            _ => APP_NAME.to_string(),
        }
    }

    fn show_error(&mut self, error: impl Display) {
        self.error_message = Some(error.to_string());
        // if a recording is playing, pause it
        self.is_recording_playing = false;
    }

    fn error_modal(&mut self, ctx: &Context) {
        let Some(ref error_message) = self.error_message else {
            return;
        };

        let response = egui::Modal::new(egui::Id::new("Error Modal")).show(ctx, |ui| {
            ui.label(RichText::new("Error").strong());
            ui.separator();
            ui.vertical_centered(|ui| {
                ui.label(error_message);
                ui.button("OK").clicked()
            }).inner
        });

        if response.should_close() || response.inner {
            self.error_message = None;
        }
    }

    fn connecting_rooms(&self) -> Vec<RoomId> {
        let mut connecting_rooms = Vec::new();
        let Some(this_room_id) = self.config.last_rdt else {
            return connecting_rooms;
        };

        for entity in self.entities.objects() {
            let EntityForm::Door { next_stage, next_room, .. } = entity.form() else {
                continue;
            };

            let other_room_id = RoomId::new(*next_stage, *next_room, this_room_id.player);
            if other_room_id != this_room_id && !connecting_rooms.contains(&other_room_id) {
                connecting_rooms.push(other_room_id);
            }
        }

        connecting_rooms
    }

    fn aot_names(&self) -> [Option<String>; NUM_AOTS] {
        let mut aot_names = [const { None }; NUM_AOTS];

        let mut door_count = 0usize;
        let mut item_count = 0usize;
        let mut other_count = 0usize;
        for entity in self.entities.objects() {
            let aot = entity.id() as usize;
            if aot >= NUM_AOTS {
                eprintln!("Invalid AOT: {}", entity.id());
                continue;
            }

            let name = match entity.object_type() {
                ObjectType::Door => {
                    let s = format!("#{aot} Door {door_count}");
                    door_count += 1;
                    s
                }
                ObjectType::Item => {
                    let s = format!("#{aot} Item {item_count}");
                    item_count += 1;
                    s
                }
                _ => {
                    let s = format!("#{aot} AOT {other_count}");
                    other_count += 1;
                    s
                }
            };

            let aot_name = &mut aot_names[aot];
            if let Some(aot_name) = aot_name {
                if *aot_name != name {
                    eprintln!("Conflicting name for AOT {}: {} vs {}", aot, name, aot_name);
                }
            }

            *aot_name = Some(name);
        }

        aot_names
    }

    fn room_filter_dropdown(ui: &mut Ui, label: &str, rooms: &[RoomId], current_room: &mut Option<RoomId>) {
        let selected_text = match current_room {
            Some(entrance_id) => entrance_id.to_string(),
            None => "None".to_string(),
        };

        egui::ComboBox::from_label(label)
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(current_room, None, "None");
                for room_id in rooms {
                    ui.selectable_value(current_room, Some(*room_id), room_id.to_string());
                }
            });
    }

    fn start_comparison(&mut self, comparison: Comparison) {
        self.comparison = Some(comparison);
        self.update_from_state();
    }

    fn select_comparison_recordings(&mut self) -> Result<()> {
        let Some(recording_paths) = FileDialog::new().add_filter("RE2 recordings", &["bin"]).pick_files() else {
            // user canceled the dialog, so just bail
            return Ok(());
        };

        let entities = self.entities.objects();
        let comparison = Comparison::load_runs(recording_paths, &self.compare_filter, entities)?;

        // close any active individual recording
        self.close_recording();

        self.start_comparison(comparison);

        Ok(())
    }

    fn open_rng_explore_window(&mut self, roll_type: RollType, rng_index: usize) {
        if self.rng_selected_roll_type != Some(roll_type) {
            self.rng_selected_outcomes.clear();
        }

        self.rng_selected_roll_type = Some(roll_type);
        self.rng_selected_index = rng_index;
        self.is_rng_explore_window_open = true;
    }

    fn rng_explore_window(&mut self, ctx: &Context) {
        let mut is_rng_explore_window_open = self.is_rng_explore_window_open;
        
        egui::Window::new("Explore RNG")
            .open(&mut is_rng_explore_window_open)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let old_roll_type = self.rng_selected_roll_type;
                egui::ComboBox::from_label("Roll type")
                    .selected_text(match self.rng_selected_roll_type {
                        Some(roll_type) => format!("{:?}", roll_type),
                        None => "None".to_string(),
                    })
                    .show_ui(ui, |ui| {
                        for (roll_type, description) in ROLL_DESCRIPTIONS.iter() {
                            if matches!(roll_type, RollType::Partial | RollType::Invalid) {
                                continue;
                            }
                            
                            ui.selectable_value(&mut self.rng_selected_roll_type, Some(roll_type), format!("{:?} ({})", roll_type, description.label("<Character>")));
                        }
                    });
                
                if self.rng_selected_roll_type != old_roll_type {
                    self.rng_selected_outcomes.clear();
                }
                
                ui.add(egui::Slider::new(&mut self.rng_selected_index, 0..=(RNG_SEQUENCE.len() - 1)).text("RNG position"));
                
                let roll = match self.rng_selected_roll_type {
                    Some(t) => {
                        let desc = &ROLL_DESCRIPTIONS[t];
                        RngDescription::new(desc.label("<Character>"), if desc.has_subject() { RollCategory::Character(0) } else { RollCategory::NonCharacter }, Some(t), RNG_SEQUENCE[self.rng_selected_index])
                    }
                    None => return,
                };

                ui.separator();

                let options = roll.options();

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Distribution");

                        ui.add_enabled(!self.rng_selected_outcomes.is_empty(), egui::Checkbox::new(&mut self.rng_distribution_binary, "By desired outcome"));

                        let half_range = (RNG_SEQUENCE.len() / 2) as isize;
                        ui.add(egui::Slider::new(&mut self.rng_distribution_range_min, -half_range..=0).text("Min"));
                        ui.add(egui::Slider::new(&mut self.rng_distribution_range_max, 0..=half_range).text("Max"));

                        let distribution = roll.distribution(self.rng_distribution_range_min, self.rng_distribution_range_max);
                        let total = distribution.iter().map(|d| d.1).sum::<usize>() as f32;

                        if self.rng_distribution_binary {
                            let mut count_desired = 0usize;
                            for (value, count) in distribution {
                                if self.rng_selected_outcomes.contains(value.as_str()) {
                                    count_desired += count;
                                }
                            }

                            let desired_percent = count_desired as f32 / total * 100.0;
                            ui.label(format!("Desired: {desired_percent:.2}%"));
                        } else {
                            for (value, count) in distribution {
                                let percent = count as f32 / total * 100.0;
                                ui.label(format!("{value}: {percent:.2}%"));
                            }
                        }
                    });

                    if !options.is_empty() {
                        ui.separator();

                        ui.vertical(|ui| {
                            ui.label("Desired outcomes");

                            for option in options {
                                let mut is_checked = self.rng_selected_outcomes.contains(option);
                                ui.checkbox(&mut is_checked, String::from(*option));
                                if is_checked {
                                    self.rng_selected_outcomes.insert(*option);
                                } else {
                                    self.rng_selected_outcomes.retain(|o| o != option);
                                }
                            }
                        });
                    }
                });
                
                if self.rng_selected_outcomes.is_empty() {
                    return;
                }

                let values = roll
                    .values_in_range(self.rng_distribution_range_min, self.rng_distribution_range_max)
                    .into_iter()
                    .map(|(i, v)| (i, self.rng_selected_outcomes.contains(v.as_str())))
                    .collect::<Vec<_>>();
                
                if values.is_empty() {
                    return;
                }
                
                ui.separator();
                
                ui.label("Runs");
                ui.add(egui::Slider::new(&mut self.rng_run_threshold, 0.0..=100.0).text("Threshold"));
                ui.add(egui::Slider::new(&mut self.rng_run_window_size, 2..=100).text("Window size"));
                
                let mut prefixes = Vec::with_capacity(values.len() + 1);
                prefixes.push(0usize);
                for (_, value) in &values {
                    prefixes.push(prefixes.last().unwrap() + if *value { 1 } else { 0 });
                }

                let num_points = values.len().saturating_sub(self.rng_run_window_size).max(1);
                let mut points = Vec::with_capacity(num_points);
                let mut threshold_regions: Vec<Range<usize>> = Vec::new();
                let f_window = self.rng_run_window_size as f64;
                for i in 0..num_points {
                    let end = (i + self.rng_run_window_size).min(values.len());
                    let desired_count = prefixes[end] - prefixes[i];
                    let desired_percent = (desired_count as f64 / f_window) * 100.0;
                    let index = values[i].0;
                    points.push([index as f64, desired_percent]);

                    if desired_percent >= self.rng_run_threshold {
                        if let Some(region) = threshold_regions.last_mut() && i < region.end {
                            region.end = end;
                        } else {
                            threshold_regions.push(i..end);
                        }
                    }
                }

                ui.collapsing("Run list", |ui| {
                    for region in threshold_regions {
                        let size = region.end - region.start;
                        let desired_count = prefixes[region.end] - prefixes[region.start];
                        let desired_percent = (desired_count as f64 / (size as f64)) * 100.0;
                        let start = values[region.start].0;
                        let end = values[region.end - 1].0;
                        ui.label(format!("{start} to {end} (size = {size}; {desired_percent:.2}%)"));
                    }
                });

                let desired_line = Line::new("desired", points);
                let threshold_line = Line::new("threshold", vec![
                    [values[0].0 as f64, self.rng_run_threshold],
                    [values.last().unwrap().0 as f64, self.rng_run_threshold],
                ]);
                Plot::new("rng_runs")
                    .x_axis_label("RNG position")
                    .y_axis_label("Desired %")
                    .show(ui, |plot_ui| {
                        plot_ui.line(desired_line);
                        plot_ui.line(threshold_line);
                    });
            });

        if self.is_rng_explore_window_open {
            self.is_rng_explore_window_open = is_rng_explore_window_open;
        }
    }

    fn compare_filter_window(&mut self, ctx: &Context) {
        let mut is_compare_filter_window_open = self.is_compare_filter_window_open;

        egui::Window::new("Compare Runs")
            .open(&mut is_compare_filter_window_open)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label(RichText::new(format!("Room {}", self.compare_filter.room_id)).strong());

                ui.separator();

                let connecting_rooms = self.connecting_rooms();

                Self::room_filter_dropdown(ui, "Entrance filter", &connecting_rooms, &mut self.compare_filter.entrance_id);
                Self::room_filter_dropdown(ui, "Exit filter", &connecting_rooms, &mut self.compare_filter.exit_id);

                ui.separator();

                ui.label(RichText::new("Required triggers").strong());

                ui.separator();

                if !self.compare_filter.checkpoints.is_empty() {
                    let aot_names = self.aot_names();
                    let end_index = self.compare_filter.checkpoints.len().saturating_sub(1);
                    let mut edit = None;
                    for (i, checkpoint) in self.compare_filter.checkpoints.iter_mut().enumerate() {
                        let Checkpoint::Aot(aot) = checkpoint;
                        let aot = *aot as usize;
                        let Some(name) = aot_names.get(aot).and_then(Option::as_ref) else {
                            eprintln!("Checkpoint {} has invalid AOT {}", i, aot);
                            continue;
                        };

                        ui.horizontal(|ui| {
                            let delete_button = egui::Button::new("⊗").fill(Color32::RED);
                            if ui.add(delete_button).clicked() {
                                edit = Some((i, 0isize));
                            }

                            ui.separator();

                            if ui.add_enabled(i > 0, egui::Button::new("⏶")).clicked() {
                                edit = Some((i, -1isize));
                            }

                            if ui.add_enabled(i < end_index, egui::Button::new("⏷")).clicked() {
                                edit = Some((i, 1isize));
                            }

                            egui::ComboBox::from_label(format!("Trigger {}", i + 1))
                                .selected_text(name)
                                .show_ui(ui, |ui| {
                                    for (aot, name) in aot_names.iter().enumerate() {
                                        let Some(name) = name else {
                                            continue;
                                        };

                                        ui.selectable_value(checkpoint, Checkpoint::Aot(aot as u8), name);
                                    }
                                });
                        });
                    }

                    if let Some((i, delta)) = edit {
                        if delta == 0 {
                            self.compare_filter.checkpoints.remove(i);
                        } else if let Some(neighbor) = i.checked_add_signed(delta) {
                            self.compare_filter.checkpoints.swap(i, neighbor);
                        }
                    }
                } else {
                    ui.label("None");
                }

                ui.separator();

                if ui.button("Add trigger").clicked() {
                    self.compare_filter.checkpoints.push(Checkpoint::Aot(0));
                }

                ui.separator();

                ui.vertical_centered(|ui| {
                    ui.add_space(5.0);
                    if ui.button("Confirm and select recordings").clicked() {
                        self.is_compare_filter_window_open = false;
                        if let Err(e) = self.select_comparison_recordings() {
                            self.show_error(format!("Failed to open comparison recordings: {}", e));
                        }
                    }
                    ui.add_space(5.0);
                });
            });

        if self.is_compare_filter_window_open {
            self.is_compare_filter_window_open = is_compare_filter_window_open;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if self.need_title_update {
            ctx.send_viewport_cmd(ViewportCommand::Title(self.title()));
            self.need_title_update = false;
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open game folder").clicked() {
                        if let Err(e) = self.prompt_load_game() {
                            self.show_error(format!("Failed to open RDT: {e}"));
                        }
                        ui.close_menu();
                    }

                    if ui.button("Open recording").clicked() && self.is_game_loaded() {
                        if let Err(e) = self.prompt_load_recording() {
                            self.show_error(format!("Failed to open recording: {e}"));
                        }
                        ui.close_menu();
                    }
                    
                    ui.separator(); // don't want open button too close to close button
                    
                    if self.comparison.is_some() {
                        if ui.button("Close comparison").clicked() {
                            self.close_comparison();
                            ui.close_menu();
                        }
                    } else if ui.add_enabled(self.active_recording.is_some(), egui::Button::new("Close recording")).clicked() {
                        self.close_recording();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Compare runs").clicked() {
                        let room_id = self.config.last_rdt.unwrap_or_else(RoomId::zero);
                        if self.compare_filter.room_id != room_id {
                            self.compare_filter = RoomFilter::basic(room_id);
                        }
                        self.is_compare_filter_window_open = true;
                        ui.close_menu();
                    }

                    if ui.button("Explore RNG").clicked() {
                        self.is_rng_explore_window_open = true;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::SidePanel::left("browser").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    for tab in BrowserTab::list() {
                        let is_tab_inactive = (tab == BrowserTab::Recording && self.active_recording.is_none())
                            || (tab == BrowserTab::Comparison && self.comparison.is_none())
                            || (tab == BrowserTab::Rng && self.active_recording().is_none());
                        
                        if is_tab_inactive {
                            continue;
                        }

                        if ui.selectable_label(self.tab == tab, tab.name()).clicked() {
                            self.tab = tab;
                        }
                    }
                });
                ui.separator();
                match self.tab {
                    BrowserTab::Game => self.rdt_browser(ui),
                    BrowserTab::Room => self.room_browser(ui),
                    BrowserTab::Settings => self.settings_browser(ui),
                    BrowserTab::Rng => self.rng_browser(ui),
                    BrowserTab::Recording => self.recording_browser(ui),
                    BrowserTab::Comparison => self.comparison_browser(ui),
                }
            });
        });

        egui::TopBottomPanel::bottom("detail").show(ctx, |ui| {
            let width = ui.max_rect().width();
            ui.vertical(|ui| {
                let mut need_toggle = false;
                let mut new_frame_index = None;

                let play_pause = if self.is_recording_playing {
                    "⏸"
                } else {
                    "▶"
                };

                if let Some(recording) = self.active_recording_mut() {
                    ui.horizontal(|ui| {
                        need_toggle = ui.button(play_pause).clicked();

                        let mut pos = recording.index();
                        let num_frames = recording.frames().len();
                        let time = recording.current_frame().map(FrameRecord::time).unwrap_or_else(|| String::from("00:00:00"));
                        ui.style_mut().spacing.slider_width = width * 0.6;
                        ui.add(egui::Slider::new(&mut pos, 0..=num_frames).text(time));
                        if pos != recording.index() {
                            new_frame_index = Some(pos);
                        }
                    });
                    ui.separator();
                }

                if need_toggle {
                    self.toggle_play_recording();
                }

                if let Some(index) = new_frame_index {
                    self.set_recording_frame(index);
                }

                self.object_details(ui);
                
                ui.separator();
                
                if let Some(pos) = self.pointer_game_pos {
                    ui.label(format!("X: {}, Z: {}", pos.x, pos.z));
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.ui_contains_pointer() {
                self.handle_input(ctx);
            }
            
            let view_center = self.calculate_origin(ctx);
            let empty_state = State::empty();
            let state = self.active_recording().and_then(Recording::current_state).unwrap_or(&empty_state);

            for (i, floor) in self.floors.visible_objects(&self.config) {
                let mut floor_draw_params = self.config.get_obj_draw_params(floor, view_center);
                // unlike the other object types, we don't draw the floor on top when it's highlighted
                // because it covers everything up and makes it hard to tell what's actually on the
                // given floor
                self.adjust_draw_for_selection(&mut floor_draw_params, floor, i);

                ui.draw_game_object(floor, &floor_draw_params, state);
            }

            for (i, collider) in self.colliders.visible_objects(&self.config) {
                let mut collider_draw_params = self.config.get_obj_draw_params(collider, view_center);
                if self.adjust_draw_for_selection(&mut collider_draw_params, collider, i) {
                    continue;
                }

                ui.draw_game_object(collider, &collider_draw_params, state);
            }

            for (i, entity) in self.entities.visible_objects(&self.config) {
                let mut entity_draw_params = self.config.get_obj_draw_params(entity, view_center);
                if self.adjust_draw_for_selection(&mut entity_draw_params, entity, i) {
                    continue;
                }

                ui.draw_game_object(entity, &entity_draw_params, state);
            }

            for (_, object) in self.objects.visible_objects(&self.config) {
                let mut object_draw_params = self.config.get_obj_draw_params(object, view_center);
                if self.adjust_draw_for_selection(&mut object_draw_params, object, object.index()) {
                    continue;
                }
                
                ui.draw_game_object(object, &object_draw_params, state);
            }

            // draw all AI zones first, then all characters, so characters are always on top of the zones
            for (i, ai_zone) in self.ai_zones.visible_objects(&self.config) {
                let (Some(character), Some(settings)) = (state.characters()[ai_zone.character_index].as_ref(), self.get_character_settings(ai_zone.character_index)) else {
                    // the character must not be none because otherwise we wouldn't have AI zones for them
                    eprintln!("AI zone {} has no character (expected character {} at index {})", i, ai_zone.character_id.name(), ai_zone.character_index);
                    continue;
                };
                // if the character the AI zones belong to isn't shown here, we shouldn't show the AI zones either
                if !self.config.should_show(character.object_type()) || !settings.show_ai() {
                    continue;
                }

                let mut ai_draw_params = self.config.get_obj_draw_params(ai_zone, view_center);
                if self.adjust_draw_for_selection(&mut ai_draw_params, ai_zone, i) {
                    continue;
                }
                
                ui.draw_game_object(ai_zone, &ai_draw_params, state);
            }

            // if the current selected object is a character, and that character has AI zones, draw those
            // zones after all other zones, but still before characters, because we always want those to
            // be on top
            if let SelectedObject::Character(i) = self.selected_object {
                if let (Some(character), Some(settings)) = (state.characters()[i].as_ref(), self.get_character_settings(i)) {
                    if self.config.should_show(character.object_type()) && settings.show_ai() {
                        for (j, ai_zone) in self.ai_zones.visible_objects(&self.config) {
                            if ai_zone.character_index != i {
                                continue;
                            }

                            let mut ai_draw_params = self.config.get_obj_draw_params(ai_zone, view_center);
                            self.adjust_draw_for_selection(&mut ai_draw_params, ai_zone, j);
                            ui.draw_game_object(ai_zone, &ai_draw_params, state);
                        }
                    }
                }
            }
            
            // also draw paths before characters so the paths are under the characters
            for (_, character) in self.characters.visible_objects(&self.config) {
                if !self.get_character_settings(character.index()).map(|s| s.show_path()).unwrap_or(false) {
                    continue;
                }

                if character.index() == 0 && self.comparison.is_some() {
                    // don't draw the normal path for the player if we're drawing comparison paths
                    continue;
                }
                
                if let Some(path) = self.active_recording().and_then(|r| r.get_path_for_character(character.index())) {
                    let mut path_draw_params = self.config.get_obj_draw_params(&path, view_center);
                    path_draw_params.stroke.width = character.size.x * self.config.zoom_scale * 2.0;
                    ui.draw_game_object(&path, &path_draw_params, state);
                }
            }

            // draw comparison paths if we're doing a comparison
            if let (Some(comparison), true) = (&self.comparison, self.show_comparison_paths) {
                let fastest_time = comparison.fastest_time();
                let time_range = (comparison.slowest_time() - fastest_time).max(1) as f32;

                // we iterate in reverse order so faster runs are drawn on top
                for run in comparison.runs_desc() {
                    // active run is drawn last so it's always on top
                    if !run.is_included() || comparison.is_active_run(run) {
                        continue;
                    }

                    let path = run.route();
                    let mut path_draw_params = self.config.get_obj_draw_params(path, view_center);

                    let time = run.len();
                    if time == fastest_time {
                        // fastest run is gold and has a slightly thicker line
                        path_draw_params.stroke.color = Color32::from_rgb(0xFF, 0xD7, 0x00);
                        path_draw_params.stroke.width = COMPARISON_PATH_EMPHASIS_WIDTH * self.config.zoom_scale;
                    } else {
                        // other runs are color-coded from green to red and opaque to transparent
                        // based on how fast they are
                        let ratio = (time - fastest_time) as f32 / time_range;
                        let red = (ratio * 255.0) as u8;
                        let green = 255 - red;
                        let alpha = (green >> 1) + 0x80;
                        path_draw_params.stroke.color = Color32::from_rgba_unmultiplied(red, green, 0, alpha);
                        path_draw_params.stroke.width = COMPARISON_PATH_WIDTH * self.config.zoom_scale;
                    }

                    ui.draw_game_object(path, &path_draw_params, state);
                }

                // draw active run last
                let run = comparison.active_run();
                let path = run.route();
                let mut path_draw_params = self.config.get_obj_draw_params(path, view_center);

                path_draw_params.stroke.color = if run.len() == fastest_time {
                    // fastest run is gold and has a slightly thicker line
                    Color32::from_rgb(0xFF, 0xD7, 0x00)
                } else {
                    // if the user has selected a run other than the fastest run, draw it in blue
                    Color32::from_rgb(0x00, 0x96, 0xFF)
                };
                path_draw_params.stroke.width = COMPARISON_PATH_EMPHASIS_WIDTH * self.config.zoom_scale;
                ui.draw_game_object(path, &path_draw_params, state);
            }
            
            // draw player's equipped weapon ranges if enabled
            if let Some(range_visualization) = WeaponRangeVisualization::for_state(state) {
                if self.config.should_show(range_visualization.object_type()) {
                    let mut range_draw_params = self.config.get_obj_draw_params(&range_visualization, view_center);
                    range_draw_params.stroke.width *= 2.0;
                    range_draw_params.stroke_kind = StrokeKind::Inside;
                    ui.draw_game_object(&range_visualization, &range_draw_params, state);
                }
            }

            for (_, character) in self.characters.visible_objects(&self.config) {
                let mut char_draw_params = self.config.get_obj_draw_params(character, view_center);
                if self.adjust_draw_for_selection(&mut char_draw_params, character, character.index()) || !self.get_character_settings(character.index()).map(|s| s.show).unwrap_or(false) {
                    continue;
                }

                ui.draw_game_object(character, &char_draw_params, state);
            }

            // draw character tooltips on top of the characters themselves
            for (_, character) in self.characters.visible_objects(&self.config) {
                let i = character.index();
                if self.selected_object.matches(character, i) || !self.get_character_settings(i).map(|s| s.show_tooltip()).unwrap_or(false) {
                    continue;
                }

                let mut char_draw_params = self.config.get_obj_draw_params(character, view_center);
                self.fade_focus(&mut char_draw_params, character);
                ui.draw_game_tooltip(character, &char_draw_params, state, i);
            }

            if let Some(recording) = self.active_recording() {
                if self.config.show_sounds {
                    // TODO: make sound text box colors configurable
                    let sound_draw_params = DrawParams {
                        origin: view_center,
                        scale: self.config.zoom_scale,
                        fill_color: TEXT_BOX_DARK,
                        stroke: Stroke {
                            color: TEXT_BOX_LIGHT,
                            width: 1.0,
                        },
                        stroke_kind: StrokeKind::Middle,
                        draw_at_origin: false,
                    };

                    for sound in recording.get_player_sounds(MAX_SOUND_AGE) {
                        let sound_box = Self::get_sound_text_box(&sound, &sound_draw_params, ui);
                        ui.painter().add(sound_box);
                    }
                }
            }

            // draw highlighted object (if any) on top
            match self.selected_object {
                SelectedObject::None | SelectedObject::Floor(_) | SelectedObject::AiZone(_) => {}
                SelectedObject::Entity(i) => {
                    let mut entity_draw_params = self.config.get_obj_draw_params(&self.entities[i], view_center);
                    entity_draw_params.highlight();
                    ui.draw_game_object(&self.entities[i], &entity_draw_params, state);
                }
                SelectedObject::Collider(i) => {
                    let mut collider_draw_params = self.config.get_obj_draw_params(&self.colliders[i], view_center);
                    collider_draw_params.highlight();
                    ui.draw_game_object(&self.colliders[i], &collider_draw_params, state);
                }
                SelectedObject::Object(i) => {
                    if let Some(object) = self.get_object(i) {
                        let mut object_draw_params = self.config.get_obj_draw_params(object, view_center);
                        object_draw_params.highlight();
                        ui.draw_game_object(object, &object_draw_params, state);
                    }
                }
                SelectedObject::Character(i) => {
                    if let (Some(character), Some(settings)) = (self.get_character(i), self.get_character_settings(i)) {
                        if settings.show {
                            let char_draw_params = self.config.get_obj_draw_params(character, view_center);
                            ui.draw_game_object(character, &char_draw_params, state);
                            if settings.show_tooltip() {
                                ui.draw_game_tooltip(character, &char_draw_params, state, i);
                            }
                        }
                    }
                }
            }

            // draw hover tooltip
            if let Some(hover_pos) = self.hover_pos {
                match self.hover_object {
                    SelectedObject::None => {}
                    SelectedObject::Floor(i) => {
                        let floor = &self.floors[i];
                        let mut floor_draw_params = self.config.get_obj_draw_params(floor, view_center);
                        floor_draw_params.highlight();
                        floor_draw_params.set_draw_origin(hover_pos);
                        ui.draw_game_tooltip(floor, &floor_draw_params, state, i);
                    }
                    SelectedObject::Entity(i) => {
                        let entity = &self.entities[i];
                        let mut entity_draw_params = self.config.get_obj_draw_params(entity, view_center);
                        entity_draw_params.highlight();
                        entity_draw_params.set_draw_origin(hover_pos);
                        ui.draw_game_tooltip(entity, &entity_draw_params, state, i);
                    }
                    SelectedObject::Collider(i) => {
                        let collider = &self.colliders[i];
                        let mut collider_draw_params = self.config.get_obj_draw_params(collider, view_center);
                        collider_draw_params.highlight();
                        collider_draw_params.set_draw_origin(hover_pos);
                        ui.draw_game_tooltip(collider, &collider_draw_params, state, i);
                    }
                    SelectedObject::Object(i) => {
                        if let Some(object) = self.get_object(i) {
                            let mut object_draw_params = self.config.get_obj_draw_params(object, view_center);
                            object_draw_params.highlight();
                            object_draw_params.set_draw_origin(hover_pos);
                            ui.draw_game_tooltip(object, &object_draw_params, state, i);
                        }
                    }
                    SelectedObject::AiZone(i) => {
                        let ai_zone = &self.ai_zones[i];
                        let mut ai_draw_params = self.config.get_obj_draw_params(ai_zone, view_center);
                        ai_draw_params.highlight();
                        ai_draw_params.set_draw_origin(hover_pos);
                        ui.draw_game_tooltip(ai_zone, &ai_draw_params, state, i);
                    }
                    SelectedObject::Character(i) => {
                        if let (Some(character), Some(settings)) = (self.get_character(i), self.get_character_settings(i)) {
                            // if the character's tooltip setting is on, we've already drawn their tooltip
                            if !settings.show_tooltip() {
                                let mut char_draw_params = self.config.get_obj_draw_params(character, view_center);
                                char_draw_params.set_draw_origin(hover_pos);
                                ui.draw_game_tooltip(character, &char_draw_params, state, i);
                            }
                        }
                    }
                }
            }

            // show player inputs in top right
            if let Some(state) = self.active_recording().and_then(Recording::current_state) {
                let input_state = state.input_state();
                let viewport = ctx.input(egui::InputState::screen_rect);
                let input_origin = viewport.right_top();

                let forward_pos = input_origin + egui::Vec2::new(-INPUT_OFFSET * 2.0, INPUT_SIZE + INPUT_MARGIN * 2.0);
                Self::draw_key(ui, "Fwd", forward_pos, input_state.is_forward_pressed);

                let right_pos = input_origin + egui::Vec2::new(-INPUT_OFFSET, INPUT_SIZE * 2.0 + INPUT_MARGIN * 3.0);
                Self::draw_key(ui, "Rgt", right_pos, input_state.is_right_pressed);

                let back_pos = input_origin + egui::Vec2::new(-INPUT_OFFSET * 2.0, INPUT_SIZE * 2.0 + INPUT_MARGIN * 3.0);
                Self::draw_key(ui, "Bck", back_pos, input_state.is_backward_pressed);

                let left_pos = input_origin + egui::Vec2::new(-INPUT_OFFSET * 3.0, INPUT_SIZE * 2.0 + INPUT_MARGIN * 3.0);
                Self::draw_key(ui, "Lft", left_pos, input_state.is_left_pressed);

                let action_pos = input_origin + egui::Vec2::new(-INPUT_OFFSET * 3.0, INPUT_SIZE * 3.0 + INPUT_MARGIN * 4.0);
                Self::draw_key(ui, "Act", action_pos, input_state.is_action_pressed);

                let run_pos = input_origin + egui::Vec2::new(-INPUT_OFFSET * 2.0, INPUT_SIZE * 3.0 + INPUT_MARGIN * 4.0);
                Self::draw_key(ui, "Run", run_pos, input_state.is_run_cancel_pressed);

                let aim_pos = input_origin + egui::Vec2::new(-INPUT_OFFSET, INPUT_SIZE * 3.0 + INPUT_MARGIN * 4.0);
                Self::draw_key(ui, "Aim", aim_pos, input_state.is_aim_pressed);
            }
        });

        // display modals if necessary
        self.error_modal(ctx);
        self.compare_filter_window(ctx);
        self.rng_explore_window(ctx);

        let repaint_duration = if self.active_recording().is_some() && self.is_recording_playing {
            let now = Instant::now();
            let duration = now - self.last_play_tick;
            if duration >= FRAME_DURATION {
                let previous_room_id = self.config.last_rdt.unwrap();
                if !self.next_recording_frame(){
                    // if we get clamped due to reaching the end of the comparison section and
                    // the other comparison paths are not playing, pause playback
                    self.is_recording_playing = false;
                } else if let Some(player) = self.get_character(0)
                    && player.is_moving()
                    // don't try to project normal movement when the room changes
                    && self.config.last_rdt.unwrap() == previous_room_id {
                    // validate our collision logic
                    let mut motion = player.motion();
                    motion.origin.set_quadrant_mask(self.center);

                    for collider in self.colliders.objects() {
                        motion.to = collider.clip_motion(&motion);
                    }
                    
                    // FIXME: seems to have issues sometimes when running in corners with multiple overlapping colliders
                    if motion.to != player.center {
                        eprintln!(
                            "Player position {:?} on frame {} did not match calculated next position {:?}. Start position {:?}, velocity {:?}, angle {}",
                            player.center, self.active_recording().map(|r| r.index()).unwrap(), motion.to, player.prev_center, player.velocity, player.angle.to_degrees(),
                        );
                    }
                }

                FRAME_DURATION
            } else {
                // schedule a re-draw for the next frame
                FRAME_DURATION - duration
            }
        } else {
            // schedule a re-draw after the hover time expires plus a small margin
            Duration::from_secs_f32(TOOLTIP_HOVER_SECONDS + 0.1)
        };
        
        ctx.request_repaint_after(repaint_duration);
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Err(e) = self.config.save() {
            eprintln!("Failed to save config: {}", e);
        }
    }
}