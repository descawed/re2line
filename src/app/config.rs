use std::path::PathBuf;

use anyhow::Result;
use enum_map::{enum_map, Enum, EnumMap};
use egui::Color32;
use serde::{Deserialize, Serialize};

use crate::aot::SceType;
use crate::collision::DrawParams;

const STROKE_WIDTH: f32 = 1.0;

#[derive(Debug, Enum, PartialEq, Eq, Hash, Clone, Copy, Deserialize, Serialize)]
pub(super) enum ObjectType {
     Floor,
     Collider,
     // AOTs
     Auto,
     Door,
     Item,
     Normal,
     Message,
     Event,
     FlagChg,
     Water,
     Move,
     Save,
     ItemBox,
     Damage,
     Status,
     Hikidashi,
     Windows,
     // end AOTs
     Object,
     Enemy,
     Player,
}

impl From<SceType> for ObjectType {
     fn from(value: SceType) -> Self {
          match value {
               SceType::Auto | SceType::Unknown => Self::Auto,
               SceType::Door => Self::Door,
               SceType::Item => Self::Item,
               SceType::Normal => Self::Normal,
               SceType::Message => Self::Message,
               SceType::Event => Self::Event,
               SceType::FlagChg => Self::Event,
               SceType::Water => Self::Water,
               SceType::Move => Self::Move,
               SceType::Save => Self::Save,
               SceType::ItemBox => Self::ItemBox,
               SceType::Damage => Self::Damage,
               SceType::Status => Self::Status,
               SceType::Hikidashi => Self::Hikidashi,
               SceType::Windows => Self::Windows,
          }
     }
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct ObjectSettings {
     pub do_fill: bool,
     pub color: Color32,
     pub show: bool,
}

impl ObjectSettings {
     fn fill(color: Color32) -> Self {
          Self {
               do_fill: true,
               color,
               show: true,
          }
     }

     fn stroke(color: Color32) -> Self {
          Self {
               do_fill: false,
               color,
               show: true,
          }
     }
     
     pub fn get_draw_params(&self, origin: egui::Pos2, scale: f32) -> DrawParams {
          DrawParams {
               origin,
               scale,
               fill_color: if self.do_fill {
                    self.color
               } else {
                    Color32::TRANSPARENT
               },
               stroke: if self.do_fill {
                    egui::Stroke::NONE
               } else {
                    egui::Stroke {
                         width: STROKE_WIDTH,
                         color: self.color,
                    }
               },
               stroke_kind: egui::StrokeKind::Middle,
          }
     }
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Config {
     pub rdt_folder: Option<PathBuf>,
     pub last_rdt: Option<PathBuf>,
     pub zoom_scale: f32,
     pub object_settings: EnumMap<ObjectType, ObjectSettings>,
}

impl Config {
     pub fn config_path() -> PathBuf {
          let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
          let config_filename = format!("{}.json", super::APP_NAME);
          config_dir.join(config_filename)
     }
     
     pub fn get() -> Result<Self> {
          let config_path = Self::config_path();
          if !config_path.exists() {
               return Ok(Self::default());
          }
          
          let config_str = std::fs::read_to_string(&config_path)?;
          let config: Self = serde_json::from_str(&config_str)?;
          Ok(config)
     }
     
     pub fn save(&self) -> Result<()> {
          let config_path = Self::config_path();
          let config_str = serde_json::to_string_pretty(self)?;
          std::fs::write(&config_path, config_str)?;
          Ok(())
     }
     
     pub fn get_object_settings(&self, object_type: ObjectType) -> &ObjectSettings {
          &self.object_settings[object_type]
     }
     
     pub fn get_draw_params(&self, object_type: ObjectType, origin: egui::Pos2) -> DrawParams {
          self.object_settings[object_type].get_draw_params(origin, self.zoom_scale)
     }
}

impl Default for Config {
     fn default() -> Self {
          Self {
               rdt_folder: None,
               last_rdt: None,
               zoom_scale: 40.0,
               object_settings: enum_map! {
                    ObjectType::Floor => ObjectSettings::fill(Color32::from_rgb(0xa4, 0x4d, 0x68)),
                    ObjectType::Collider => ObjectSettings::stroke(Color32::from_rgb(0x63, 0xb3, 0x4d)),
                    ObjectType::Auto => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xcf, 0x8d, 0xc9, 0x80)),
                    ObjectType::Door => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x59, 0x70, 0xd8, 0x80)),
                    ObjectType::Item => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x4c, 0xb2, 0x92, 0x80)),
                    ObjectType::Normal => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xdb, 0x8b, 0x72, 0x80)),
                    ObjectType::Message => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xb9, 0x78, 0x31, 0x80)),
                    ObjectType::Event => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xd0, 0x77, 0xe1, 0x80)),
                    ObjectType::FlagChg => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xc2, 0x42, 0x9e, 0x80)),
                    ObjectType::Water => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x5e, 0x9b, 0xd5, 0x80)),
                    ObjectType::Move => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x69, 0x7b, 0x37, 0x80)),
                    ObjectType::Save => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xca, 0x46, 0x4d, 0x80)),
                    ObjectType::ItemBox => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xbc, 0xb0, 0x45, 0x80)),
                    ObjectType::Damage => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xd2, 0x52, 0x2c, 0x80)),
                    ObjectType::Status => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xde, 0x4f, 0x85, 0x80)),
                    ObjectType::Hikidashi => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x91, 0x50, 0xc3, 0x80)),
                    ObjectType::Windows => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x79, 0x61, 0xa4, 0x80)),
                    ObjectType::Object => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xd0, 0xd0, 0xd0, 0xc0)),
                    ObjectType::Enemy => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xfd, 0xd0, 0x17, 0xd0)),
                    ObjectType::Player => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x57, 0xe9, 0x64, 0xd0)),
               },
          }
     }
}