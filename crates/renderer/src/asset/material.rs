use std::sync::Arc;

use super::texture::TextureAsset;

#[derive(Debug, Clone, Copy, Default)]
pub enum MaterialAlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

#[derive(Debug, Clone)]
pub struct MaterialAsset {
    pub name: Option<String>,
    pub unlit: bool,
    pub diffuse_color: Option<[f32; 4]>,
    pub diffuse_texture: Option<Arc<TextureAsset>>,
    pub alpha_mode: Option<MaterialAlphaMode>,
}
