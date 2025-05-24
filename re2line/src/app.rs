use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::BufReader;
use std::str::FromStr;
use std::time::Instant;

use anyhow::{anyhow, bail, Result};
use eframe::{Frame, Storage};
use egui::{Color32, Context, Key, Ui, ViewportCommand};
use egui::widgets::color_picker::Alpha;
use epaint::{Stroke, StrokeKind};
use re2shared::game::NUM_CHARACTERS;
use re2shared::record::FrameRecord;
use rfd::FileDialog;

use crate::aot::{Entity, SceType};
use crate::character::{Character, CharacterId};
use crate::collision::{Collider, DrawParams};
use crate::draw::{VAlign, text_box};
use crate::math::{Fixed16, Fixed32, UFixed16, Vec2};
use crate::rdt::Rdt;
use crate::record::{PlayerSound, Recording, State, FRAME_DURATION};

mod config;
use config::{Config, ObjectType};
pub use config::RoomId;

pub const APP_NAME: &str = "re2line";

const DETAIL_MAX_ROWS: usize = 4;
const FAST_FORWARD: isize = 30;
const MAX_SOUND_AGE: usize = 100;

const INPUT_MARGIN: f32 = 2.0;
const INPUT_SIZE: f32 = 30.0;
const INPUT_OFFSET: f32 = INPUT_SIZE + INPUT_MARGIN;

