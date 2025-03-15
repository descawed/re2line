use std::fs::File;
use std::path::Path;
use std::io::BufReader;

use anyhow::Result;
use eframe::{Frame, Storage};
use egui::Context;
use rfd::FileDialog;

use crate::aot::Entity;
use crate::collision::{Collider, RectCollider};
use crate::math::Fixed12;
use crate::rdt::Rdt;

mod config;
use config::{Config, ObjectType};

pub const APP_NAME: &str = "re2line";

pub struct App {
    center: (Fixed12, Fixed12),
    colliders: Vec<Collider>,
    entities: Vec<Entity>,
    floors: Vec<RectCollider>,
    pan: egui::Vec2,
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
            config: Config::get()?,
        })
    }

    const fn scale(&self) -> f32 {
        self.config.zoom_scale
    }

    fn handle_input(&mut self, ctx: &Context) -> egui::Pos2 {
        let (viewport, scroll) = ctx.input(|i| {
            if i.pointer.primary_down() && !i.pointer.primary_pressed() {
                self.pan -= i.pointer.delta();
            }

            (i.screen_rect(), i.smooth_scroll_delta)
        });

        self.config.zoom_scale += scroll.y * 0.05;

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
        let view_center = self.handle_input(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Err(e) = self.prompt_load_rdt() {
                            eprintln!("Failed to open RDT: {}", e);
                        }
                    }
                });
            });

            let floor_draw_params = self.config.get_draw_params(ObjectType::Floor, view_center);

            for floor in &self.floors {
                let shape = floor.gui_shape(&floor_draw_params);
                ui.painter().add(shape);
            }

            let collider_draw_params = self.config.get_draw_params(ObjectType::Collider, view_center);

            for collider in &self.colliders {
                let shape = collider.gui_shape(&collider_draw_params);
                ui.painter().add(shape);
            }

            for entity in &self.entities {
                let entity_draw_params = self.config.get_draw_params(entity.sce().into(), view_center);
                let shape = entity.gui_shape(&entity_draw_params);
                ui.painter().add(shape);
            }
        });
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Err(e) = self.config.save() {
            eprintln!("Failed to save config: {}", e);
        }
    }
}