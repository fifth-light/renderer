use std::path::PathBuf;

use glam::Mat4;

use super::node::NodeAssetId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkinAssetId {
    PathIndex(PathBuf, usize),
    NameIndex(String, usize),
    RandomIndex(u64, usize),
    String(String),
    Path(PathBuf),
}

#[derive(Debug, Clone)]
pub struct SkinAsset {
    pub id: SkinAssetId,
    pub joint_ids: Vec<NodeAssetId>,
    pub inverse_bind_matrices: Vec<Mat4>,
}
