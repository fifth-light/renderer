use wgpu::{Device, Queue};

use crate::renderer::{
    context::{GlobalContext, LocalContext},
    OngoingRenderState, RendererState,
};

use super::{new_node_id, RenderNode, RenderNodeItem};

#[derive(Debug)]
pub struct JointNode {
    id: usize,
    skins: Vec<(usize, usize)>,
    first_frame: bool,
    pub node: RenderNodeItem,
}

impl JointNode {
    pub fn new(skins: Vec<(usize, usize)>, node: RenderNodeItem) -> Self {
        Self {
            id: new_node_id(),
            skins,
            first_frame: true,
            node,
        }
    }

    pub fn joints(&self) -> &[(usize, usize)] {
        &self.skins
    }
}

impl RenderNode for JointNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(
        &mut self,
        local_context: &LocalContext,
        global_context: &mut GlobalContext,
        invalid: bool,
    ) {
        if invalid || self.first_frame {
            for (skin, joint_index) in self.joints() {
                global_context.update_joint(*skin, *joint_index, *local_context.transform());
            }
            self.first_frame = false;
        }
        self.node.update(local_context, global_context, invalid)
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
