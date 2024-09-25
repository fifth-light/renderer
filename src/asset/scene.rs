use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use super::{
    node::{NodeAsset, NodeAssetId},
    skin::{SkinAsset, SkinAssetId},
};

#[derive(Debug, Clone, Default)]
pub struct SceneAsset {
    pub name: Option<String>,
    pub nodes: Vec<NodeAsset>,
    pub joint_nodes: HashMap<NodeAssetId, BTreeSet<SkinAssetId>>,
    pub skinned_nodes: Vec<NodeAsset>,
    pub skins: HashMap<SkinAssetId, Arc<SkinAsset>>,
}
