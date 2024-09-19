use glam::Mat4;

use super::node::{NodeAsset, NodeAssetId};

#[derive(Debug, Clone)]
pub struct SkinAsset {
    pub joint_ids: Vec<NodeAssetId>,
    pub root_joint: Box<NodeAsset>,
    pub inverse_bind_matrices: Vec<Mat4>,
}
