use std::{collections::VecDeque, fmt::Debug};

use glam::Vec3;
use renderer_protocol::entity::BaseEntityData;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod player;
pub mod test;

pub trait Message: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {}

pub trait Output: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {}

pub trait State: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {
    fn id(&self) -> Uuid;
    fn position(&self) -> Vec3;
}

impl Message for () {}

impl Output for () {}

pub trait Entity: Debug {
    type Message: Message;
    type Output: Output;
    type State: State;

    fn base_data(&self) -> &BaseEntityData;
    fn position(&self) -> Vec3 {
        self.base_data().position()
    }
    fn id(&self) -> Uuid {
        self.base_data().id()
    }

    fn clone_state(&self) -> Self::State;

    fn process_message(
        &mut self,
        message: Self::Message,
        pending_messages: &mut VecDeque<Self::Message>,
        on_change: impl FnMut(Self::Output),
    );
}

impl State for BaseEntityData {
    fn id(&self) -> Uuid {
        self.id
    }

    fn position(&self) -> Vec3 {
        self.position
    }
}
