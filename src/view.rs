use eframe::Frame;
use egui::Context;

use crate::aot::{Entity, SceType};
use crate::collision::{Collider, DrawParams, RectCollider};
use crate::math::Fixed12;
use crate::rdt::Rdt;

pub struct View {
    center: (Fixed12, Fixed12),
    colliders: Vec<Collider>,
    entities: Vec<Entity>,
    floors: Vec<RectCollider>,
    pan: egui::Vec2,
    scale: f32,
}

impl View {
    pub fn new(rdt: Rdt) -> Self {
        let (x, y) = rdt.get_center();
        Self {
            center: (x, -y),
            colliders: rdt.get_colliders(),
            entities: rdt.get_entities(),
            floors: rdt.get_floors(),
            pan: egui::Vec2::ZERO,
            scale: 20.0,
        }
    }
}

impl eframe::App for View {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        let (viewport, scroll) = ctx.input(|i| {
            if i.pointer.primary_down() && !i.pointer.primary_pressed() {
                self.pan -= i.pointer.delta();
            }

            (i.screen_rect(), i.smooth_scroll_delta)
        });

        self.scale += scroll.y * 0.05;

        let window_center = viewport.center();
        let view_center = egui::Pos2::new(
            self.center.0 * self.scale - window_center.x,
            self.center.1 * self.scale - window_center.y,
        ) + self.pan;

        egui::CentralPanel::default().show(ctx, |ui| {
            let floor_draw_params = DrawParams {
                origin: view_center,
                scale: self.scale,
                fill_color: egui::Color32::DARK_RED,
                stroke: egui::Stroke::NONE,
                stroke_kind: egui::StrokeKind::Outside,
            };

            for floor in &self.floors {
                let shape = floor.gui_shape(&floor_draw_params);
                ui.painter().add(shape);
            }

            let collider_draw_params = DrawParams {
                origin: view_center,
                scale: self.scale,
                fill_color: egui::Color32::TRANSPARENT,
                stroke: egui::Stroke {
                    width: 1.0,
                    color: egui::Color32::GREEN,
                },
                stroke_kind: egui::StrokeKind::Middle,
            };

            for collider in &self.colliders {
                let shape = collider.gui_shape(&collider_draw_params);
                ui.painter().add(shape);
            }

            let mut entity_draw_params = DrawParams {
                origin: view_center,
                scale: self.scale,
                fill_color: egui::Color32::BLUE,
                stroke: egui::Stroke::NONE,
                stroke_kind: egui::StrokeKind::Outside,
            };

            for entity in &self.entities {
                entity_draw_params.fill_color = match entity.sce() {
                    SceType::Door => egui::Color32::BLUE,
                    SceType::Item => egui::Color32::LIGHT_GRAY,
                    SceType::Damage => egui::Color32::ORANGE,
                    SceType::Normal => continue, // don't know what this is but they're huge and cover the whole screen
                    _ => egui::Color32::YELLOW,
                };

                let shape = entity.gui_shape(&entity_draw_params);
                ui.painter().add(shape);
            }
        });
    }
}