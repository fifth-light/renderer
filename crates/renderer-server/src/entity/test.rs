use std::collections::VecDeque;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{BaseEntityData, Entity, Message, Output};

#[derive(Debug, Clone)]
pub struct TestEntity {
    base_data: BaseEntityData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TestEntityMessage {
    NewPosition(Vec3),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TestEntityOutput {
    NewPosition(Vec3),
}

impl Message for TestEntityMessage {}

impl Output for TestEntityOutput {}

impl Entity for TestEntity {
    type Message = TestEntityMessage;
    type Output = TestEntityOutput;
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
            TestEntityMessage::NewPosition(new_position) => {
                self.base_data.position = new_position;
                on_change(TestEntityOutput::NewPosition(new_position));
            }
        }
    }
}
