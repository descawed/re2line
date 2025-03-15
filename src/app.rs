use std::fs::File;
use std::path::Path;
use std::io::BufReader;

use anyhow::Result;
use eframe::{Frame, Storage};
use egui::Context;
use rfd::FileDialog;

use crate::aot::{Entity, SceType};
use crate::collision::{Collider, RectCollider};
use crate::math::Fixed12;
use crate::rdt::Rdt;

mod config;
use config::{Config, ObjectType};

pub const APP_NAME: &str = "re2line";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedObject {
    None,
    Entity(usize),
    Collider(usize),
    Floor(usize),
}

pub struct App {
    center: (Fixed12, Fixed12),
    colliders: Vec<Collider>,
    entities: Vec<Entity>,
    floors: Vec<RectCollider>,
    pan: egui::Vec2,
    selected_object: SelectedObject,
    config: Config,
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

    fn set_rdt(&mut self, rdt: Rdt) {
        let (x, y) = rdt.get_center();
        self.center = (x, -y);
        self.colliders = rdt.get_colliders();
        self.entities = rdt.get_entities();
        self.floors = rdt.get_floors();
        self.pan = egui::Vec2::ZERO;
        self.selected_object = SelectedObject::None;
    }

    pub fn try_resume_rdt(&mut self) -> Result<()> {
        if let Some(ref path) = self.config.rdt_folder {
            self.load_rdt(path.clone())?;
        }

        Ok(())
    }

    pub fn load_rdt(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let rdt = Rdt::read(reader)?;

        self.set_rdt(rdt);

        Ok(())
    }

    fn prompt_load_rdt(&mut self) -> Result<()> {
        let Some(file) = FileDialog::new()
            .add_filter("RDTs", &["rdt", "RDT"])
            .set_directory("/media/jacob/E2A6DD85A6DD5A9D/games/BIOHAZARD 2 PC/pl0/Rdt") // TODO: remove after testing
            .pick_file() else {
            return Ok(());
        };

        self.load_rdt(file.as_path())
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Err(e) = self.prompt_load_rdt() {
                            eprintln!("Failed to open RDT: {}", e);
                        }
                    }
                });
            });
        });
        
        egui::SidePanel::left("browser").show(ctx, |ui| {
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