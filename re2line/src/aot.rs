use residat::common::{Fixed16, Vec2};
use residat::re2::{Item, SceType, SAT_TRIGGER_CENTER};

use crate::app::{DrawParams, Floor, GameObject, ObjectType, RoomId};
use crate::collision::Collider;
use crate::record::State;

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

    pub fn could_trigger(&self, point: Vec2, floor: Floor) -> bool {
        self.sce.is_trigger() && self.floor.matches(floor) && self.collider.contains_point(point)
    }

    pub fn form(&self) -> &EntityForm {
        &self.form
    }

    pub fn floor(&self) -> Floor {
        self.floor
    }

    pub fn sce(&self) -> SceType {
        self.sce
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
            EntityForm::Item { i_item, n_item, flag, .. } => {
                groups.push((String::from("Item"), vec![
                    format!("Type: {}", Item::name_from_id(i_item)),
                    format!("Count: {}", n_item),
                    format!("Flag: {}", flag),
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
                player.center
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