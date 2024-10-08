use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device, Queue,
};

use crate::renderer::node::light::LightData;

pub const MAX_POINT_LIGHTS: usize = 128;
pub const MAX_DIRECTIONAL_LIGHTS: usize = 64;
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
struct DirectionalLightItem {
    position: [f32; 3],
    constant: f32,
    direction: [f32; 3],
    linear: f32,
    color: [f32; 3],
    quadratic: f32,
    range_inner: f32,
    range_outer: f32,
    padding: [u8; 8],
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
pub struct GlobalLightParam {
    pub start_strength: f32,
    pub stop_strength: f32,
    pub max_strength: f32,
    pub border_start_strength: f32,
    pub border_stop_strength: f32,
    pub border_max_strength: f32,
    pub ambient_strength: f32,
}

impl Default for GlobalLightParam {
    fn default() -> Self {
        Self {
            start_strength: 0.30,
            stop_strength: 1.00,
            max_strength: 0.80,
            border_start_strength: 0.40,
            border_stop_strength: 0.80,
            border_max_strength: 0.20,
            ambient_strength: 0.60,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightUniform {
    point_length: u32,             // 4  4
    directional_length: u32,       // 4  8
    parallel_length: u32,          // 4  12
    light_param: GlobalLightParam, // 28 40
    padding: [u8; 8],              // 8  48
    point: [PointLightItem; MAX_POINT_LIGHTS],
    directional: [DirectionalLightItem; MAX_DIRECTIONAL_LIGHTS],
    parallel: [ParallelLightData; MAX_PARALLEL_LIGHTS],
}

#[derive(Debug)]
pub struct LightUniformBuffer {
    buffer: Buffer,
    uniform: LightUniform,
    pub items: Vec<LightData>,
}

impl LightUniformBuffer {
    pub fn new(device: &Device, items: Vec<LightData>, light_param: GlobalLightParam) -> Self {
        let mut point_length: u32 = 0;
        let mut directional_length: u32 = 0;
        let mut parallel_length: u32 = 0;
        let mut point: [PointLightItem; MAX_POINT_LIGHTS] = [Default::default(); MAX_POINT_LIGHTS];
        let mut directional: [DirectionalLightItem; MAX_DIRECTIONAL_LIGHTS] =
            [Default::default(); MAX_DIRECTIONAL_LIGHTS];
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
                LightData::Directional {
                    position,
                    color,
                    direction,
                    constant,
                    linear,
                    quadratic,
                    range_inner,
                    range_outer,
                } => {
                    let directional = &mut directional[directional_length as usize];
                    directional.position = position.to_array();
                    directional.color = color.to_array();
                    directional.direction = direction.to_array();
                    directional.constant = *constant;
                    directional.linear = *linear;
                    directional.quadratic = *quadratic;
                    directional.range_inner = *range_inner;
                    directional.range_outer = *range_outer;
                    directional_length += 1;
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
            directional_length,
            parallel_length,
            light_param,
            padding: [0; 8],
            point,
            directional,
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

    pub fn set_param(&mut self, param: GlobalLightParam) {
        self.uniform.light_param = param;
    }

    pub fn param(&self) -> &GlobalLightParam {
        &self.uniform.light_param
    }

    pub fn update(&mut self, queue: &Queue) {
        let mut point_length: u32 = 0;
        let mut directional_length: u32 = 0;
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
                LightData::Directional {
                    position,
                    color,
                    direction,
                    constant,
                    linear,
                    quadratic,
                    range_inner,
                    range_outer,
                } => {
                    let directional = &mut self.uniform.directional[directional_length as usize];
                    directional.position = position.to_array();
                    directional.color = color.to_array();
                    directional.direction = direction.to_array();
                    directional.constant = *constant;
                    directional.linear = *linear;
                    directional.quadratic = *quadratic;
                    directional.range_inner = *range_inner;
                    directional.range_outer = *range_outer;
                    directional_length += 1;
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
        self.uniform.directional_length = directional_length;
        self.uniform.parallel_length = parallel_length;
        queue.write_buffer(&self.buffer, 0, cast_slice(&[self.uniform]));
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}
