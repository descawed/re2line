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
                fill_color: egui::Color32::from_rgb(0xa4, 0x4d, 0x68),
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
                    color: egui::Color32::from_rgb(0x63, 0xb3, 0x4d),
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
                    SceType::Door => egui::Color32::from_rgb(0x59, 0x70, 0xd8),
                    SceType::Item => egui::Color32::from_rgb(0x4c, 0xb2, 0x92),
                    SceType::Damage => egui::Color32::from_rgb(0xd2, 0x52, 0x2c),
                    SceType::Message => egui::Color32::from_rgb(0xb9, 0x78, 0x31),
                    SceType::Water => egui::Color32::from_rgb(0x5e, 0x9b, 0xd5),
                    SceType::Normal => egui::Color32::from_rgb(0xdb, 0x8b, 0x72),
                    SceType::Event => egui::Color32::from_rgb(0xd0, 0x77, 0xe1),
                    SceType::FlagChg => egui::Color32::from_rgb(0xc2, 0x42, 0x9e),
                    SceType::Move => egui::Color32::from_rgb(0x69, 0x7b, 0x37),
                    SceType::Windows => egui::Color32::from_rgb(0x79, 0x61, 0xa4),
                    SceType::ItemBox => egui::Color32::from_rgb(0xbc, 0xb0, 0x45),
                    SceType::Status => egui::Color32::from_rgb(0xde, 0x4f, 0x85),
                    SceType::Save => egui::Color32::from_rgb(0xca, 0x46, 0x4d),
                    SceType::Hikidashi => egui::Color32::from_rgb(0x91, 0x50, 0xc3),
                    SceType::Auto => egui::Color32::from_rgb(0xcf, 0x8d, 0xc9),
                    SceType::Unknown => egui::Color32::BLACK,
                };

                // make entities partially transparent so we can still see the scene and any overlapping
                // entities
                let rgba: egui::Rgba = entity_draw_params.fill_color.into();
                entity_draw_params.fill_color = rgba.multiply(0.5).into();

                let shape = entity.gui_shape(&entity_draw_params);
                ui.painter().add(shape);
            }
        });
    }
}