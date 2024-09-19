use std::iter;

use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupLayout, BindingResource, Device,
    Extent3d, FilterMode, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, Sampler,
    SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::asset::texture::{
    TextureAsset, TextureAssetFormat, TextureMagFilter, TextureMinFilter, TextureMipmapFilter,
    TextureWrappingMode,
};

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
        let (format, data) = match asset.format {
            TextureAssetFormat::Ru8 => (TextureFormat::R8Unorm, &asset.data),
            TextureAssetFormat::Rgu8 => (TextureFormat::Rg8Unorm, &asset.data),
            TextureAssetFormat::Rgbu8 => (
                TextureFormat::Rgba8UnormSrgb,
                &asset
                    .data
                    .chunks_exact(3)
                    .flat_map(|chunk| chunk.iter().cloned().chain(iter::once(u8::MAX)))
                    .collect::<Vec<u8>>(),
            ),
            TextureAssetFormat::Rgbau8 => (TextureFormat::Rgba8UnormSrgb, &asset.data),
            TextureAssetFormat::Ru16 => (TextureFormat::R16Unorm, &asset.data),
            TextureAssetFormat::Rgu16 => (TextureFormat::Rg16Unorm, &asset.data),
            TextureAssetFormat::Rgbu16 => (
                TextureFormat::Rgba16Unorm,
                &asset
                    .data
                    .chunks_exact(6)
                    .flat_map(|chunk| chunk.iter().cloned().chain(u16::MAX.to_le_bytes()))
                    .collect::<Vec<u8>>(),
            ),
            TextureAssetFormat::Rgbau16 => (TextureFormat::Rgba16Unorm, &asset.data),
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
                bytes_per_row: Some(4 * asset.size.0),
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

    pub fn create_bind_group(
        &self,
        device: &Device,
        bind_group_layout: &BindGroupLayout,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some(&self.id),
        })
    }
}
