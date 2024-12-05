use glam::Mat4;

use crate::index::AssetIndex;

#[derive(Debug, Clone)]
pub struct SkinAsset {
    pub id: AssetIndex,
    pub inverse_bind_matrices: Vec<Mat4>,
    pub joint_ids: Vec<AssetIndex>,
    pub skeleton: Option<AssetIndex>,
}
