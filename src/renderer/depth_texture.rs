use std::sync::mpsc;

use image::GrayImage;
use wgpu::{
    BufferAddress, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Device, Extent3d,
    ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, Maintain, MapMode, Origin3d, Queue,
    Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, COPY_BYTES_PER_ROW_ALIGNMENT,
};

pub const DEPTH_TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth32Float;
pub const DEPTH_PIXEL_SIZE: usize = size_of::<f32>();

pub struct DepthTexture {
    texture: Texture,
    texture_view: TextureView,
    size: (u32, u32),
    texture_size: Extent3d,
}

impl DepthTexture {
    pub fn new(device: &Device, size: (u32, u32)) -> Self {
        let texture_size = Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        };
        let desc = TextureDescriptor {
            label: Some("Depth texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEPTH_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            texture_view,
            size,
            texture_size,
        }
    }

    pub fn dump_depth(&self, device: &Device, queue: &Queue) -> GrayImage {
        let actual_bytes_per_row = DEPTH_PIXEL_SIZE as u32 * self.size.0;
        let stride_per_row = if actual_bytes_per_row % COPY_BYTES_PER_ROW_ALIGNMENT == 0 {
            0
        } else {
            COPY_BYTES_PER_ROW_ALIGNMENT - actual_bytes_per_row % COPY_BYTES_PER_ROW_ALIGNMENT
        };
        let bytes_per_row = actual_bytes_per_row + stride_per_row;
        let buffer_size = bytes_per_row * self.size.1;
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Depth Dump Buffer"),
            size: buffer_size as BufferAddress,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::DepthOnly,
            },
            ImageCopyBuffer {
                buffer: &buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.size.1),
                },
            },
            self.texture_size,
        );
        queue.submit(Some(encoder.finish()));

        let buffer_slice = buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        buffer_slice.map_async(MapMode::Read, move |result| {
            tx.send(result.unwrap()).unwrap();
        });
        device.poll(Maintain::Wait);
        rx.recv().unwrap();
        let buffer_view = buffer_slice.get_mapped_range();
        let data: Vec<u8> = buffer_view
            .chunks_exact(bytes_per_row as usize)
            .flat_map(|row| row[0..actual_bytes_per_row as usize].into_iter())
            .cloned()
            .collect();
        let data: Vec<u8> = data
            .chunks_exact(4)
            .map(|chunk| (f32::from_ne_bytes(chunk.try_into().unwrap()) * u8::MAX as f32) as u8)
            .collect();
        drop(buffer_view);
        let image = GrayImage::from_raw(self.size.0, self.size.1, data).unwrap();

        buffer.unmap();
        image
    }

    pub fn texture_view(&self) -> &TextureView {
        &self.texture_view
    }
}
