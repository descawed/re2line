use residat::common::{Fixed16, Vec2};
use residat::re2::{
    Item, SceType,
    SAT_TRIGGER_CENTER, SAT_TRIGGER_ON_ACTION,
    SAT_TRIGGER_BY_PLAYER, SAT_TRIGGER_BY_ALLY, SAT_TRIGGER_BY_NPC, SAT_TRIGGER_BY_OBJECT,
};

use crate::app::{DrawParams, Floor, GameObject, ObjectType, RoomId};
use crate::collision::Collider;
use crate::record::State;

pub const NUM_AOTS: usize = 32;

#[derive(Debug)]
pub enum EntityForm {
    Door {
        next_pos_x: Fixed16,
        next_pos_y: Fixed16,
        next_pos_z: Fixed16,
        next_cdir_y: Fixed16,
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
    floor: Floor,
    id: u8,
    sce: SceType,
    sat: u8,
}

impl Entity {
    pub fn new(form: EntityForm, collider: Collider, floor: u8, id: u8, sce: u8, sat: u8) -> Self {
        Self {
            form,
            collider,
            floor: Floor::Id(floor),
            id,
            sce: SceType::from(sce),
            sat,
        }
    }

    pub const fn is_trigger_on_enter(&self) -> bool {
        self.sat & SAT_TRIGGER_CENTER != 0
    }
    
    pub const fn is_trigger_on_action(&self) -> bool {
        self.sat & SAT_TRIGGER_ON_ACTION != 0
    }
    
    pub const fn can_object_type_trigger(&self, object_type: ObjectType) -> bool {
        match object_type {
            ObjectType::Player => self.sat & SAT_TRIGGER_BY_PLAYER != 0,
            ObjectType::Ally => self.sat & SAT_TRIGGER_BY_ALLY != 0,
            ObjectType::Enemy | ObjectType::Neutral => self.sat & SAT_TRIGGER_BY_NPC != 0,
            ObjectType::Object => self.sat & SAT_TRIGGER_BY_OBJECT != 0,
            _ => false,
        }
    }

    pub fn could_trigger(&self, point: Vec2, floor: Floor) -> bool {
        self.sce.is_trigger() && self.floor.matches(floor) && self.collider.contains_point(point)
    }
    
    pub fn is_triggered(&self, object_type: ObjectType, center_point: Vec2, interaction_point: Vec2, floor: Floor, is_action_pressed: bool) -> bool {
        if !self.can_object_type_trigger(object_type) {
            return false;
        }

        if self.is_trigger_on_action() && !is_action_pressed {
            return false;
        }

        let point = if self.is_trigger_on_enter() {
            center_point
        } else {
            interaction_point
        };
        
        if !self.could_trigger(point, floor) {
            return false;
        }
        
        true
    }

    pub const fn form(&self) -> &EntityForm {
        &self.form
    }

    pub const fn floor(&self) -> Floor {
        self.floor
    }

    pub const fn sce(&self) -> SceType {
        self.sce
    }
    
    pub const fn id(&self) -> u8 {
        self.id
    }
}

impl GameObject for Entity {
    fn object_type(&self) -> ObjectType {
        self.sce().into()
    }

    fn contains_point(&self, point: Vec2) -> bool {
        self.collider.contains_point(point)
    }

    fn name(&self) -> String {
        self.sce().name().to_string()
    }

    fn description(&self) -> String {
        let description = format!(
            "Floor: {} | ID: {} | Type: {}",
            self.floor, self.id, self.sce.name(),
        );

        match self.form {
            EntityForm::Door { next_stage, next_room, next_n_floor, .. } => {
                // FIXME: don't know the player ID here
                let room_id = RoomId::new(next_stage, next_room, 0);
                format!("{}\nTarget room: {} | Target floor: {}", description, room_id, next_n_floor)
            }
            EntityForm::Item { i_item, n_item, flag, .. } => {
                format!("{}\nItem ID: {} | Item count: {} | Flag: {}", description, i_item, n_item, flag)
            }
            EntityForm::Other => description,
        }
    }

    fn details(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = self.collider.details();

        groups.push((String::from("Object"), vec![
            format!("Floor: {}", self.floor),
            format!("ID: {}", self.id),
            format!("Type: {}", self.sce.name()),
        ]));

        match self.form {
            EntityForm::Door { next_pos_x, next_pos_y, next_pos_z, next_cdir_y, next_stage, next_room, next_n_floor } => {
                groups.push((String::from("Door"), vec![
                    format!("Target X: {}", next_pos_x),
                    format!("Target Y: {}", next_pos_y),
                    format!("Target Z: {}", next_pos_z),
                    format!("Target Angle: {:.1}°", next_cdir_y.to_degrees()),
                    format!("Target Stage: {}", next_stage),
                    format!("Target Room: {}", next_room),
                    format!("Target Floor: {}", next_n_floor),
                ]));
            }
            EntityForm::Item { i_item, n_item, flag, md1, action } => {
                groups.push((String::from("Item"), vec![
                    format!("Type: {}", Item::name_from_id(i_item)),
                    format!("Count: {}", n_item),
                    format!("Flag: {}", flag),
                    format!("MD1: {}", md1),
                    format!("Action: {}", action),
                ]));
            }
            EntityForm::Other => {}
        }

        groups
    }

    fn floor(&self) -> Floor {
        self.floor
    }

    fn gui_shape(&self, draw_params: &DrawParams, state: &State) -> egui::Shape {
        let mut draw_params = draw_params.clone();
        if let Some(ref player) = state.characters()[0] {
            let trigger_point = if self.is_trigger_on_enter() {
                player.center()
            } else {
                player.interaction_point()
            };
            
            if self.could_trigger(trigger_point, player.floor()) {
                draw_params.outline();
            }
        }
        
        self.collider.gui_shape(&draw_params, state)
    }
}