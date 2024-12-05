use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    io::Cursor,
    path::Path,
    sync::Arc,
};

use image::{DynamicImage, GenericImageView, ImageError, ImageReader};

use crate::{
    archive::{Archive, Entry},
    index::AssetIndex,
    texture::{SamplerAsset, TextureAsset, TextureAssetFormat},
};

#[derive(Debug)]
pub enum TextureLoadError<E> {
    Io(E),
    Image(ImageError),
}

impl<E: Display> Display for TextureLoadError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TextureLoadError::Io(err) => Display::fmt(&err, f),
            TextureLoadError::Image(err) => Display::fmt(&err, f),
        }
    }
}

impl<E: Error> Error for TextureLoadError<E> {}

impl<E> From<ImageError> for TextureLoadError<E> {
    fn from(value: ImageError) -> Self {
        TextureLoadError::Image(value)
    }
}

#[derive(Default)]
pub struct TextureLoader {
    texture_cache: HashMap<AssetIndex, Arc<TextureAsset>>,
}

impl TextureLoader {
    pub fn load_image(
        &mut self,
        id: AssetIndex,
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

    fn load_from_buffer_uncached(
        &mut self,
        id: AssetIndex,
        buffer: &[u8],
        sampler: SamplerAsset,
    ) -> Result<Arc<TextureAsset>, ImageError> {
        let reader = ImageReader::new(Cursor::new(buffer));
        let image = reader.with_guessed_format()?.decode()?;
        let texture = self.load_image(id, image, sampler);
        Ok(texture)
    }

    pub fn load_from_archive<T, A: Archive<T>, P: AsRef<Path>>(
        &mut self,
        id: AssetIndex,
        archive: &mut A,
        path: P,
        sampler: SamplerAsset,
    ) -> Result<Option<Arc<TextureAsset>>, TextureLoadError<A::Error>> {
        if let Some(texture) = self.texture_cache.get(&id) {
            return Ok(Some(texture.clone()));
        }

        let Some(mut buffer) = archive.by_path(path).map_err(TextureLoadError::Io)? else {
            return Ok(None);
        };
        let buffer = buffer.unpack().map_err(TextureLoadError::Io)?;
        let texture = self.load_from_buffer_uncached(id.clone(), &buffer, sampler)?;

        self.texture_cache.insert(id, texture.clone());
        Ok(Some(texture))
    }
}
