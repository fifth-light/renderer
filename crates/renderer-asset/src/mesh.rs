use super::primitive::PrimitiveAsset;

#[derive(Debug, Clone)]
pub struct MeshAsset {
    pub name: Option<String>,
    pub primitives: Vec<PrimitiveAsset>,
    pub weights: Vec<f32>,
}
