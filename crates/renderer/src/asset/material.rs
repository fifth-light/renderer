use std::sync::Arc;

use glam::Vec2;

use super::texture::TextureAsset;

#[derive(Debug, Clone, Copy, Default)]
pub enum MaterialAlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

#[derive(Debug, Clone)]
pub struct TextureAssetTransform {
    pub offset: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub tex_coord: Option<u32>,
}

impl Default for TextureAssetTransform {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            tex_coord: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaterialTexture {
    pub texture: Arc<TextureAsset>,
    pub transform: Option<TextureAssetTransform>,
}

#[derive(Debug, Clone)]
pub struct MaterialAsset {
    pub name: Option<String>,
    pub unlit: bool,
    pub diffuse_color: Option<[f32; 4]>,
    pub diffuse_texture: Option<MaterialTexture>,
    pub alpha_mode: Option<MaterialAlphaMode>,
}
