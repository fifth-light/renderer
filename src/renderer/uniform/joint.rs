use std::iter;

use bytemuck::cast_slice;
use glam::Mat4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device, Queue,
};

pub const MAX_JOINTS: usize = 1024;

#[derive(Debug)]
pub struct JointsUniformBuffer {
    buffer: Buffer,
    pub items: Vec<Mat4>,
}

impl JointsUniformBuffer {
    pub fn new(device: &Device, items: Vec<Mat4>) -> Self {
        assert!(items.len() <= MAX_JOINTS);
        let padded_items: Vec<[f32; 16]> = items
            .iter()
            .map(|matrix| matrix.to_cols_array())
            .chain(iter::repeat(Mat4::IDENTITY.to_cols_array()))
            .take(MAX_JOINTS)
            .collect();
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance Uniform Buffer"),
            contents: cast_slice(&padded_items),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self { buffer, items }
    }

    pub fn update(&self, queue: &Queue) {
        assert!(self.items.len() <= MAX_JOINTS);
        let items: Vec<[f32; 16]> = self
            .items
            .iter()
            .map(|matrix| matrix.to_cols_array())
            .collect();
        queue.write_buffer(&self.buffer, 0, cast_slice(&items));
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