const TEXT_BOX_DARK: Color32 = Color32::from_rgb(0x30, 0x30, 0x30);
const TEXT_BOX_LIGHT: Color32 = Color32::from_rgb(0xe0, 0xe0, 0xe0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedObject {
    None,
    Entity(usize),
    Collider(usize),
    Floor(usize),
    Character(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserTab {
    Game,
    Room,
    Settings,
    Rng,
}

impl BrowserTab {
    const fn list() -> [BrowserTab; 4] {
        [BrowserTab::Game, BrowserTab::Room, BrowserTab::Rng, BrowserTab::Settings]
    }

    const fn name(&self) -> &'static str {
        match self {
            Self::Game => "Game",
            Self::Room => "Room",
            Self::Settings => "Settings",
            Self::Rng => "RNG",
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct CharacterSettings {
    pub show_tooltip: bool,
    pub show_ai: bool,
}

impl Default for CharacterSettings {
    fn default() -> Self {
        Self {
            show_tooltip: true,
            show_ai: true,
        }
    }
}

pub struct App {
    center: (Fixed16, Fixed16),
    colliders: Vec<Collider>,
    entities: Vec<Entity>,
    floors: Vec<Collider>,
    pan: egui::Vec2,
    selected_object: SelectedObject,
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
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            center: (Fixed16(0), Fixed16(0)),
            colliders: Vec::new(),
            entities: Vec::new(),
            floors: Vec::new(),
            pan: egui::Vec2::ZERO,
            selected_object: SelectedObject::None,
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
        })
    }

    const fn scale(&self) -> f32 {
        self.config.zoom_scale
    }

    fn toggle_play_recording(&mut self) {
        if self.active_recording.is_none() {
            return;
        }

        self.is_recording_playing = !self.is_recording_playing;
        self.last_play_tick = Instant::now();
    }

    fn click_select(&mut self, pos: Vec2) {
        if let Some(recording) = &self.active_recording {
            if let Some(state) = recording.current_state() {
                for (i, character) in state.characters().iter().enumerate() {
                    let Some(character) = character.as_ref() else {
                        continue;
                    };
                    
                    if character.contains_point(pos) {
                        self.selected_object = SelectedObject::Character(i);
                        return;
                    }
                }
            }
        }

        for (i, entity) in self.entities.iter().enumerate() {
            let object_type: ObjectType = entity.sce().into();
            if !self.config.should_show(object_type) {
                continue;
            }

            if entity.contains_point(pos) {
                self.selected_object = SelectedObject::Entity(i);
                return;
            }
        }
        
        if self.config.should_show(ObjectType::Collider) {
            for (i, collider) in self.colliders.iter().enumerate() {
                if collider.contains_point(pos) {
                    self.selected_object = SelectedObject::Collider(i);
                    return;
                }
            }
        }

        if self.config.should_show(ObjectType::Floor) {
            for (i, floor) in self.floors.iter().enumerate() {
                if floor.contains_point(pos) {
                    self.selected_object = SelectedObject::Floor(i);
                    return;
                }
            }
        }
        
        self.selected_object = SelectedObject::None;
    }
    
    fn screen_pos_to_game_pos(&self, pos: egui::Pos2, viewport: egui::Rect) -> Vec2 {
        let viewport_center = viewport.center().to_vec2();
        let view_relative = (pos + self.pan - viewport_center) / self.scale();
        Vec2::new(Fixed32::from_f32(view_relative.x) + self.center.0.to_32(), -(Fixed32::from_f32(view_relative.y) + self.center.1.to_32()))
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

            self.config.zoom_scale += i.smooth_scroll_delta.y * 0.05;

            if !egui_wants_kb_input {
                if i.key_pressed(Key::Space) {
                    self.toggle_play_recording();
                }

                if self.active_recording.is_some() {
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
            self.center.0 * self.scale() - window_center.x,
            self.center.1 * self.scale() - window_center.y,
        ) + self.pan
    }

    fn set_rdt(&mut self, rdt: Rdt, id: RoomId) {
        let (x, y) = rdt.get_center();
        self.center = (x, -y);
        self.colliders = rdt.get_colliders();
        self.entities = rdt.get_entities();
        self.floors = rdt.get_floors();
        self.pan = egui::Vec2::ZERO;
        self.selected_object = SelectedObject::None;
        self.config.last_rdt = Some(id);
        self.need_title_update = true;
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
        let reader = BufReader::new(file);
        self.active_recording = Some(Recording::read(reader)?);
        // reset character display settings for new recording
        self.character_settings.clear();

        Ok(())
    }

    fn prompt_load_recording(&mut self) -> Result<()> {
        let Some(path) = FileDialog::new().pick_file() else {
            return Ok(());
        };

        self.load_recording(path)
    }
    
    fn close_recording(&mut self) {
        self.active_recording = None;
        self.is_recording_playing = false;
        self.character_settings.clear();
        if matches!(self.selected_object, SelectedObject::Character(_)) {
            self.selected_object = SelectedObject::None;
        }
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

                ui.separator();
            }

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
                for (i, entity) in self.entities.iter().enumerate() {
                    if entity.sce() != SceType::Door {
                        continue;
                    }

                    ui.selectable_value(&mut self.selected_object, SelectedObject::Entity(i), format!("Door {}", door_count));
                    door_count += 1;
                }
            });

            ui.collapsing("Item", |ui| {
                let mut item_count = 0;
                for (i, entity) in self.entities.iter().enumerate() {
                    if entity.sce() != SceType::Item {
                        continue;
                    }

                    ui.selectable_value(&mut self.selected_object, SelectedObject::Entity(i), format!("Item {}", item_count));
                    item_count += 1;
                }
            });

            ui.collapsing("AOT", |ui| {
                let mut aot_count = 0;
                for (i, entity) in self.entities.iter().enumerate() {
                    if matches!(entity.sce(), SceType::Door | SceType::Item) {
                        continue;
                    }

                    ui.selectable_value(&mut self.selected_object, SelectedObject::Entity(i), format!("AOT {}", aot_count));
                    aot_count += 1;
                }
            });

            if let Some(recording) = &mut self.active_recording {
                if let Some(state) = recording.current_state() {
                    ui.collapsing("Characters", |ui| {
                        for (i, character) in state.characters().iter().enumerate() {
                            let Some(character) = character.as_ref() else {
                                continue;
                            };

                            ui.selectable_value(&mut self.selected_object, SelectedObject::Character(i), format!("#{}: {}", i, character.name()));
                        }
                    });
                }
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
                eprintln!("Failed to load room {}: {}", id, e);
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
    
    fn rng_browser(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            let Some(ref recording) = self.active_recording else {
                return;
            };
            
            // show in reverse order so newest items are at the top
            for roll in recording.get_rng_descriptions().into_iter().rev() {
                ui.label(roll);
            }
        });
    }

    fn settings_browser(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            ui.checkbox(&mut self.config.show_sounds, "Show sounds");
            ui.separator();

            for (object_type, object_settings) in &mut self.config.object_settings {
                ui.label(egui::RichText::new(object_type.name()).strong());
                ui.checkbox(&mut object_settings.show, "Show");
                egui::widgets::color_picker::color_picker_color32(ui, &mut object_settings.color, Alpha::OnlyBlend);
                ui.separator();
            }
        });
    }

    fn get_character(&self, index: usize) -> Option<&Character> {
        self.active_recording.as_ref().and_then(Recording::current_state).and_then(|state| {
            state.characters().get(index)
        }).and_then(Option::as_ref)
    }

    fn get_character_settings(&self, index: usize) -> Option<CharacterSettings> {
        let room_id = self.active_recording.as_ref().and_then(Recording::current_state).map(State::room_id)?;
        let character_id = self.get_character(index)?.id;
        Some(self.character_settings.get(&(room_id, character_id, index)).copied().unwrap_or_default())
    }

    fn get_character_settings_mut(&mut self, index: usize) -> Option<&mut CharacterSettings> {
        let room_id = self.active_recording.as_ref().and_then(Recording::current_state).map(State::room_id)?;
        let character_id = self.get_character(index)?.id;
        Some(self.character_settings.entry((room_id, character_id, index)).or_default())
    }

    fn object_details(&mut self, ui: &mut Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            let description = match self.selected_object {
                SelectedObject::Floor(i) => self.floors[i].describe(),
                SelectedObject::Entity(i) => self.entities[i].describe(),
                SelectedObject::Collider(i) => self.colliders[i].describe(),
                SelectedObject::Character(i) => match self.get_character(i) {
                    Some(character) => character.describe(),
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
                            ui.label(egui::RichText::new(group_name.clone()).strong());
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
                            ui.label(egui::RichText::new("Display").strong());
                            ui.checkbox(&mut settings.show_tooltip, "Show tooltip");
                            ui.checkbox(&mut settings.show_ai, "Show AI");
                        });
                    }
                }

                ui.shrink_height_to_current();
            });
        });
    }

    fn change_recording_frame<F>(&mut self, func: F)
    where F: FnOnce(&mut Recording) -> Option<&State>
    {
        self.last_play_tick = Instant::now();
        if let Some(next_state) = self.active_recording.as_mut().and_then(func) {
            let new_room_id = next_state.room_id();
            if self.config.last_rdt != Some(new_room_id) {
                if let Err(e) = self.load_room(new_room_id) {
                    eprintln!("Failed to load room {}: {}", new_room_id, e);
                }
            }
        } else {
            // pause once we reach the end of the recording
            self.is_recording_playing = false;
        }
    }

    fn prev_recording_frame(&mut self) {
        self.change_recording_frame(Recording::prev);
    }

    fn next_recording_frame(&mut self) {
        self.change_recording_frame(Recording::next);
    }

    fn set_recording_frame(&mut self, frame: usize) {
        self.change_recording_frame(|recording| recording.set_index(frame));
    }
    
    fn move_recording_frame(&mut self, delta: isize) {
        let Some(index) = self.active_recording.as_ref().map(Recording::index) else {
            return;
        };
        
        let new_index = (index as isize + delta).max(0) as usize;
        self.set_recording_frame(new_index);
    }

    fn player_positions(&self) -> Option<(Vec2, Vec2, u8)> {
        let recording = self.active_recording.as_ref()?;
        let state = recording.current_state()?;
        let player = state.characters().get(0)?.as_ref()?;
        Some((player.center, player.interaction_point(), player.floor))
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if let (true, Some(room_id)) = (self.need_title_update, self.config.last_rdt) {
            ctx.send_viewport_cmd(ViewportCommand::Title(format!("{} - {}", APP_NAME, room_id)));
            self.need_title_update = false;
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open game folder").clicked() {
                        if let Err(e) = self.prompt_load_game() {
                            eprintln!("Failed to open RDT: {}", e);
                        }
                    }

                    if ui.button("Open recording").clicked() && self.is_game_loaded() {
                        if let Err(e) = self.prompt_load_recording() {
                            eprintln!("Failed to open recording: {}", e);
                        }
                    }
                    
                    ui.separator(); // don't want open button too close to close button
                    
                    if ui.button("Close recording").clicked() && self.active_recording.is_some() {
                        self.close_recording();
                    }
                });
            });
        });

        egui::SidePanel::left("browser").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    for tab in BrowserTab::list() {
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
                }
            });
        });

        egui::TopBottomPanel::bottom("detail").show(ctx, |ui| {
            let width = ui.max_rect().width();
            ui.vertical(|ui| {
                let mut need_toggle = false;
                let mut new_frame_index = None;
                if let Some(recording) = &mut self.active_recording {
                    ui.horizontal(|ui| {
                        let play_pause = if self.is_recording_playing {
                            "⏸"
                        } else {
                            "▶"
                        };

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
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.ui_contains_pointer() {
                self.handle_input(ctx);
            }
            
            let view_center = self.calculate_origin(ctx);
            let (player_pos, player_interaction_pos, player_floor) = match self.player_positions() {
                Some((player_pos, player_interaction_pos, player_floor)) => (Some(player_pos), Some(player_interaction_pos), Some(player_floor)),
                None => (None, None, None),
            };

            if self.config.should_show(ObjectType::Floor) {
                for (i, floor) in self.floors.iter().enumerate() {
                    let mut floor_draw_params = self.config.get_draw_params(ObjectType::Floor, view_center);
                    if self.selected_object == SelectedObject::Floor(i) {
                        // unlike the other object types, we don't draw the floor on top when it's highlighted
                        // because it covers everything up and makes it hard to tell what's actually on the
                        // given floor
                        floor_draw_params.highlight();
                    }

                    let shape = floor.gui_shape(&floor_draw_params);
                    ui.painter().add(shape);
                }
            }

            let mut collider_draw_params = self.config.get_draw_params(ObjectType::Collider, view_center);
            if self.config.should_show(ObjectType::Collider) {
                for (i, collider) in self.colliders.iter().enumerate() {
                    if self.selected_object == SelectedObject::Collider(i) {
                        continue;
                    }

                    let shape = collider.gui_shape(&collider_draw_params);
                    ui.painter().add(shape);
                }
            }

            for (i, entity) in self.entities.iter().enumerate() {
                if self.selected_object == SelectedObject::Entity(i) {
                    continue;
                }

                let object_type: ObjectType = entity.sce().into();
                if !self.config.should_show(object_type) {
                    continue;
                }

                let mut entity_draw_params = self.config.get_draw_params(object_type, view_center);

                let trigger_point = if entity.is_trigger_on_enter() {
                    player_pos
                } else {
                    player_interaction_pos
                };
                if let (Some(trigger_point), Some(trigger_floor)) = (trigger_point, player_floor) {
                    if entity.could_trigger(trigger_point, trigger_floor) {
                        entity_draw_params.outline();
                    }
                }

                let shape = entity.gui_shape(&entity_draw_params);
                ui.painter().add(shape);
            }

            if let Some(recording) = &self.active_recording {
                if let Some(state) = recording.current_state() {
                    let mut ai_zones = Vec::with_capacity(NUM_CHARACTERS);
                    let mut character_icons = Vec::with_capacity(NUM_CHARACTERS);

                    for (i, character) in state.characters().iter().enumerate() {
                        if self.selected_object == SelectedObject::Character(i) {
                            continue;
                        }

                        let (Some(character), Some(settings)) = (character.as_ref(), self.get_character_settings(i)) else {
                            continue;
                        };

                        let object_type: ObjectType = character.type_().into();
                        if !self.config.should_show(object_type) {
                            continue;
                        }

                        let char_draw_params = self.config.get_draw_params(object_type, view_center);
                        character_icons.push(character.gui_shape(&char_draw_params, ui, i, settings.show_tooltip));
                        if settings.show_ai {
                            ai_zones.push(character.gui_ai(&char_draw_params, player_pos));
                        }
                    }

                    // draw all AI zones first, then all characters, so characters are always on top of the zones
                    for ai_zone in ai_zones {
                        ui.painter().add(ai_zone);
                    }

                    // if the current selected object is a character, and that character has AI zones, draw those
                    // zones after all other zones, but still before characters, because we always want those to
                    // be on top
                    if let SelectedObject::Character(i) = self.selected_object {
                        if let (Some(Some(character)), Some(settings)) = (state.characters().iter().nth(i).map(Option::as_ref), self.get_character_settings(i)) {
                            let object_type: ObjectType = character.type_().into();
                            if settings.show_ai && self.config.should_show(object_type) {
                                let char_draw_params = self.config.get_draw_params(object_type, view_center);
                                ui.painter().add(character.gui_ai(&char_draw_params, player_pos));
                            }
                        }
                    }

                    for character_icon in character_icons {
                        ui.painter().add(character_icon);
                    }
                }

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
                    };

                    for sound in recording.get_player_sounds(MAX_SOUND_AGE) {
                        let sound_box = Self::get_sound_text_box(&sound, &sound_draw_params, ui);
                        ui.painter().add(sound_box);
                    }
                }
            }

            // draw highlighted object (if any) on top
            match self.selected_object {
                SelectedObject::None | SelectedObject::Floor(_) => {}
                SelectedObject::Entity(i) => {
                    let mut entity_draw_params = self.config.get_draw_params(self.entities[i].sce().into(), view_center);
                    entity_draw_params.highlight();
                    let shape = self.entities[i].gui_shape(&entity_draw_params);
                    ui.painter().add(shape);
                }
                SelectedObject::Collider(i) => {
                    collider_draw_params.highlight();
                    let shape = self.colliders[i].gui_shape(&collider_draw_params);
                    ui.painter().add(shape);
                }
                SelectedObject::Character(i) => {
                    let (Some(character), Some(settings)) = (self.get_character(i), self.get_character_settings(i)) else {
                        return;
                    };

                    let object_type: ObjectType = character.type_().into();
                    let char_draw_params = self.config.get_draw_params(object_type, view_center);
                    ui.painter().add(character.gui_shape(&char_draw_params, ui, i, settings.show_tooltip));
                }
            }

            // show player inputs in top right
            if let Some(recording) = &self.active_recording {
                if let Some(state) = recording.current_state() {
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
            }
        });

        if self.active_recording.is_some() && self.is_recording_playing {
            let now = Instant::now();
            let duration = now - self.last_play_tick;
            if duration >= FRAME_DURATION {
                self.next_recording_frame();
            }

            // re-draw regularly while we're animating
            ctx.request_repaint();
        }
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Err(e) = self.config.save() {
            eprintln!("Failed to save config: {}", e);
        }
    }
}