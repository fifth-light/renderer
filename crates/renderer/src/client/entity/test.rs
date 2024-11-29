use renderer_protocol::entity::{BaseEntityData, TestEntityOutput};

use super::{Entity, Output};

#[derive(Debug, Clone)]
pub struct TestEntity {
    base_data: BaseEntityData,
}

impl Output for TestEntityOutput {}

impl From<BaseEntityData> for TestEntity {
    fn from(base_data: BaseEntityData) -> Self {
        Self { base_data }
    }
}

impl Entity for TestEntity {
    type Output = TestEntityOutput;
    type State = BaseEntityData;

    fn base_data(&self) -> &BaseEntityData {
        &self.base_data
    }

    fn process_output(&mut self, output: Self::Output) {
        match output {
            TestEntityOutput::NewPosition(new_position) => self.base_data.position = new_position,
        }
    }
}
