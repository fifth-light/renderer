use bytemuck::{cast_slice, Pod, Zeroable};
use glam::Mat4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device, Queue,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct InstanceUniform {
    transform: [[f32; 4]; 4],
}

#[derive(Debug)]
pub struct InstanceUniformBuffer {
    buffer: Buffer,
    pub transform: Mat4,
}

impl InstanceUniformBuffer {
    pub fn new(device: &Device, transform: Mat4) -> Self {
        let uniform = InstanceUniform {
            transform: transform.to_cols_array_2d(),
        };
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance Uniform Buffer"),
            contents: cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self { buffer, transform }
    }

    pub fn update(&self, queue: &Queue) {
        let uniform = InstanceUniform {
            transform: self.transform.to_cols_array_2d(),
        };
        queue.write_buffer(&self.buffer, 0, cast_slice(&[uniform]));
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
