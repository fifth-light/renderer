use std::mem;

use serde::{Deserialize, Serialize};

use crate::entity::{EntitiesIds, EntitiesOutputs, EntityStates};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TickOutput {
    pub new_entity_states: EntityStates,
    pub entity_outputs: EntitiesOutputs,
    pub removed_entity_uuids: EntitiesIds,
}

impl TickOutput {
    pub fn take(&mut self) -> Self {
        Self {
            new_entity_states: mem::take(&mut self.new_entity_states),
            entity_outputs: mem::take(&mut self.entity_outputs),
            removed_entity_uuids: mem::take(&mut self.removed_entity_uuids),
        }
    }
}
