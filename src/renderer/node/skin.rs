use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use glam::Mat4;
use log::warn;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, Device, Queue};

use crate::renderer::{
    context::{GlobalContext, LocalContext},
    uniform::skin::SkinUniformBuffer,
    OngoingRenderState, RendererState,
};

use super::{new_node_id, RenderNode, RenderNodeItem};

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn new_skin_id() -> usize {
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug)]
struct SkinBuffer {
    buffer: SkinUniformBuffer,
    bind_group: BindGroup,
}

#[derive(Debug, Clone)]
pub struct SkinData {
    pub id: usize,
    pub inverse_bind_matrix: Vec<Mat4>,
}

#[derive(Debug)]
pub struct SkinNode {
    id: usize,
    data: Arc<SkinData>,
    items: Vec<Mat4>,
    buffer: Option<SkinBuffer>,
    invalid: bool,
    pub node: RenderNodeItem,
}

impl SkinNode {
    pub fn new(data: Arc<SkinData>, node: RenderNodeItem) -> Self {
        Self {
            id: new_node_id(),
            data,
            items: Vec::new(),
            buffer: None,
            invalid: true,
            node,
        }
    }

    pub fn skin_id(&self) -> usize {
        self.data.id
    }
}

impl RenderNode for SkinNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(
        &mut self,
        local_context: &LocalContext,
        global_context: &mut GlobalContext,
        invalid: bool,
    ) {
        if let Some(joints) = global_context.updated_joints().get(&self.skin_id()) {
            for (index, matrix) in joints {
                let matrix = *matrix * self.data.inverse_bind_matrix[*index];
                if let Some(buffer) = &mut self.buffer {
                    buffer.buffer.items[*index] = matrix;
                } else {
                    if self.items.len() < *index + 1 {
                        self.items.resize(self.items.len() + 1, Mat4::IDENTITY);
                    }
                    self.items[*index] = matrix;
                }
            }
            self.invalid = true;
        }
        self.node.update(local_context, global_context, invalid);
    }

    fn prepare(&mut self, device: &Device, queue: &Queue, renderer_state: &mut RendererState) {
        if let Some(buffer) = &mut self.buffer {
            if self.invalid {
                buffer.buffer.update(queue);
            }
        } else {
            let items = mem::take(&mut self.items);
            let buffer = SkinUniformBuffer::new(device, items);
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Skin Bind Group"),
                layout: renderer_state.bind_group_layout().instance_uniform_layout(),
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: buffer.buffer().as_entire_binding(),
                }],
            });
            self.buffer = Some(SkinBuffer { buffer, bind_group });
        }
        self.node.prepare(device, queue, renderer_state);
    }

    fn draw<'a>(
        &'a self,
        renderer_state: &'a RendererState,
        ongoing_state: &mut OngoingRenderState<'a>,
    ) {
        if let Some(buffer) = &self.buffer {
            ongoing_state.set_joint(Some(&buffer.bind_group));
            self.node.draw(renderer_state, ongoing_state);
            ongoing_state.set_joint(None);
        } else {
            warn!("No bind group for SkinNode #{}, skip drawing.", self.id);
            warn!("Did you call prepare() before drawing skin node?");
        }
    }
}
