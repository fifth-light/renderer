use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::{BaseEntityData, Entity, PlayerEntity, TestEntity};

#[derive(Debug)]
pub struct EntityItem<E: Entity> {
    entity: E,
    input: VecDeque<E::Input>,
    messages: VecDeque<E::Message>,
    changes: VecDeque<E::Output>,
}

impl<E: Entity> EntityItem<E> {
    pub fn process_input(&mut self) {
        while let Some(input) = self.input.pop_front() {
            self.entity
                .process_input(input, &mut self.messages, &mut self.changes);
        }
    }
}

#[derive(Debug, Default)]
pub struct Entities {
    test: HashMap<Uuid, EntityItem<TestEntity>>,
    player: HashMap<Uuid, EntityItem<PlayerEntity>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityStates {
    test: Vec<BaseEntityData>,
    player: Vec<BaseEntityData>,
}

impl Entities {
    pub fn state(&self) -> EntityStates {
        fn clone_base_data<E: Entity>(item: &EntityItem<E>) -> E::State {
            item.entity.clone_state()
        }
        EntityStates {
            test: self.test.values().map(clone_base_data).collect(),
            player: self.player.values().map(clone_base_data).collect(),
        }
    }

    pub fn process_inputs(&mut self) {
        self.test.values_mut().for_each(EntityItem::process_input);
    }
}

#[derive(Debug)]
pub struct World {
    entities: Entities,
}

impl World {
    pub fn empty() -> Self {
        Self {
            entities: Entities::default(),
        }
    }

    pub fn entity_states(&self) -> EntityStates {
        self.entities.state()
    }
}
