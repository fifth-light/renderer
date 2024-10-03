use std::{path::PathBuf, sync::Arc};

use glam::{Mat4, Quat, Vec3};

use super::{camera::CameraAsset, mesh::MeshAsset, skin::SkinAsset};

#[derive(Debug, Clone)]
pub struct MatrixNodeTransform(pub Mat4);

#[derive(Debug, Clone)]
pub struct DecomposedTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for DecomposedTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NodeTransform {
    Matrix(MatrixNodeTransform),
    Decomposed(DecomposedTransform),
}

impl Default for NodeTransform {
    fn default() -> Self {
        Self::Decomposed(DecomposedTransform::default())
    }
}

impl From<MatrixNodeTransform> for Mat4 {
    fn from(value: MatrixNodeTransform) -> Self {
        value.0
    }
}

impl From<DecomposedTransform> for Mat4 {
    fn from(value: DecomposedTransform) -> Self {
        Mat4::from_translation(value.translation)
            * Mat4::from_quat(value.rotation)
            * Mat4::from_scale(value.scale)
    }
}

impl From<NodeTransform> for Mat4 {
    fn from(value: NodeTransform) -> Self {
        match value {
            NodeTransform::Matrix(matrix) => matrix.0,
            NodeTransform::Decomposed(decomposed) => decomposed.into(),
        }
    }
}

impl From<NodeTransform> for MatrixNodeTransform {
    fn from(value: NodeTransform) -> Self {
        match value {
            NodeTransform::Matrix(matrix) => matrix,
            NodeTransform::Decomposed(decomposed) => MatrixNodeTransform(decomposed.into()),
        }
    }
}

impl From<NodeTransform> for DecomposedTransform {
    fn from(value: NodeTransform) -> Self {
        match value {
            NodeTransform::Matrix(matrix) => {
                let (scale, rotation, translation) = matrix.0.to_scale_rotation_translation();
                DecomposedTransform {
                    translation,
                    rotation,
                    scale,
                }
            }
            NodeTransform::Decomposed(decomposed) => decomposed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeAssetId {
    PathIndex(PathBuf, usize),
    NameIndex(String, usize),
    String(String),
    Path(PathBuf),
}

impl From<(PathBuf, usize)> for NodeAssetId {
    fn from(value: (PathBuf, usize)) -> Self {
        Self::PathIndex(value.0, value.1)
    }
}

impl From<(String, usize)> for NodeAssetId {
    fn from(value: (String, usize)) -> Self {
        Self::NameIndex(value.0, value.1)
    }
}

impl From<String> for NodeAssetId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<PathBuf> for NodeAssetId {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

#[derive(Debug, Clone)]
pub struct NodeAsset {
    pub id: NodeAssetId,
    pub name: Option<String>,
    pub transform: Option<NodeTransform>,
    pub mesh: Option<MeshAsset>,
    pub skin: Option<Arc<SkinAsset>>,
    pub camera: Option<CameraAsset>,
    pub has_animation: bool,
    pub children: Vec<NodeAsset>,
}
