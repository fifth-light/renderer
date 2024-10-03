use std::iter;

use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device, Queue,
};

pub const MAX_JOINTS: usize = 512;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SkinMatrix {
    transform: [[f32; 4]; 4],
    normal: [[f32; 4]; 3],
}

#[derive(Debug)]
pub struct SkinUniformBuffer {
    buffer: Buffer,
    pub items: Vec<Mat4>,
}

impl SkinUniformBuffer {
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

    pub fn new(device: &Device, items: Vec<Mat4>) -> Self {
        assert!(items.len() <= MAX_JOINTS);
        let padded_items: Vec<SkinMatrix> = items
            .iter()
            .map(|matrix| {
                let transform = matrix.to_cols_array_2d();
                let normal = Self::calculate_normal_transform(matrix);
                SkinMatrix { transform, normal }
            })
            .chain(iter::repeat(SkinMatrix::default()))
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
        let items: Vec<SkinMatrix> = self
            .items
            .iter()
            .map(|matrix| {
                let transform = matrix.to_cols_array_2d();
                let normal = Self::calculate_normal_transform(matrix);
                SkinMatrix { transform, normal }
            })
            .collect();
        queue.write_buffer(&self.buffer, 0, cast_slice(&items));
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
