use std::iter;

use glam::Vec2;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource,
    Device, Extent3d, FilterMode, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, Sampler,
    SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

use renderer_asset::texture::{
    TextureAsset, TextureAssetFormat, TextureMagFilter, TextureMinFilter, TextureMipmapFilter,
    TextureWrappingMode,
};

use super::uniform::texture::TextureUniformBuffer;

#[derive(Debug, Clone)]
pub struct TextureTransform {
    pub offset: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Default for TextureTransform {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

#[derive(Debug)]
pub struct TextureItem {
    id: String,
    texture_view: TextureView,
    sampler: Sampler,
}

impl TextureItem {
    pub fn from_asset(
        device: &Device,
        queue: &Queue,
        asset: &TextureAsset,
        label: Option<&str>,
    ) -> Self {
        let size = Extent3d {
            width: asset.size.0,
            height: asset.size.1,
            depth_or_array_layers: 1,
        };
        let (format, data, bytes_per_pixel) = match asset.format {
            TextureAssetFormat::Ru8 => (TextureFormat::R8Unorm, &asset.data, 1),
            TextureAssetFormat::Rgu8 => (TextureFormat::Rg8Unorm, &asset.data, 2),
            TextureAssetFormat::Rgbu8 => (
                TextureFormat::Rgba8UnormSrgb,
                &asset
                    .data
                    .chunks_exact(3)
                    .flat_map(|chunk| chunk.iter().cloned().chain(iter::once(u8::MAX)))
                    .collect::<Vec<u8>>(),
                4,
            ),
            TextureAssetFormat::Rgbau8 => (TextureFormat::Rgba8UnormSrgb, &asset.data, 4),
            TextureAssetFormat::Ru16 => (TextureFormat::R16Unorm, &asset.data, 2),
            TextureAssetFormat::Rgu16 => (TextureFormat::Rg16Unorm, &asset.data, 4),
            TextureAssetFormat::Rgbu16 => (
                TextureFormat::Rgba16Unorm,
                &asset
                    .data
                    .chunks_exact(6)
                    .flat_map(|chunk| chunk.iter().cloned().chain(u16::MAX.to_le_bytes()))
                    .collect::<Vec<u8>>(),
                6,
            ),
            TextureAssetFormat::Rgbau16 => (TextureFormat::Rgba16Unorm, &asset.data, 8),
        };
        let texture = device.create_texture(&TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            data,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_pixel * asset.size.0),
                rows_per_image: Some(asset.size.1),
            },
            size,
        );
        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        fn wrap_mode(wrap_mode: TextureWrappingMode) -> AddressMode {
            match wrap_mode {
                TextureWrappingMode::ClampToEdge => AddressMode::ClampToEdge,
                TextureWrappingMode::MirroredRepeat => AddressMode::MirrorRepeat,
                TextureWrappingMode::Repeat => AddressMode::Repeat,
            }
        }

        let sampler = device.create_sampler(&SamplerDescriptor {
            label,
            address_mode_u: wrap_mode(asset.sampler.wrap_x),
            address_mode_v: wrap_mode(asset.sampler.wrap_y),
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: match asset.sampler.mag_filter {
                TextureMagFilter::Nearest => FilterMode::Nearest,
                TextureMagFilter::Linear => FilterMode::Linear,
            },
            min_filter: match asset.sampler.min_filter {
                TextureMinFilter::Nearest => FilterMode::Nearest,
                TextureMinFilter::Linear => FilterMode::Linear,
            },
            mipmap_filter: match asset.sampler.mipmap_filter {
                TextureMipmapFilter::Nearest => FilterMode::Nearest,
                TextureMipmapFilter::Linear => FilterMode::Linear,
            },
            ..Default::default()
        });

        Self {
            texture_view,
            sampler,
            id: asset.id.to_string(),
        }
    }

    pub fn empty(device: &Device, queue: &Queue) -> Self {
        let size = Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &[255u8; 4],
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            size,
        );
        let texture_view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture_view,
            sampler,
            id: "_empty".to_string(),
        }
    }

    pub fn create_bind_group(
        &self,
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        transform_uniform: &TextureUniformBuffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: transform_uniform.buffer().as_entire_binding(),
                },
            ],
            label: Some(&self.id),
        })
    }
}
