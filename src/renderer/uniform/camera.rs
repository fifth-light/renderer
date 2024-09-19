use bytemuck::{cast_slice, Pod, Zeroable};
use glam::Mat4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device, Queue,
};

use crate::renderer::camera::Camera;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, Default)]
struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

pub struct CameraUniformBuffer {
    buffer: Buffer,
    uniform: CameraUniform,
}

impl CameraUniformBuffer {
    pub fn new(device: &Device, camera: &Camera, default_aspect: f32) -> Self {
        let uniform = CameraUniform {
            view_proj: camera.matrix(default_aspect).to_cols_array_2d(),
        };
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self { buffer, uniform }
    }

    pub fn update_view_proj(&mut self, view_proj: Mat4) {
        self.uniform.view_proj = view_proj.to_cols_array_2d();
    }

    pub fn update(&self, queue: &Queue) {
        queue.write_buffer(&self.buffer, 0, cast_slice(&[self.uniform]));
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
