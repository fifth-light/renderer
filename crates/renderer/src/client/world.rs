use std::collections::hash_map::Entry;

use egui::ahash::HashMap;
use log::warn;
use renderer_protocol::{
    entity::{EntitiesIds, EntitiesOutputs, EntityStates},
    tick::TickOutput,
};
use uuid::Uuid;

use crate::renderer::OngoingRenderState;

use super::entity::{object::ObjectEntity, player::PlayerEntity, Entity, State};

#[derive(Debug)]
pub struct Entities {
    pub player: HashMap<Uuid, PlayerEntity>,
    pub object: HashMap<Uuid, ObjectEntity>,
}

impl From<EntityStates> for Entities {
    fn from(states: EntityStates) -> Self {
        macro_rules! create_entities {
            ($type:ty, $entity:ident) => {
                states
                    .$entity
                    .into_iter()
                    .map(|state| (state.id(), <$type>::from(state)))
                    .collect()
            };
        }
        Self {
            player: create_entities!(PlayerEntity, player),
            object: create_entities!(ObjectEntity, object),
        }
    }
}

impl Entities {
    pub fn render(&self, render_state: &mut OngoingRenderState) {
        macro_rules! render {
            ($map:expr, $render_state:expr) => {
                $map.values()
                    .for_each(|entity| entity.render($render_state));
            };
        }
        render!(self.object, render_state);
        render!(self.player, render_state);
    }

    fn remove(&mut self, ids: EntitiesIds) {
        macro_rules! remove {
            ($entity:ident, $name:literal) => {
                ids.$entity.into_iter().for_each(|id| {
                    if self.$entity.remove(&id).is_none() {
                        warn!("Remove unknown {}: {:?}", $name, id)
                    }
                });
            };
        }
        remove!(object, "object");
        remove!(player, "player");
    }

    fn add_entity(&mut self, state: EntityStates) {
        macro_rules! add {
            ($entry:ident, $type:ty) => {
                state
                    .$entry
                    .into_iter()
                    .map(<$type>::from)
                    .for_each(|entity| match self.$entry.entry(entity.id()) {
                        Entry::Occupied(_) => {
                            warn!("Insert existing {}: {:?}", stringify!($entry), entity.id())
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(entity);
                        }
                    });
            };
        }
        add!(object, ObjectEntity);
        add!(player, PlayerEntity);
    }

    fn process_output(&mut self, output: EntitiesOutputs) {
        macro_rules! process {
            ($entry:ident, $name:literal) => {
                output
                    .$entry
                    .into_iter()
                    .for_each(|(id, output)| match self.$entry.get_mut(&id) {
                        Some(entity) => entity.process_output(output),
                        None => warn!("Handle output for unknown {}: {:?}", $name, id),
                    });
            };
        }
        process!(object, "object");
        process!(player, "player");
    }
}

#[derive(Debug)]
pub struct World {
    pub entities: Entities,
}

impl World {
    pub fn new(entity_states: EntityStates) -> Self {
        Self {
            entities: Entities::from(entity_states),
        }
    }

    pub fn render(&self, render_state: &mut OngoingRenderState) {
        self.entities.render(render_state);
    }

    pub fn update(&mut self, tick_output: TickOutput) {
        self.entities.remove(tick_output.removed_entity_uuids);
        self.entities.add_entity(tick_output.new_entity_states);
        self.entities.process_output(tick_output.entity_outputs)
    }
}
