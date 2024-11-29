use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEntityData {
    pub id: Uuid,
    pub position: Vec3,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntityStates {
    pub test: Vec<BaseEntityData>,
    pub player: Vec<BaseEntityData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerEntityOutput {
    NewPosition(Vec3),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TestEntityOutput {
    NewPosition(Vec3),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntitiesOutputs {
    pub test: Vec<(Uuid, TestEntityOutput)>,
    pub player: Vec<(Uuid, PlayerEntityOutput)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntitiesIds {
    pub test: Vec<Uuid>,
    pub player: Vec<Uuid>,
}
