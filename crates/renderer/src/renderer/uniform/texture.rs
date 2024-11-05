use bytemuck::cast_slice;
use glam::{Mat3, Mat4};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device,
};

use crate::renderer::texture::TextureTransform;

pub struct TextureUniformBuffer {
    buffer: Buffer,
}

impl TextureUniformBuffer {
    pub fn new(device: &Device, transform: TextureTransform) -> Self {
        let matrix = Mat3::from_scale_angle_translation(
            transform.scale,
            transform.rotation,
            transform.offset,
        );
        let matrix = Mat4::from_mat3(matrix);
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: cast_slice(&[matrix.to_cols_array()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self { buffer }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
