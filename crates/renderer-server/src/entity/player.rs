use std::collections::VecDeque;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{BaseEntityData, Entity, Input, Output};

#[derive(Debug, Clone)]
pub struct PlayerEntity {
    base_data: BaseEntityData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerEntityInput {
    NewPosition(Vec3),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerEntityOutput {
    NewPosition(Vec3),
}

impl Input for PlayerEntityInput {}

impl Output for PlayerEntityOutput {}

impl Entity for PlayerEntity {
    type Message = ();
    type Input = PlayerEntityInput;
    type Output = PlayerEntityOutput;
    type State = BaseEntityData;

    fn base_data(&self) -> &BaseEntityData {
        &self.base_data
    }

    fn clone_state(&self) -> Self::State {
        self.base_data.clone()
    }

    fn process_input(
        &mut self,
        input: Self::Input,
        _pending_messages: &mut VecDeque<Self::Message>,
        changes: &mut VecDeque<Self::Output>,
    ) {
        match input {
            PlayerEntityInput::NewPosition(new_position) => {
                self.base_data.position = new_position;
                changes.push_back(PlayerEntityOutput::NewPosition(new_position));
            }
        }
    }

    fn process_message(
        &mut self,
        _message: Self::Message,
        _pending_messages: &mut VecDeque<Self::Message>,
        _changes: &mut VecDeque<Self::Output>,
    ) {
    }
}
