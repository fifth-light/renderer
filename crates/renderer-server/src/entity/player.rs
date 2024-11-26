use std::collections::VecDeque;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{BaseEntityData, Entity, Message, Output};

#[derive(Debug, Clone)]
pub struct PlayerEntity {
    base_data: BaseEntityData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerEntityInput {
    NewPosition(Vec3),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerEntityMessage {
    NewPosition(Vec3),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerEntityOutput {
    NewPosition(Vec3),
}

impl Message for PlayerEntityMessage {}

impl Output for PlayerEntityOutput {}

impl Entity for PlayerEntity {
    type Message = PlayerEntityMessage;
    type Output = PlayerEntityOutput;
    type State = BaseEntityData;

    fn base_data(&self) -> &BaseEntityData {
        &self.base_data
    }

    fn clone_state(&self) -> Self::State {
        self.base_data.clone()
    }

    fn process_message(
        &mut self,
        message: Self::Message,
        _pending_messages: &mut VecDeque<Self::Message>,
        mut on_change: impl FnMut(Self::Output),
    ) {
        match message {
            PlayerEntityMessage::NewPosition(new_position) => {
                self.base_data.position = new_position;
                on_change(PlayerEntityOutput::NewPosition(new_position));
            }
        }
    }
}

impl PlayerEntity {
    pub fn new(id: Uuid, position: Vec3) -> Self {
        Self {
            base_data: BaseEntityData { id, position },
        }
    }

    pub fn process_input(
        &self,
        input: PlayerEntityInput,
        pending_messages: &mut VecDeque<PlayerEntityMessage>,
    ) {
        match input {
            PlayerEntityInput::NewPosition(new_position) => {
                pending_messages.push_back(PlayerEntityMessage::NewPosition(new_position));
            }
        }
    }
}
