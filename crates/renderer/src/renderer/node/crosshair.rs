use wgpu::{Device, PrimitiveTopology, Queue};

use crate::renderer::{
    context::{GlobalContext, LocalContext},
    pipeline::{PipelineIdentifier, Pipelines, ShaderAlphaMode, ShaderType},
    vertex::{ColorVertex, VertexBuffer},
    RendererBindGroupLayout,
};

use super::{
    new_node_id,
    primitive::{PrimitiveNode, PrimitiveNodeContent},
    OngoingRenderState, RenderNode, RendererState,
};

#[rustfmt::skip]
const CROSSHAIR_VERTICES: &[ColorVertex] = &[
    ColorVertex { position: [0.0, 0.0, 0.0], color: [1.0, 0.0, 0.0, 1.0], normal: [0.0, 0.0, 0.0], tangent: [0.0, 0.0, 0.0] },
    ColorVertex { position: [1.0, 0.0, 0.0], color: [1.0, 0.0, 0.0, 1.0], normal: [0.0, 0.0, 0.0], tangent: [0.0, 0.0, 0.0] },
    ColorVertex { position: [0.0, 0.0, 0.0], color: [0.0, 1.0, 0.0, 1.0], normal: [0.0, 0.0, 0.0], tangent: [0.0, 0.0, 0.0] },
    ColorVertex { position: [0.0, 1.0, 0.0], color: [0.0, 1.0, 0.0, 1.0], normal: [0.0, 0.0, 0.0], tangent: [0.0, 0.0, 0.0] },
    ColorVertex { position: [0.0, 0.0, 0.0], color: [0.0, 0.0, 1.0, 1.0], normal: [0.0, 0.0, 0.0], tangent: [0.0, 0.0, 0.0] },
    ColorVertex { position: [0.0, 0.0, 1.0], color: [0.0, 0.0, 1.0, 1.0], normal: [0.0, 0.0, 0.0], tangent: [0.0, 0.0, 0.0] },
];

#[derive(Debug)]
pub struct CrosshairNode {
    id: usize,
    node: PrimitiveNode,
}

impl CrosshairNode {
    pub fn new(
        device: &Device,
        bind_group_layouts: &RendererBindGroupLayout,
        pipelines: &mut Pipelines,
    ) -> CrosshairNode {
        let buffer = VertexBuffer::new(device, CROSSHAIR_VERTICES, None);
        let pipeline = pipelines.get(
            device,
            bind_group_layouts,
            PipelineIdentifier {
                shader: ShaderType::Light,
                primitive_topology: PrimitiveTopology::LineList,
                alpha_mode: ShaderAlphaMode::Opaque,
            },
            false,
        );
        let node = PrimitiveNode::new(None, PrimitiveNodeContent::Color { buffer }, pipeline, None);
        CrosshairNode {
            id: new_node_id(),
            node,
        }
    }

    pub fn node(&self) -> &PrimitiveNode {
        &self.node
    }
}

impl RenderNode for CrosshairNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(
        &mut self,
        local_context: &LocalContext,
        global_context: &mut GlobalContext,
        invalid: bool,
    ) {
        self.node.update(local_context, global_context, invalid);
    }

    fn prepare(&mut self, device: &Device, queue: &Queue, renderer_state: &mut RendererState) {
        self.node.prepare(device, queue, renderer_state);
    }

    fn draw<'a>(
        &'a self,
        renderer_state: &'a RendererState,
        ongoing_state: &mut OngoingRenderState<'a>,
    ) {
        self.node.draw(renderer_state, ongoing_state);
    }
}
