use std::{
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureAssetId {
    PathIndex(PathBuf, usize),
    NameIndex(String, usize),
    String(String),
    Path(PathBuf),
}

impl Display for TextureAssetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureAssetId::PathIndex(path, id) => write!(f, "{} #{}", path.to_string_lossy(), id),
            TextureAssetId::NameIndex(name, id) => write!(f, "{} #{}", name, id),
            TextureAssetId::String(str) => str.fmt(f),
            TextureAssetId::Path(path) => path.to_string_lossy().fmt(f),
        }
    }
}

impl TextureAssetId {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        Self::Path(path.as_ref().to_path_buf())
    }
}

impl From<String> for TextureAssetId {
    fn from(value: String) -> Self {
        TextureAssetId::String(value)
    }
}

impl From<&Path> for TextureAssetId {
    fn from(value: &Path) -> Self {
        TextureAssetId::Path(value.to_path_buf())
    }
}

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
    pub id: TextureAssetId,
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
