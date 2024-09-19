use std::{collections::BTreeMap, mem};

use glam::Mat4;
use log::warn;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, Device, Queue};

use crate::renderer::{
    context::{Context, DEFAULT_CONTEXT},
    uniform::joint::JointsUniformBuffer,
    OngoingRenderState, RendererState,
};

use super::{new_node_id, RenderNode, RenderNodeItem};

#[derive(Debug)]
struct JointGroupBuffer {
    buffer: JointsUniformBuffer,
    bind_group: BindGroup,
}

#[derive(Debug)]
pub struct JointGroupNode {
    id: usize,
    transforms: Vec<Mat4>,
    invalid: bool,
    buffer: Option<JointGroupBuffer>,
    // node id -> index of items
    joint_id_map: BTreeMap<usize, usize>,
    inverse_bind_matrices: Vec<Mat4>,
    pub node: RenderNodeItem,
    pub joint_root: RenderNodeItem,
}

impl JointGroupNode {
    pub fn new(
        node: RenderNodeItem,
        joint_ids: Vec<usize>,
        joint_root: RenderNodeItem,
        inverse_bind_matrices: Vec<Mat4>,
    ) -> Self {
        let transforms = (0..joint_ids.len())
            .map(|index| {
                inverse_bind_matrices
                    .get(index)
                    .cloned()
                    .unwrap_or(Mat4::IDENTITY)
            })
            .collect();
        let mut joint_id_map = BTreeMap::new();
        for (index, node_id) in joint_ids.into_iter().enumerate() {
            joint_id_map.insert(node_id, index);
        }
        Self {
            id: new_node_id(),
            transforms,
            invalid: false,
            buffer: None,
            node,
            inverse_bind_matrices,
            joint_id_map,
            joint_root,
        }
    }

    pub fn set_items(&mut self, func: impl FnOnce(&mut Vec<Mat4>)) {
        if let Some(buffer) = &mut self.buffer {
            func(&mut buffer.buffer.items);
        } else {
            func(&mut self.transforms);
        }
        self.invalid = true;
    }

    pub fn clone_items(&mut self, new_items: &Vec<Mat4>) {
        if let Some(buffer) = &mut self.buffer {
            buffer.buffer.items.clone_from(new_items);
        } else {
            self.transforms.clone_from(new_items);
        }
        self.invalid = true;
    }

    pub fn joint_ids(&self) -> Vec<usize> {
        let mut ids: Vec<(usize, usize)> = self
            .joint_id_map
            .iter()
            .map(|(node_id, index)| (*node_id, *index))
            .collect();
        ids.sort_by_key(|(_, index)| *index);
        ids.into_iter().map(|(node_id, _)| node_id).collect()
    }

    pub fn joint_matrixs(&self) -> &[Mat4] {
        if let Some(buffer) = &self.buffer {
            &buffer.buffer.items
        } else {
            &self.transforms
        }
    }
}

impl RenderNode for JointGroupNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(&mut self, context: &Context, invalid: bool) -> bool {
        let joint_changed = self.joint_root.update(context, invalid);
        let node_changed = self.node.update(context, invalid);

        if joint_changed {
            let joints = if let Some(buffer) = &mut self.buffer {
                &mut buffer.buffer.items
            } else {
                &mut self.transforms
            };

            fn update_joints(
                joint_id_map: &BTreeMap<usize, usize>,
                joints: &mut Vec<Mat4>,
                inverse_bind_matrices: &[Mat4],
                node: &RenderNodeItem,
            ) {
                match node {
                    RenderNodeItem::Group(group) => group.iter().for_each(|node| {
                        update_joints(joint_id_map, joints, inverse_bind_matrices, node)
                    }),
                    RenderNodeItem::Transform(transform) => {
                        if let Some(joint_index) = joint_id_map.get(&transform.id()) {
                            let joint_matrix =
                                *transform.context().unwrap_or(&DEFAULT_CONTEXT).transform()
                                    * inverse_bind_matrices[*joint_index];
                            joints[*joint_index] = joint_matrix;
                        }
                        update_joints(joint_id_map, joints, inverse_bind_matrices, &transform.node);
                    }
                    RenderNodeItem::Joint(_) => {
                        unreachable!("Nesting joint node is not allowed")
                    }
                    _ => (),
                }
            }

            update_joints(
                &self.joint_id_map,
                joints,
                &self.inverse_bind_matrices,
                &self.joint_root,
            );
            self.invalid = true;
        }

        joint_changed || node_changed
    }

    fn prepare(&mut self, device: &Device, queue: &Queue, renderer_state: &mut RendererState) {
        if let Some(buffer) = &self.buffer {
            if self.invalid {
                buffer.buffer.update(queue);
                self.invalid = false;
            }
        } else {
            if self.transforms.is_empty() {
                warn!("Bad joint node #{}: no joint nodes", self.id);
                return;
            }
            let buffer = JointsUniformBuffer::new(device, mem::take(&mut self.transforms));
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Joint Bind Group"),
                layout: &renderer_state.instance_uniform_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: buffer.buffer().as_entire_binding(),
                }],
            });
            self.buffer = Some(JointGroupBuffer { buffer, bind_group })
        }
        self.joint_root.prepare(device, queue, renderer_state);
        self.node.prepare(device, queue, renderer_state);
    }

    fn draw<'a>(
        &'a self,
        renderer_state: &'a RendererState,
        ongoing_state: &mut OngoingRenderState<'a>,
    ) {
        if let Some(buffer) = &self.buffer {
            self.joint_root.draw(renderer_state, ongoing_state);
            ongoing_state.set_joint(Some(&buffer.bind_group));
            self.node.draw(renderer_state, ongoing_state);
            ongoing_state.set_joint(None);
        } else {
            warn!("No bind group for JointNode #{}, skip drawing.", self.id);
            warn!("Did you call prepare() before drawing joint node?");
        }
    }
}
