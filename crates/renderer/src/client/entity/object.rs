use glam::Vec3;
use renderer_protocol::entity::{
    BaseEntityData, EntityResourceData, ObjectEntityOutput, ObjectEntityState,
};
use uuid::Uuid;

use super::{Entity, Output, State};

#[derive(Debug, Clone)]
pub struct ObjectEntity {
    base: BaseEntityData,
    resource: EntityResourceData,
}

impl Output for ObjectEntityOutput {}

impl State for ObjectEntityState {
    fn id(&self) -> Uuid {
        self.base.id()
    }

    fn position(&self) -> Vec3 {
        self.base.position()
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
    type Output = ObjectEntityOutput;
    type State = ObjectEntityState;

    fn base_data(&self) -> &BaseEntityData {
        &self.base
    }

    fn process_output(&mut self, output: Self::Output) {
        match output {
            ObjectEntityOutput::NewPosition(new_position) => self.base.position = new_position,
        }
    }
}

impl ObjectEntity {
    pub fn resource(&self) -> &EntityResourceData {
        &self.resource
    }
}
