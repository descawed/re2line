use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::BufReader;
use std::str::FromStr;
use anyhow::{Result, bail, anyhow};
use eframe::{Frame, Storage};
use egui::{Context, Ui, ViewportCommand};
use rfd::FileDialog;

use crate::aot::{Entity, SceType};
use crate::collision::Collider;
use crate::math::Fixed12;
use crate::rdt::Rdt;

mod config;
use config::{Config, ObjectType, RoomId};

pub const APP_NAME: &str = "re2line";

const DETAIL_MAX_ROWS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedObject {
    None,
    Entity(usize),
    Collider(usize),
    Floor(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserTab {
    Game,
    Room,
}

impl BrowserTab {
    const fn list() -> [BrowserTab; 2] {
        [BrowserTab::Game, BrowserTab::Room]
    }

    const fn name(&self) -> &'static str {
        match self {
            Self::Game => "Game",
            Self::Room => "Room",
        }
    }
}

pub struct App {
    center: (Fixed12, Fixed12),
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
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            center: (Fixed12(0), Fixed12(0)),
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
        })
    }

    const fn scale(&self) -> f32 {
        self.config.zoom_scale
    }

    fn calculate_origin(&mut self, ctx: &Context, handle_input: bool) -> egui::Pos2 {
        let viewport = ctx.input(|i| {
            if handle_input {
                if i.pointer.primary_down() && !i.pointer.primary_pressed() {
                    self.pan -= i.pointer.delta();
                }

                self.config.zoom_scale += i.smooth_scroll_delta.y * 0.05;
            }

            i.screen_rect()
        });

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

    fn room_browser(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if let (true, Some(room_id)) = (self.need_title_update, self.config.last_rdt) {
            ctx.send_viewport_cmd(ViewportCommand::Title(format!("{} - {}", APP_NAME, room_id)));
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Err(e) = self.prompt_load_game() {
                            eprintln!("Failed to open RDT: {}", e);
                        }
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
                }
            });
        });

        egui::TopBottomPanel::bottom("detail").show(ctx, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                let description = match self.selected_object {
                    SelectedObject::Floor(i) => self.floors[i].describe(),
                    SelectedObject::Entity(i) => self.entities[i].describe(),
                    SelectedObject::Collider(i) => self.colliders[i].describe(),
                    SelectedObject::None => return,
                };
                
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
                    
                    ui.shrink_height_to_current();
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let view_center = self.calculate_origin(ctx, ui.ui_contains_pointer());

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

            let mut collider_draw_params = self.config.get_draw_params(ObjectType::Collider, view_center);
            for (i, collider) in self.colliders.iter().enumerate() {
                if self.selected_object == SelectedObject::Collider(i) {
                    continue;
                }

                let shape = collider.gui_shape(&collider_draw_params);
                ui.painter().add(shape);
            }

            for (i, entity) in self.entities.iter().enumerate() {
                if self.selected_object == SelectedObject::Entity(i) {
                    continue;
                }

                let entity_draw_params = self.config.get_draw_params(entity.sce().into(), view_center);
                let shape = entity.gui_shape(&entity_draw_params);
                ui.painter().add(shape);
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
            }
        });
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Err(e) = self.config.save() {
            eprintln!("Failed to save config: {}", e);
        }
    }
}