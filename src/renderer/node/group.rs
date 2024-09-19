use core::slice;

use wgpu::{Device, Queue};

use crate::renderer::context::Context;

use super::{
    new_node_id, transform::TransformNode, OngoingRenderState, RenderNode, RenderNodeItem,
    RendererState,
};

#[derive(Default, Debug)]
pub struct GroupNode {
    id: usize,
    label: Option<String>,
    nodes: Vec<RenderNodeItem>,
}

impl RenderNode for GroupNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(&mut self, context: &Context, invalid: bool) -> bool {
        let mut updated = false;
        for item in &mut self.nodes {
            if item.update(context, invalid) {
                updated = true
            }
        }
        updated
    }

    fn prepare(&mut self, device: &Device, queue: &Queue, renderer_state: &mut RendererState) {
        for item in &mut self.nodes {
            item.prepare(device, queue, renderer_state)
        }
    }

    fn draw<'a>(
        &'a self,
        renderer_state: &'a RendererState,
        ongoing_state: &mut OngoingRenderState<'a>,
    ) {
        for item in &self.nodes {
            item.draw(renderer_state, ongoing_state)
        }
    }
}

impl GroupNode {
    pub fn new(label: Option<String>) -> Self {
        Self {
            id: new_node_id(),
            label,
            nodes: Vec::new(),
        }
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn push(&mut self, node: RenderNodeItem) {
        self.nodes.push(node)
    }

    pub fn iter(&self) -> slice::Iter<'_, RenderNodeItem> {
        self.nodes.iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, RenderNodeItem> {
        self.nodes.iter_mut()
    }

    pub fn find_transform_node_mut(&mut self, id: usize) -> Option<&mut TransformNode> {
        fn find_node(item: &mut RenderNodeItem, id: usize) -> Option<&mut TransformNode> {
            match item {
                RenderNodeItem::Group(group) => group.find_transform_node_mut(id),
                RenderNodeItem::Transform(transform) => {
                    if transform.id() == id {
                        Some(transform)
                    } else {
                        find_node(&mut transform.node, id)
                    }
                }
                RenderNodeItem::Joint(joint) => {
                    find_node(&mut joint.node, id).or(find_node(&mut joint.joint_root, id))
                }
                _ => None,
            }
        }

        self.nodes.iter_mut().find_map(|item| find_node(item, id))
    }
}
