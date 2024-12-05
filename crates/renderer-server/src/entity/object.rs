use std::collections::VecDeque;

use glam::Vec3;
use renderer_protocol::entity::{EntityResourceData, ObjectEntityOutput, ObjectEntityState};
use serde::{Deserialize, Serialize};

use super::{BaseEntityData, Entity, Message, Output, State};

#[derive(Debug, Clone)]
pub struct ObjectEntity {
    base: BaseEntityData,
    resource: EntityResourceData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ObjectEntityMessage {
    NewPosition(Vec3),
}

impl Message for ObjectEntityMessage {}

impl Output for ObjectEntityOutput {}

impl State for ObjectEntityState {
    fn id(&self) -> uuid::Uuid {
        self.base.id
    }

    fn position(&self) -> Vec3 {
        self.base.position
    }
}

impl From<ObjectEntityState> for ObjectEntity {
    fn from(state: ObjectEntityState) -> Self {
        Self {
            base: state.base,
            resource: state.resource,
        }
    }
}

impl Entity for ObjectEntity {
    type Message = ObjectEntityMessage;
    type Output = ObjectEntityOutput;
    type State = ObjectEntityState;

    fn base_data(&self) -> &BaseEntityData {
        &self.base
    }

    fn clone_state(&self) -> Self::State {
        Self::State {
            base: self.base.clone(),
            resource: self.resource.clone(),
        }
    }

    fn process_message(
        &mut self,
        message: Self::Message,
        _pending_messages: &mut VecDeque<Self::Message>,
        mut on_change: impl FnMut(Self::Output),
    ) {
        match message {
            ObjectEntityMessage::NewPosition(new_position) => {
                self.base.position = new_position;
                on_change(ObjectEntityOutput::NewPosition(new_position));
            }
        }
    }
}
