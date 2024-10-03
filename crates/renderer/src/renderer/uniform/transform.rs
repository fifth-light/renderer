use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device, Queue,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct InstanceUniform {
    transform: [[f32; 4]; 4],
    normal: [[f32; 4]; 3],
    padding: [u8; 12],
}

#[derive(Debug)]
pub struct InstanceUniformBuffer {
    buffer: Buffer,
    pub transform: Mat4,
}

impl InstanceUniformBuffer {
    fn pad_vec3(vec: &Vec3) -> [f32; 4] {
        [vec.x, vec.y, vec.z, 0.0]
    }

    fn calculate_normal_transform(matrix: &Mat4) -> [[f32; 4]; 3] {
        let result = Mat3::from_mat4(matrix.inverse().transpose());
        [
            Self::pad_vec3(&result.x_axis),
            Self::pad_vec3(&result.y_axis),
            Self::pad_vec3(&result.z_axis),
        ]
    }

    pub fn new(device: &Device, transform: Mat4) -> Self {
        let normal_transform = Self::calculate_normal_transform(&transform);
        let uniform = InstanceUniform {
            transform: transform.to_cols_array_2d(),
            normal: normal_transform,
            padding: [0; 12],
        };
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance Uniform Buffer"),
            contents: cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self { buffer, transform }
    }

    pub fn update(&self, queue: &Queue) {
        let normal_transform = Self::calculate_normal_transform(&self.transform);
        let uniform = InstanceUniform {
            transform: self.transform.to_cols_array_2d(),
            normal: normal_transform,
            padding: [0; 12],
        };
        queue.write_buffer(&self.buffer, 0, cast_slice(&[uniform]));
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
