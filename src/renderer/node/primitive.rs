use std::{fmt::Debug, sync::Arc};

use wgpu::BindGroup;

use crate::renderer::{
    index::IndexBuffer, pipeline::RenderPipelineItem, vertex::VertexBuffer, OngoingRenderState,
    RenderBindGroups, RendererState,
};

use super::{new_node_id, RenderNode};

#[derive(Debug)]
pub enum PrimitiveNodeContent {
    Color {
        buffer: VertexBuffer,
    },
    Texture {
        buffer: VertexBuffer,
        bind_group: Arc<BindGroup>,
    },
    ColorSkin {
        buffer: VertexBuffer,
    },
    TextureSkin {
        buffer: VertexBuffer,
        bind_group: Arc<BindGroup>,
    },
}

#[derive(Debug)]
pub struct PrimitiveNode {
    id: usize,
    pub indices: Option<IndexBuffer>,
    pub content: PrimitiveNodeContent,
    pub pipeline: Arc<RenderPipelineItem>,
}

impl PrimitiveNode {
    pub fn new(
        indices: Option<IndexBuffer>,
        content: PrimitiveNodeContent,
        pipeline: Arc<RenderPipelineItem>,
    ) -> Self {
        Self {
            id: new_node_id(),
            indices,
            content,
            pipeline,
        }
    }
}

impl RenderNode for PrimitiveNode {
    fn id(&self) -> usize {
        self.id
    }

    fn draw(&self, _renderer_state: &RendererState, ongoing_state: &mut OngoingRenderState) {
        ongoing_state
            .render_pass
            .set_pipeline(self.pipeline.render_pipeline());
        let vertex = match &self.content {
            PrimitiveNodeContent::ColorSkin { buffer } | PrimitiveNodeContent::Color { buffer } => {
                ongoing_state.bind_groups(RenderBindGroups::Color);
                buffer
            }
            PrimitiveNodeContent::TextureSkin { buffer, bind_group }
            | PrimitiveNodeContent::Texture { buffer, bind_group } => {
                ongoing_state.bind_groups(RenderBindGroups::Texture {
                    texture: bind_group,
                });
                buffer
            }
        };

        if let Some(indices) = &self.indices {
            vertex.draw_with_indexes(indices, &mut ongoing_state.render_pass);
        } else {
            vertex.draw(&mut ongoing_state.render_pass);
        }
    }
}
