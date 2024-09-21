use std::{fmt::Debug, sync::Arc};

use log::warn;
use wgpu::BindGroup;

use crate::renderer::{
    index::IndexBuffer,
    pipeline::{RenderPipelineItem, ShaderType},
    vertex::VertexBuffer,
    OngoingRenderState, RenderBindGroups, RendererState,
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
            PrimitiveNodeContent::Color { buffer } => {
                assert!(matches!(
                    self.pipeline.shader_type(),
                    ShaderType::Color | ShaderType::Light
                ));
                ongoing_state.bind_groups(RenderBindGroups::Color);
                buffer
            }
            PrimitiveNodeContent::ColorSkin { buffer } => {
                assert_eq!(self.pipeline.shader_type(), ShaderType::ColorSkin);
                if !ongoing_state.is_joint_bound() {
                    warn!(
                        "Trying to draw skinned primitive node #{} without joints bound.",
                        self.id
                    );
                    return;
                }
                ongoing_state.bind_groups(RenderBindGroups::Color);
                buffer
            }
            PrimitiveNodeContent::Texture { buffer, bind_group } => {
                assert_eq!(self.pipeline.shader_type(), ShaderType::Texture);
                ongoing_state.bind_groups(RenderBindGroups::Texture {
                    texture: bind_group,
                });
                buffer
            }
            PrimitiveNodeContent::TextureSkin { buffer, bind_group } => {
                assert_eq!(self.pipeline.shader_type(), ShaderType::TextureSkin);
                if !ongoing_state.is_joint_bound() {
                    warn!("Trying to draw a skinned primitive without joints bound.");
                    return;
                }
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
