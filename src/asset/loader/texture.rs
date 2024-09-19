use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, BufRead, Seek},
    path::Path,
    sync::Arc,
};

use image::{DynamicImage, GenericImageView, ImageError, ImageReader};

use crate::asset::texture::{SamplerAsset, TextureAsset, TextureAssetFormat, TextureAssetId};

#[derive(Debug)]
pub enum TextureLoadError {
    Io(io::Error),
    Image(ImageError),
}

impl Display for TextureLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TextureLoadError::Io(err) => Display::fmt(&err, f),
            TextureLoadError::Image(err) => Display::fmt(&err, f),
        }
    }
}

impl Error for TextureLoadError {}

impl From<io::Error> for TextureLoadError {
    fn from(value: io::Error) -> Self {
        TextureLoadError::Io(value)
    }
}

impl From<ImageError> for TextureLoadError {
    fn from(value: ImageError) -> Self {
        TextureLoadError::Image(value)
    }
}

#[derive(Default)]
pub struct TextureLoader {
    texture_cache: HashMap<TextureAssetId, Arc<TextureAsset>>,
}

impl TextureLoader {
    pub fn load_image(
        &mut self,
        id: TextureAssetId,
        image: DynamicImage,
        sampler: SamplerAsset,
    ) -> Arc<TextureAsset> {
        let (dimensions, buffer, format) = match image {
            DynamicImage::ImageRgb8(image) => (
                image.dimensions(),
                image.into_vec(),
                TextureAssetFormat::Rgbu8,
            ),
            DynamicImage::ImageRgba8(image) => (
                image.dimensions(),
                image.into_vec(),
                TextureAssetFormat::Rgbau8,
            ),
            DynamicImage::ImageRgb16(image) => (
                image.dimensions(),
                image
                    .into_vec()
                    .into_iter()
                    .flat_map(|item| item.to_le_bytes())
                    .collect(),
                TextureAssetFormat::Rgbu16,
            ),
            DynamicImage::ImageRgba16(image) => (
                image.dimensions(),
                image
                    .into_vec()
                    .into_iter()
                    .flat_map(|item| item.to_le_bytes())
                    .collect(),
                TextureAssetFormat::Rgbau16,
            ),
            DynamicImage::ImageRgb32F(image) => {
                let converted: DynamicImage = image.into();
                (
                    converted.dimensions(),
                    converted
                        .into_rgb16()
                        .into_vec()
                        .into_iter()
                        .flat_map(|item| item.to_le_bytes())
                        .collect(),
                    TextureAssetFormat::Rgbu16,
                )
            }
            DynamicImage::ImageRgba32F(image) => {
                let converted: DynamicImage = image.into();
                (
                    converted.dimensions(),
                    converted
                        .into_rgba16()
                        .into_vec()
                        .into_iter()
                        .flat_map(|item| item.to_le_bytes())
                        .collect(),
                    TextureAssetFormat::Rgbau16,
                )
            }
            _ => (
                image.dimensions(),
                image.into_rgba8().into_vec(),
                TextureAssetFormat::Rgbau8,
            ),
        };

        Arc::new(TextureAsset {
            id,
            size: dimensions,
            format,
            data: buffer,
            sampler,
        })
    }

    pub fn load_by_id(&self, id: &TextureAssetId) -> Option<Arc<TextureAsset>> {
        self.texture_cache.get(id).cloned()
    }

    pub fn load_from_buffer<R: BufRead + Seek>(
        &mut self,
        id: TextureAssetId,
        reader: R,
        sampler: SamplerAsset,
    ) -> Result<Arc<TextureAsset>, ImageError> {
        if let Some(texture) = self.texture_cache.get(&id) {
            return Ok(texture.clone());
        }

        let reader = ImageReader::new(reader);
        let image = reader.with_guessed_format()?.decode()?;
        let texture = self.load_image(id.clone(), image, sampler);

        self.texture_cache.insert(id, texture.clone());
        Ok(texture)
    }

    pub fn load_from_path(
        &mut self,
        id: TextureAssetId,
        path: &Path,
        sampler: SamplerAsset,
    ) -> Result<Arc<TextureAsset>, TextureLoadError> {
        if let Some(texture) = self.texture_cache.get(&id) {
            return Ok(texture.clone());
        }
        let reader = ImageReader::open(path)?;
        let image = reader.with_guessed_format()?.decode()?;
        let texture = self.load_image(id.clone(), image, sampler);

        self.texture_cache.insert(id, texture.clone());
        Ok(texture)
    }
}
