use glam::Vec3;
use renderer_asset::index::BundleIndex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEntityData {
    pub id: Uuid,
    pub position: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityResourceData {
    Box,
    Crosshair,
    External {
        bundle_index: BundleIndex,
        link: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectEntityState {
    pub base: BaseEntityData,
    pub resource: EntityResourceData,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntityStates {
    pub object: Vec<ObjectEntityState>,
    pub player: Vec<BaseEntityData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerEntityOutput {
    NewPosition(Vec3),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ObjectEntityOutput {
    NewPosition(Vec3),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntitiesOutputs {
    pub object: Vec<(Uuid, ObjectEntityOutput)>,
    pub player: Vec<(Uuid, PlayerEntityOutput)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntitiesIds {
    pub object: Vec<Uuid>,
    pub player: Vec<Uuid>,
}
