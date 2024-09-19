use bytemuck::cast_slice;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device,
};

#[derive(Debug)]
pub struct IndexBuffer {
    buffer: Buffer,
    indices: usize,
}

impl IndexBuffer {
    pub fn new(device: &Device, indices: &[u32], label: Option<&str>) -> Self {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: cast_slice(indices),
            usage: BufferUsages::INDEX,
        });
        Self {
            buffer,
            indices: indices.len(),
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn indices(&self) -> usize {
        self.indices
    }
}
