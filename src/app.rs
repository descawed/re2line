use anyhow::Result;
use eframe::{Frame, Storage};
use egui::Context;

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
    pub fn new(rdt: Rdt) -> Result<Self> {
        let (x, y) = rdt.get_center();
        Ok(Self {
            center: (x, -y),
            colliders: rdt.get_colliders(),
            entities: rdt.get_entities(),
            floors: rdt.get_floors(),
            pan: egui::Vec2::ZERO,
            config: Config::get()?,
        })
    }
    
    const fn scale(&self) -> f32 {
        self.config.zoom_scale
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        let (viewport, scroll) = ctx.input(|i| {
            if i.pointer.primary_down() && !i.pointer.primary_pressed() {
                self.pan -= i.pointer.delta();
            }

            (i.screen_rect(), i.smooth_scroll_delta)
        });

        self.config.zoom_scale += scroll.y * 0.05;

        let window_center = viewport.center();
        let view_center = egui::Pos2::new(
            self.center.0 * self.scale() - window_center.x,
            self.center.1 * self.scale() - window_center.y,
        ) + self.pan;

        egui::CentralPanel::default().show(ctx, |ui| {
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