use renderer_protocol::{
    entity::{BaseEntityData, PlayerEntityOutput},
    input::PlayerEntityInput,
};

use crate::renderer::camera::Camera;

use super::{Entity, Output};

#[derive(Debug, Clone)]
pub struct PlayerEntity {
    base_data: BaseEntityData,
}

impl Output for PlayerEntityOutput {}

impl From<BaseEntityData> for PlayerEntity {
    fn from(base_data: BaseEntityData) -> Self {
        Self { base_data }
    }
}

impl Entity for PlayerEntity {
    type Output = PlayerEntityOutput;
    type State = BaseEntityData;

    fn base_data(&self) -> &BaseEntityData {
        &self.base_data
    }

    fn process_output(&mut self, output: Self::Output) {
        match output {
            PlayerEntityOutput::NewPosition(new_position) => {
                self.base_data.position = new_position;
            }
        }
    }
}

impl PlayerEntity {
    pub fn send_input(&self, input: &mut Vec<PlayerEntityInput>) {
        input.push(PlayerEntityInput::NewPosition(self.position()));
    }

    pub fn update(&mut self, camera: &Camera) {
        self.base_data.position = camera.view.eye;
    }
}
