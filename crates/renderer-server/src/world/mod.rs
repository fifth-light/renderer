use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    error::Error,
    fmt::{self, Display, Formatter},
};

use log::warn;
use renderer_protocol::{entity::EntityStates, input::PlayerEntityInput, tick::TickOutput};
use uuid::Uuid;

use crate::entity::{player::PlayerEntity, test::TestEntity, Entity};

#[derive(Debug)]
pub struct EntityAlreadyExists;

impl Display for EntityAlreadyExists {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Entity already exists")
    }
}

impl Error for EntityAlreadyExists {}

#[derive(Debug)]
pub struct EntityItem<E: Entity> {
    entity: E,
    messages: VecDeque<E::Message>,
}

impl<E: Entity> EntityItem<E> {
    fn new(entity: E) -> Self {
        Self {
            entity,
            messages: VecDeque::new(),
        }
    }

    #[must_use]
    fn process_messages(&mut self, output: &mut Vec<(Uuid, E::Output)>) -> bool {
        let mut has_message = false;
        while let Some(message) = self.messages.pop_back() {
            has_message = true;
            let id = self.entity.id();
            self.entity
                .process_message(message, &mut self.messages, |change| {
                    output.push((id, change));
                });
        }
        has_message
    }

    fn clone_state(&self) -> E::State {
        self.entity.clone_state()
    }
}

impl EntityItem<PlayerEntity> {
    fn process_input(&mut self, input: PlayerEntityInput) {
        self.entity.process_input(input, &mut self.messages);
    }
}

#[derive(Debug)]
pub struct EntityItems<E: Entity> {
    items: HashMap<Uuid, EntityItem<E>>,
    pending_removed: HashSet<Uuid>,
}

impl<E: Entity> Default for EntityItems<E> {
    fn default() -> Self {
        Self {
            items: HashMap::new(),
            pending_removed: HashSet::new(),
        }
    }
}

impl<E: Entity> EntityItems<E> {
    fn clone_state(&self) -> Vec<E::State> {
        self.items.values().map(EntityItem::clone_state).collect()
    }

    fn queue_remove(&mut self, id: Uuid) {
        self.pending_removed.insert(id);
    }

    fn clear_removed(&mut self, ids: &mut Vec<Uuid>) {
        for id in self.pending_removed.drain() {
            match self.items.remove(&id) {
                Some(_) => {
                    ids.push(id);
                }
                None => {
                    warn!("Remove non-existing entity id: {:?}", id);
                }
            }
        }
    }

    fn insert_new(&mut self, entity: E) -> Result<E::State, EntityAlreadyExists> {
        match self.items.entry(entity.id()) {
            Entry::Occupied(_) => Err(EntityAlreadyExists),
            Entry::Vacant(entry) => {
                let item = EntityItem::new(entity);
                let state = item.clone_state();
                entry.insert(item);
                Ok(state)
            }
        }
    }

    #[must_use]
    fn process_messages(&mut self, output: &mut Vec<(Uuid, E::Output)>) -> bool {
        let mut has_message = false;
        for item in self.items.values_mut() {
            if item.process_messages(output) {
                has_message = true;
            }
        }
        has_message
    }
}

impl EntityItems<PlayerEntity> {
    fn process_inputs(&mut self, id: Uuid, input: PlayerEntityInput) {
        if let Some(player) = self.items.get_mut(&id) {
            player.process_input(input);
        } else {
            warn!("Input with unknown player: {}", id)
        }
    }
}

#[derive(Debug, Default)]
pub struct Entities {
    test: EntityItems<TestEntity>,
    player: EntityItems<PlayerEntity>,
}

impl Entities {
    pub fn state(&self) -> EntityStates {
        EntityStates {
            test: self.test.clone_state(),
            player: self.player.clone_state(),
        }
    }

    pub fn queue_remove_player(&mut self, id: Uuid) {
        self.player.queue_remove(id);
    }

    pub fn process_player_inputs(&mut self, id: Uuid, input: PlayerEntityInput) {
        self.player.process_inputs(id, input);
    }

    pub fn insert_player(
        &mut self,
        player: PlayerEntity,
        output: &mut TickOutput,
    ) -> Result<(), EntityAlreadyExists> {
        let state = self.player.insert_new(player)?;
        output.new_entity_states.player.push(state);
        Ok(())
    }

    pub fn clear_removed_entities(&mut self, output: &mut TickOutput) {
        self.test
            .clear_removed(&mut output.removed_entity_uuids.test);
        self.player
            .clear_removed(&mut output.removed_entity_uuids.player);
    }

    pub fn process_messages(&mut self, output: &mut TickOutput) {
        loop {
            let mut has_message = false;

            has_message |= self.test.process_messages(&mut output.entity_outputs.test);
            has_message |= self
                .player
                .process_messages(&mut output.entity_outputs.player);

            if !has_message {
                break;
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct World {
    pub entities: Entities,
    pub tick_output: TickOutput,
}

impl World {
    pub fn insert_player(&mut self, player: PlayerEntity) -> Result<(), EntityAlreadyExists> {
        self.entities.insert_player(player, &mut self.tick_output)
    }

    #[must_use]
    pub fn tick(&mut self) -> TickOutput {
        self.entities.clear_removed_entities(&mut self.tick_output);
        self.entities.process_messages(&mut self.tick_output);
        self.tick_output.take()
    }
}
