use crate::collision::{Collider, DrawParams};
use crate::math::Fixed12;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SceType {
    Auto = 0,
    Door = 1,
    Item = 2,
    Normal = 3,
    Message = 4,
    Event = 5,
    FlagChg = 6,
    Water = 7,
    Move = 8,
    Save = 9,
    ItemBox = 10,
    Damage = 11,
    Status = 12,
    Hikidashi = 13,
    Windows = 14,
    Unknown = 0xFF,
}

impl From<u8> for SceType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Auto,
            1 => Self::Door,
            2 => Self::Item,
            3 => Self::Normal,
            4 => Self::Message,
            5 => Self::Event,
            6 => Self::FlagChg,
            7 => Self::Water,
            8 => Self::Move,
            9 => Self::Save,
            10 => Self::ItemBox,
            11 => Self::Damage,
            12 => Self::Status,
            13 => Self::Hikidashi,
            14 => Self::Windows,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub enum EntityForm {
    Door {
        next_pos_x: Fixed12,
        next_pos_y: Fixed12,
        next_pos_z: Fixed12,
        next_cdir_y: Fixed12,
        next_stage: u8,
        next_room: u8,
        next_n_floor: u8,
    },
    Item {
        i_item: u16,
        n_item: u16,
        flag: u16,
        md1: u8,
        action: u8,
    },
    Other,
}

#[derive(Debug)]
pub struct Entity {
    form: EntityForm,
    collider: Collider,
    floor: u8,
    id: u8,
    sce: SceType,
}

impl Entity {
    pub fn new(form: EntityForm, collider: Collider, floor: u8, id: u8, sce: u8) -> Self {
        Self {
            form,
            collider,
            floor,
            id,
            sce: SceType::from(sce),
        }
    }

    pub fn gui_shape(&self, draw_params: &DrawParams) -> egui::Shape {
        self.collider.gui_shape(draw_params)
    }

    pub fn form(&self) -> &EntityForm {
        &self.form
    }

    pub fn floor(&self) -> u8 {
        self.floor
    }

    pub fn sce(&self) -> SceType {
        self.sce
    }
}