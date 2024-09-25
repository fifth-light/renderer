use super::material::MaterialAsset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveAssetMode {
    Points,
    LineStrip,
    LineList,
    TriangleStrip,
    TriangleList,
}

pub type TexCoords = Vec<[f32; 2]>;
pub type VertexColor = Vec<[f32; 4]>;
pub type SkinJoints = Vec<[u16; 4]>;
pub type SkinWeights = Vec<[f32; 4]>;

#[derive(Debug, Clone)]
pub struct PrimitiveSkin {
    pub joints: SkinJoints,
    pub weights: SkinWeights,
}

pub enum PrimitiveVertex {}

#[derive(Debug, Clone)]
pub struct PrimitiveAsset {
    pub name: Option<String>,

    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tex_coords: Vec<TexCoords>,
    pub vertex_color: Vec<VertexColor>,
    pub indices: Option<Vec<u32>>,

    pub skin: Vec<PrimitiveSkin>,
    pub material: Option<MaterialAsset>,
    pub mode: PrimitiveAssetMode,
}
