use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use enum_map::{enum_map, EnumMap};
use egui::Color32;
use serde::{Deserialize, Serialize};

use crate::character::PLAYER_COLLISION_MASK;
use super::game::{DrawParams, GameObject, ObjectType};

const STROKE_WIDTH: f32 = 1.0;
const STAGE_CHARACTERS: &str = "123456789ABCDEFG";

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, Deserialize, Serialize)]
pub struct RoomId {
     pub stage: u8,
     pub room: u8,
     pub player: u8,
}

impl RoomId {
     pub const fn new(stage: u8, room: u8, player: u8) -> Self {
          Self { stage, room, player }
     }
     
     pub const fn zero() -> Self {
          Self { stage: 0, room: 0, player: 0 }
     }
}

impl std::fmt::Display for RoomId {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          let stage = self.stage as usize;
          write!(f, "{}{:02X}{}", &STAGE_CHARACTERS[stage..stage + 1], self.room, self.player)
     }
}

impl FromStr for RoomId {
     type Err = anyhow::Error;

     fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
          if s.len() != 4 {
               return Err(anyhow!("Invalid room ID: {}", s));
          }

          let stage_char = s.get(0..1).unwrap().to_uppercase();
          let stage = STAGE_CHARACTERS.find(&stage_char).ok_or_else(|| anyhow!("Invalid stage ID in room ID {}", s))? as u8;
          let room = u8::from_str_radix(s.get(1..3).unwrap(), 16)?;
          let player = s.get(3..4).ok_or_else(|| anyhow!("Invalid player ID in room ID {}", s))?.parse::<u8>()?;
          
          Ok(Self { stage, room, player })
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
               draw_at_origin: false,
          }
     }
}

const fn default_true() -> bool {
     true
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Config {
     pub rdt_folder: Option<PathBuf>,
     pub last_rdt: Option<RoomId>,
     pub zoom_scale: f32,
     #[serde(default = "default_true")]
     pub show_sounds: bool,
     #[serde(default)]
     pub focus_current_selected_object: bool,
     #[serde(default)]
     pub alternate_collision_colors: bool,
     #[serde(default = "default_true")]
     pub default_show_character_tooltips: bool,
     #[serde(default = "default_true")]
     pub show_character_rng: bool,
     #[serde(default = "default_true")]
     pub show_known_non_character_rng: bool,
     #[serde(default = "default_true")]
     pub show_unknown_rng: bool,
     #[serde(default)]
     pub show_all_objects: bool,
     pub object_settings: EnumMap<ObjectType, ObjectSettings>,
}

impl Config {
     pub fn config_path() -> PathBuf {
          let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("../../.."));
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
     
     pub fn get_draw_params(&self, object_type: ObjectType, origin: egui::Pos2) -> DrawParams {
          self.object_settings[object_type].get_draw_params(origin, self.zoom_scale)
     }
     
     pub fn get_obj_draw_params<O: GameObject>(&self, object: &O, origin: egui::Pos2) -> DrawParams {
          let object_type = object.object_type();
          let mut params = self.get_draw_params(object_type, origin);
          if self.alternate_collision_colors && matches!(object_type, ObjectType::Collider) {
               let collision_mask = object.collision_mask();
               if collision_mask == 0 {
                    params.set_color(Color32::from_gray(0x20));
               } else if collision_mask & PLAYER_COLLISION_MASK == 0 {
                    params.set_color(self.object_settings[ObjectType::Enemy].color);
               }
          }
          
          params
     }
     
     pub fn should_show(&self, object_type: ObjectType) -> bool {
          self.object_settings[object_type].show
     }
}

impl Default for Config {
     fn default() -> Self {
          Self {
               rdt_folder: None,
               last_rdt: None,
               zoom_scale: 40.0,
               show_sounds: true,
               focus_current_selected_object: false,
               alternate_collision_colors: false,
               default_show_character_tooltips: true,
               show_character_rng: true,
               show_known_non_character_rng: true,
               show_unknown_rng: true,
               show_all_objects: false,
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
                    ObjectType::Ally => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x57, 0xe9, 0xd3, 0xd0)),
                    ObjectType::Neutral => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x57, 0xcc, 0x57, 0xd0)),
                    ObjectType::AiAggroZone => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xfc, 0x98, 0x03, 0xb0)),
                    ObjectType::AiAttackZone => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0xfc, 0x1c, 0x03, 0xb0)),
                    ObjectType::AiTacticZone => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x5e, 0x03, 0xfc, 0xb0)),
                    ObjectType::AiHitZone => ObjectSettings::fill(Color32::from_rgba_unmultiplied(0x4a, 0x04, 0x2e, 0xb0)),
                    ObjectType::WeaponRange => ObjectSettings::stroke(Color32::from_rgba_unmultiplied(41, 0, 188, 128)),
                    ObjectType::CharacterPath => ObjectSettings::stroke(Color32::from_rgba_unmultiplied(0x57, 0xe9, 0x64, 0x80)),
               },
          }
     }
}