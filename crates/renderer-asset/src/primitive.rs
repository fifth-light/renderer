use std::sync::Arc;

use super::material::MaterialAsset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveAssetMode {
    Points,
    LineStrip,
    LineList,
    TriangleStrip,
    TriangleList,
}

pub type Position = Vec<[f32; 3]>;
pub type Normal = Vec<[f32; 3]>;
pub type Tangent = Vec<[f32; 4]>;
pub type TexCoord = Vec<[f32; 2]>;
pub type VertexColor = Vec<[f32; 4]>;
pub type Joints = Vec<[u16; 4]>;
pub type Weights = Vec<[f32; 4]>;

#[derive(Debug, Clone)]
pub struct PrimitiveAssetAttributes {
    pub position: Position,
    pub normal: Normal,
    pub tangent: Tangent,
    pub tex_coord: Vec<TexCoord>,
    pub color: Vec<VertexColor>,
    pub joints: Vec<Joints>,
    pub weights: Vec<Weights>,
}

#[derive(Debug, Clone)]
pub struct PrimitiveAssetMorphTarget {
    pub position: Position,
    pub normal: Normal,
    pub tangent: Tangent,
}

#[derive(Debug, Clone)]
pub struct PrimitiveAsset {
    pub attributes: PrimitiveAssetAttributes,
    pub indices: Option<Vec<u32>>,
    pub material: Option<Arc<MaterialAsset>>,
    pub mode: PrimitiveAssetMode,
    pub targets: Vec<PrimitiveAssetMorphTarget>,
}
