use std::{collections::VecDeque, fmt::Debug};

use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod test;
pub use test::TestEntity;

mod player;
pub use player::PlayerEntity;

pub trait Message: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {}

pub trait Input: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {}

pub trait Output: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {}

pub trait State: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {
    fn id(&self) -> Uuid;
    fn position(&self) -> Vec3;
}

impl Message for () {}

impl Input for () {}

impl Output for () {}

pub trait Entity: Debug {
    type Message: Message;
    type Input: Input;
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

    fn process_input(
        &mut self,
        input: Self::Input,
        pending_messages: &mut VecDeque<Self::Message>,
        changes: &mut VecDeque<Self::Output>,
    );

    fn process_message(
        &mut self,
        message: Self::Message,
        pending_messages: &mut VecDeque<Self::Message>,
        changes: &mut VecDeque<Self::Output>,
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEntityData {
    pub id: Uuid,
    pub position: Vec3,
}

impl State for BaseEntityData {
    fn id(&self) -> Uuid {
        self.id
    }

    fn position(&self) -> Vec3 {
        self.position
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityInput {}
