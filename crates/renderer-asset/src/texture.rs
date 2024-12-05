use std::sync::Arc;

use crate::index::AssetIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureAssetFormat {
    Ru8,
    Rgu8,
    Rgbu8,
    Rgbau8,
    Ru16,
    Rgu16,
    Rgbu16,
    Rgbau16,
}

#[derive(Debug, Clone)]
pub struct TextureAsset {
    pub id: AssetIndex,
    pub size: (u32, u32),
    pub format: TextureAssetFormat,
    pub data: Vec<u8>,
    pub sampler: SamplerAsset,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TextureMagFilter {
    Nearest,
    #[default]
    Linear,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TextureMinFilter {
    #[default]
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TextureMipmapFilter {
    Nearest,
    #[default]
    Linear,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TextureWrappingMode {
    #[default]
    ClampToEdge,
    MirroredRepeat,
    Repeat,
}

#[derive(Debug, Clone, Default)]
pub struct SamplerAsset {
    pub mag_filter: TextureMagFilter,
    pub min_filter: TextureMinFilter,
    pub mipmap_filter: TextureMipmapFilter,
    pub wrap_x: TextureWrappingMode,
    pub wrap_y: TextureWrappingMode,
}

#[derive(Debug, Clone, Default)]
pub struct TextureAssetTransform {
    pub offset: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
    pub tex_coord: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct TextureInfo {
    pub texture: Arc<TextureAsset>,
    pub tex_coord: usize,
    pub transform: Option<TextureAssetTransform>,
}

impl TextureInfo {
    pub(crate) fn from_texture(texture: Arc<TextureAsset>) -> Self {
        Self {
            texture,
            tex_coord: 0,
            transform: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NormalTextureInfo {
    pub texture: Arc<TextureAsset>,
    pub tex_coord: usize,
    pub scale: f32,
}

impl NormalTextureInfo {
    pub(crate) fn from_texture(texture: Arc<TextureAsset>) -> Self {
        Self {
            texture,
            tex_coord: 0,
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OcclusionTextureInfo {
    pub texture: Arc<TextureAsset>,
    pub tex_coord: usize,
    pub strength: f32,
}

#[derive(Debug, Clone)]
pub struct ShadingShiftTextureInfo {
    pub texture: Arc<TextureAsset>,
    pub tex_coord: usize,
    pub scale: f32,
}
