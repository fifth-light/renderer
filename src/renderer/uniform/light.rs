use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device, Queue,
};

use crate::renderer::node::light::LightData;

pub const MAX_POINT_LIGHTS: usize = 128;
pub const MAX_PARALLEL_LIGHTS: usize = 16;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct PointLightItem {
    position: [f32; 3],
    padding_0: [u8; 4],
    color: [f32; 3],
    constant: f32,
    linear: f32,
    quadratic: f32,
    padding_1: [u8; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct ParallelLightData {
    direction: [f32; 3],
    padding: [u8; 4],
    color: [f32; 3],
    strength: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightUniform {
    pub point_length: u32,    // 4 byte
    pub parallel_length: u32, // 4 byte
    padding: [u8; 8],         // 8 byte
    point: [PointLightItem; MAX_POINT_LIGHTS],
    parallel: [ParallelLightData; MAX_PARALLEL_LIGHTS],
}

#[derive(Debug)]
pub struct LightUniformBuffer {
    buffer: Buffer,
    uniform: LightUniform,
    pub items: Vec<LightData>,
}

impl LightUniformBuffer {
    pub fn new(device: &Device, items: Vec<LightData>) -> Self {
        let mut point_length: u32 = 0;
        let mut parallel_length: u32 = 0;
        let mut point: [PointLightItem; MAX_POINT_LIGHTS] = [Default::default(); MAX_POINT_LIGHTS];
        let mut parallel: [ParallelLightData; MAX_PARALLEL_LIGHTS] =
            [Default::default(); MAX_PARALLEL_LIGHTS];
        for item in &items {
            match item {
                LightData::Point {
                    position,
                    color,
                    constant,
                    linear,
                    quadratic,
                } => {
                    let point = &mut point[point_length as usize];
                    point.position = position.to_array();
                    point.color = color.to_array();
                    point.constant = *constant;
                    point.linear = *linear;
                    point.quadratic = *quadratic;
                    point_length += 1;
                }
                LightData::Parallel {
                    direction,
                    color,
                    strength,
                } => {
                    let parallel = &mut parallel[parallel_length as usize];
                    parallel.direction = direction.to_array();
                    parallel.color = color.to_array();
                    parallel.strength = *strength;
                    parallel_length += 1;
                }
            }
        }
        let uniform = LightUniform {
            point_length,
            parallel_length,
            padding: [0; 8],
            point,
            parallel,
        };
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Light Uniform Buffer"),
            contents: cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self {
            buffer,
            uniform,
            items,
        }
    }

    pub fn update(&mut self, queue: &Queue) {
        let mut point_length: u32 = 0;
        let mut parallel_length: u32 = 0;
        for item in &self.items {
            match item {
                LightData::Point {
                    position,
                    color,
                    constant,
                    linear,
                    quadratic,
                } => {
                    let point = &mut self.uniform.point[point_length as usize];
                    point.position = position.to_array();
                    point.color = color.to_array();
                    point.constant = *constant;
                    point.linear = *linear;
                    point.quadratic = *quadratic;
                    point_length += 1;
                }
                LightData::Parallel {
                    direction,
                    color,
                    strength,
                } => {
                    let parallel = &mut self.uniform.parallel[parallel_length as usize];
                    parallel.direction = direction.to_array();
                    parallel.color = color.to_array();
                    parallel.strength = *strength;
                    parallel_length += 1;
                }
            }
        }
        self.uniform.point_length = point_length;
        self.uniform.parallel_length = parallel_length;
        queue.write_buffer(&self.buffer, 0, cast_slice(&[self.uniform]));
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
