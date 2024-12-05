use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use crate::index::AssetIndex;

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

#[derive(Debug, Clone)]
pub struct NodeAsset {
    pub id: AssetIndex,
    pub name: Option<String>,
    pub camera: Option<CameraAsset>,
    pub children: Vec<NodeAsset>,
    pub skin: Option<Arc<SkinAsset>>,
    pub transform: Option<NodeTransform>,
    pub mesh: Option<MeshAsset>,
    pub weights: Vec<f32>,
}
