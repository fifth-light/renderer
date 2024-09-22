use glam::Vec3;
use wgpu::{Device, PrimitiveTopology, Queue};

use crate::renderer::{
    context::{GlobalContext, LocalContext},
    index::IndexBuffer,
    pipeline::{PipelineIdentifier, Pipelines, ShaderAlphaMode, ShaderType},
    vertex::{ColorVertex, VertexBuffer},
    OngoingRenderState, RendererBindGroupLayout, RendererState,
};

use super::{
    new_node_id,
    primitive::{PrimitiveNode, PrimitiveNodeContent},
    RenderNode,
};

#[rustfmt::skip]
const LIGHT_POSITIONS: &[[f32; 3]] = &[
    [-0.05, -0.05, -0.05],
    [0.05,  -0.05, -0.05],
    [0.05,   0.05, -0.05],
    [-0.05,  0.05, -0.05],
    [0.05,  -0.05,  0.05],
    [0.05,   0.05,  0.05],
    [-0.05, -0.05,  0.05],
    [-0.05,  0.05,  0.05],
];

#[rustfmt::skip]
const LIGHT_INDICES: &[u32] = &[
    0, 2, 1,
    2, 0, 3,
    1, 2, 5,
    1, 5, 4,
    4, 0, 1,
    0, 4, 6,
    4, 5, 7,
    4, 7, 6,
    0, 6, 3,
    6, 7, 3,
    5, 2, 3,
    5, 3, 7,
];

#[derive(Debug, Clone)]
pub enum LightData {
    Point {
        position: Vec3,
        color: Vec3,
        constant: f32,
        linear: f32,
        quadratic: f32,
    },
    Parallel {
        direction: Vec3,
        color: Vec3,
        strength: f32,
    },
}

#[derive(Debug, Clone)]
pub enum LightParam {
    Point {
        color: Vec3,
        constant: f32,
        linear: f32,
        quadratic: f32,
    },
    Parallel {
        direction: Vec3,
        color: Vec3,
        strength: f32,
    },
}

#[derive(Debug)]
pub struct LightNode {
    id: usize,
    node: Option<PrimitiveNode>,
    param: LightParam,
}

impl LightNode {
    pub fn new(
        device: &Device,
        bind_group_layouts: &RendererBindGroupLayout,
        pipelines: &mut Pipelines,
        param: LightParam,
        show_box: bool,
    ) -> Self {
        let indices = IndexBuffer::new(device, LIGHT_INDICES, None);
        let color = match param {
            LightParam::Point { color, .. } => color,
            LightParam::Parallel { color, .. } => color,
        };
        let color_array = color.to_array();
        let vertices: Vec<ColorVertex> = LIGHT_POSITIONS
            .iter()
            .map(|position| ColorVertex {
                position: *position,
                color: [color_array[0], color_array[1], color_array[2], 1.0],
                normal: [0.0, 0.0, 0.0],
            })
            .collect();
        let buffer = VertexBuffer::new(device, &vertices, None);
        let pipeline = pipelines.get(
            device,
            bind_group_layouts,
            PipelineIdentifier {
                shader: ShaderType::Light,
                primitive_topology: PrimitiveTopology::TriangleList,
                alpha_mode: ShaderAlphaMode::Opaque,
            },
            false,
        );
        let node = if let LightParam::Point { .. } = param {
            if show_box {
                Some(PrimitiveNode::new(
                    Some(indices),
                    PrimitiveNodeContent::Color { buffer },
                    pipeline,
                    None,
                ))
            } else {
                None
            }
        } else {
            None
        };
        LightNode {
            id: new_node_id(),
            node,
            param,
        }
    }

    pub fn node(&self) -> Option<&PrimitiveNode> {
        self.node.as_ref()
    }

    pub fn param(&self) -> &LightParam {
        &self.param
    }
}

impl RenderNode for LightNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(
        &mut self,
        local_context: &LocalContext,
        global_context: &mut GlobalContext,
        invalid: bool,
    ) {
        let (_, _, position) = local_context.transform().to_scale_rotation_translation();
        let light_data = match self.param {
            LightParam::Point {
                color,
                constant,
                linear,
                quadratic,
            } => LightData::Point {
                position,
                color,
                constant,
                linear,
                quadratic,
            },
            LightParam::Parallel {
                direction,
                color,
                strength,
            } => LightData::Parallel {
                direction,
                color,
                strength,
            },
        };
        global_context.add_light(light_data);
        if let Some(node) = self.node.as_mut() {
            node.update(local_context, global_context, invalid);
        }
    }

    fn prepare(&mut self, device: &Device, queue: &Queue, renderer_state: &mut RendererState) {
        if let Some(node) = self.node.as_mut() {
            node.prepare(device, queue, renderer_state);
        }
    }

    fn draw<'a>(
        &'a self,
        renderer_state: &'a RendererState,
        ongoing_state: &mut OngoingRenderState<'a>,
    ) {
        if let Some(node) = self.node.as_ref() {
            node.draw(renderer_state, ongoing_state);
        }
    }
}
