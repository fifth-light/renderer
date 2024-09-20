use glam::{Mat4, Quat, Vec3};
use log::warn;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, Device, Queue};

use crate::{
    asset::node::{DecomposedTransform, NodeTransform},
    renderer::{
        context::{GlobalContext, LocalContext},
        uniform::transform::InstanceUniformBuffer,
        OngoingRenderState, RendererState,
    },
};

use super::{new_node_id, RenderNode, RenderNodeItem};

#[derive(Debug)]
struct TransformBuffer {
    buffer: InstanceUniformBuffer,
    bind_group: BindGroup,
}

#[derive(Debug)]
pub struct TransformNode {
    id: usize,
    // Set updated to false when this node changed
    updated: bool,
    transform: DecomposedTransform,
    // Set invalid to true when the result context changed
    invalid: bool,
    context: Option<LocalContext>,
    buffer: Option<TransformBuffer>,
    pub node: RenderNodeItem,
}

impl TransformNode {
    pub fn new(node: RenderNodeItem) -> Self {
        Self::from_decomposed_transform(DecomposedTransform::default(), node)
    }

    pub fn from_decomposed_transform(transform: DecomposedTransform, node: RenderNodeItem) -> Self {
        Self {
            id: new_node_id(),
            updated: true,
            transform,
            invalid: false,
            context: None,
            buffer: None,
            node,
        }
    }

    pub fn from_transform(transform: NodeTransform, node: RenderNodeItem) -> Self {
        Self::from_decomposed_transform(transform.into(), node)
    }

    pub fn from_scale(scale: Vec3, node: RenderNodeItem) -> Self {
        Self::from_decomposed_transform(
            DecomposedTransform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale,
            },
            node,
        )
    }

    pub fn matrix(&self) -> Mat4 {
        self.transform.clone().into()
    }

    pub fn transform(&self) -> &DecomposedTransform {
        &self.transform
    }

    pub fn set_transform(&mut self, new_transform: DecomposedTransform) {
        self.updated = true;
        self.transform = new_transform;
    }

    pub fn context(&self) -> Option<&LocalContext> {
        self.context.as_ref()
    }
}

impl RenderNode for TransformNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(
        &mut self,
        local_context: &LocalContext,
        global_context: &mut GlobalContext,
        invalid: bool,
    ) {
        if self.updated || invalid || self.context.is_none() {
            // update the context and mark the node to be updated when prepare
            self.invalid = true;
            self.updated = false;

            let context = local_context.add_transform(&self.matrix());
            self.node.update(&context, global_context, true);
            self.context = Some(context);
        } else if let Some(local_context) = &self.context {
            self.node.update(local_context, global_context, invalid);
        } else {
            // No update, no invalid, bad state
            warn!(
                "Bad transform state for node #{}, force trigger invalidate.",
                self.id
            );
            self.invalid = true;
            let context = local_context.add_transform(&self.matrix());
            self.node.update(&context, global_context, true);
            self.context = Some(context);
        }
    }

    fn prepare(&mut self, device: &Device, queue: &Queue, renderer_state: &mut RendererState) {
        let context = match &self.context {
            Some(context) => context,
            None => {
                warn!(
                    "Preparing transform node ${} without context, no-op.",
                    self.id
                );
                warn!("Did you call update() before preparing transform node?");
                return;
            }
        };
        if let Some(buffer) = &mut self.buffer {
            if self.invalid {
                buffer.buffer.transform = *context.transform();
                buffer.buffer.update(queue);
            }
        } else {
            let buffer = InstanceUniformBuffer::new(device, self.matrix());
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Transform Bind Group"),
                layout: &renderer_state.instance_uniform_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: buffer.buffer().as_entire_binding(),
                }],
            });
            self.buffer = Some(TransformBuffer { buffer, bind_group });
            self.invalid = false;
        }
        self.node.prepare(device, queue, renderer_state);
    }

    fn draw<'a>(
        &'a self,
        renderer_state: &'a RendererState,
        ongoing_state: &mut OngoingRenderState<'a>,
    ) {
        if let Some(buffer) = &self.buffer {
            let orig_bind_group = ongoing_state.set_instance(&buffer.bind_group);
            self.node.draw(renderer_state, ongoing_state);
            ongoing_state.set_instance(orig_bind_group);
        } else {
            warn!("Trying to draw transform node #{} without buffer", self.id);
            warn!("Did you call prepare() before drawing transform node?");
        }
    }
}
