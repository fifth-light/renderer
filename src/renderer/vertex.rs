use std::mem::size_of;

use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, Buffer, BufferAddress, BufferUsages, Device, IndexFormat, RenderPass,
    VertexAttribute, VertexBufferLayout, VertexStepMode,
};

use super::index::IndexBuffer;

pub trait Vertex: Copy + Clone + Pod + Zeroable {
    const ATTRIBS: &[VertexAttribute];

    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex for ColorVertex {
    const ATTRIBS: &[VertexAttribute] = &vertex_attr_array![
        0 => Float32x3,
        1 => Float32x4
    ];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TextureVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex for TextureVertex {
    const ATTRIBS: &[VertexAttribute] = &vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2
    ];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ColorSkinVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub joint_index: [u16; 4],
    pub joint_weight: [f32; 4],
}

impl Vertex for ColorSkinVertex {
    const ATTRIBS: &[VertexAttribute] = &vertex_attr_array![
        0 => Float32x3,
        1 => Float32x4,
        2 => Uint16x4,
        3 => Float32x4
    ];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TextureSkinVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub joint_index: [u16; 4],
    pub joint_weight: [f32; 4],
}

impl Vertex for TextureSkinVertex {
    const ATTRIBS: &[VertexAttribute] = &vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Uint16x4,
        3 => Float32x4
    ];
}

#[derive(Debug)]
pub struct VertexBuffer {
    pub buffer: Buffer,
    pub vertices: usize,
}

impl VertexBuffer {
    pub fn new<T: Vertex>(device: &Device, vertices: &[T], label: Option<&str>) -> Self {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: cast_slice(vertices),
            usage: BufferUsages::VERTEX,
        });
        Self {
            buffer,
            vertices: vertices.len(),
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn vertices(&self) -> usize {
        self.vertices
    }

    pub fn draw(&self, render_pass: &mut RenderPass) {
        render_pass.set_vertex_buffer(0, self.buffer().slice(..));
        render_pass.draw(0..self.vertices() as u32, 0..1);
    }

    pub fn draw_with_indexes(&self, index_buffer: &IndexBuffer, render_pass: &mut RenderPass) {
        render_pass.set_vertex_buffer(0, self.buffer().slice(..));
        render_pass.set_index_buffer(index_buffer.buffer().slice(..), IndexFormat::Uint32);
        render_pass.draw_indexed(0..index_buffer.indices() as u32, 0, 0..1);
    }
}
