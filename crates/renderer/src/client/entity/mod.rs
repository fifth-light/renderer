use std::fmt::Debug;

use glam::Vec3;
use renderer_protocol::entity::BaseEntityData;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::renderer::OngoingRenderState;

pub mod object;
pub mod player;

pub trait Output: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {}

pub trait State: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Send + Sync {
    fn id(&self) -> Uuid;
    fn position(&self) -> Vec3;
}

impl Output for () {}

pub trait Entity: Debug + From<Self::State> {
    type Output: Output;
    type State: State;

    fn base_data(&self) -> &BaseEntityData;
    fn position(&self) -> Vec3 {
        self.base_data().position()
    }
    fn id(&self) -> Uuid {
        self.base_data().id()
    }

    fn render(&self, _render_state: &mut OngoingRenderState) {}

    fn process_output(&mut self, output: Self::Output);
}

impl State for BaseEntityData {
    fn id(&self) -> Uuid {
        self.id
    }

    fn position(&self) -> Vec3 {
        self.position
    }
}
