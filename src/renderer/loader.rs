use std::{cmp::Ordering, collections::HashMap, iter, sync::Arc, time::Duration};

use glam::Mat4;
use log::warn;
use wgpu::{BindGroup, Device, PrimitiveTopology, Queue};

use crate::{
    asset::{
        animation::{AnimationAsset, AnimationChannelAsset},
        camera::{CameraAsset, CameraProjectionAsset},
        mesh::MeshAsset,
        node::{NodeAsset, NodeAssetId},
        primitive::{PrimitiveAsset, PrimitiveAssetMode},
        scene::SceneAsset,
        skin::SkinAsset,
        texture::{TextureAsset, TextureAssetId},
    },
    renderer::{
        index::IndexBuffer,
        pipeline::{PipelineIdentifier, Pipelines, ShaderType},
        texture::TextureItem,
        vertex::{ColorVertex, TextureVertex, VertexBuffer},
        RendererState,
    },
};

use crate::renderer::node::{
    group::GroupNode,
    primitive::{PrimitiveNode, PrimitiveNodeContent},
    transform::TransformNode,
    RenderNodeItem,
};

use super::{
    animation::{AnimationGroupNode, AnimationNode, AnimationState},
    camera::CameraProjection,
    node::{camera::CameraNode, joint::JointGroupNode, new_node_id, RenderNode},
    vertex::{ColorSkinVertex, TextureSkinVertex},
};

pub struct RendererAssetLoader<'a> {
    state: &'a RendererState,
    texture_cache: HashMap<TextureAssetId, Arc<BindGroup>>,
    animate_nodes: HashMap<NodeAssetId, usize>,
    pipelines: &'a mut Pipelines,
}

impl<'a> RendererAssetLoader<'a> {
    pub fn new(state: &'a RendererState, pipelines: &'a mut Pipelines) -> Self {
        Self {
            state,
            texture_cache: HashMap::new(),
            animate_nodes: HashMap::new(),
            pipelines,
        }
    }

    pub fn load_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        asset: &TextureAsset,
    ) -> Arc<BindGroup> {
        if let Some(texture) = self.texture_cache.get(&asset.id) {
            return texture.clone();
        }
        let texture = TextureItem::from_asset(device, queue, asset, Some(&asset.id.to_string()));
        let bind_group =
            Arc::new(texture.create_bind_group(device, self.state.texture_bind_group_layout()));
        self.texture_cache
            .insert(asset.id.clone(), bind_group.clone());
        bind_group
    }

    pub fn load_primitive(
        &mut self,
        device: &Device,
        queue: &Queue,
        primitive: PrimitiveAsset,
    ) -> PrimitiveNode {
        let indices = primitive
            .indices
            .map(|indices| IndexBuffer::new(device, &indices, None));

        let positions = primitive.positions;
        let vertex_color = primitive.vertex_color;
        let tex_coords = primitive.tex_coords;
        let skins = primitive.skin;
        let diffuse_color = primitive
            .material
            .as_ref()
            .and_then(|material| material.diffuse_color);
        let diffuse_texture = primitive
            .material
            .and_then(|material| material.diffuse_texture);
        let content = match (
            vertex_color.first(),
            tex_coords.first(),
            skins.first(),
            diffuse_texture,
        ) {
            (_, Some(tex_coords), Some(skin), Some(diffuse_texture)) => {
                let vertices: Vec<_> = positions
                    .into_iter()
                    .zip(tex_coords)
                    .zip(&skin.joints)
                    .zip(&skin.weights)
                    .map(|(((position, tex_coords), joint_index), joint_weight)| {
                        TextureSkinVertex {
                            position,
                            tex_coords: *tex_coords,
                            joint_index: *joint_index,
                            joint_weight: *joint_weight,
                        }
                    })
                    .collect();
                let bind_group = self.load_texture(device, queue, &diffuse_texture);
                PrimitiveNodeContent::TextureSkin {
                    buffer: VertexBuffer::new(device, &vertices, primitive.name.as_deref()),
                    bind_group,
                }
            }
            (Some(vertex_color), _, Some(skin), _) => {
                let vertices: Vec<_> = positions
                    .into_iter()
                    .zip(
                        vertex_color
                            .iter()
                            .chain(iter::repeat(&[1.0, 1.0, 1.0, 1.0])),
                    )
                    .zip(&skin.joints)
                    .zip(&skin.weights)
                    .map(
                        |(((position, color), joint_index), joint_weight)| ColorSkinVertex {
                            position,
                            color: *color,
                            joint_index: *joint_index,
                            joint_weight: *joint_weight,
                        },
                    )
                    .collect();
                PrimitiveNodeContent::ColorSkin {
                    buffer: VertexBuffer::new(device, &vertices, primitive.name.as_deref()),
                }
            }
            (_, _, Some(skin), _) => {
                let color = diffuse_color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
                let vertices: Vec<_> = positions
                    .into_iter()
                    .zip(iter::repeat(&color))
                    .zip(&skin.joints)
                    .zip(&skin.weights)
                    .map(
                        |(((position, color), joint_index), joint_weight)| ColorSkinVertex {
                            position,
                            color: *color,
                            joint_index: *joint_index,
                            joint_weight: *joint_weight,
                        },
                    )
                    .collect();
                PrimitiveNodeContent::ColorSkin {
                    buffer: VertexBuffer::new(device, &vertices, primitive.name.as_deref()),
                }
            }
            (_, Some(tex_coords), _, Some(diffuse_texture)) => {
                let vertices: Vec<_> = positions
                    .into_iter()
                    .zip(tex_coords)
                    .map(|(position, tex_coords)| TextureVertex {
                        position,
                        tex_coords: *tex_coords,
                    })
                    .collect();
                let bind_group = self.load_texture(device, queue, &diffuse_texture);
                PrimitiveNodeContent::Texture {
                    buffer: VertexBuffer::new(device, &vertices, primitive.name.as_deref()),
                    bind_group,
                }
            }
            (Some(vertex_color), _, _, _) => {
                let vertices: Vec<_> = positions
                    .into_iter()
                    .zip(
                        vertex_color
                            .iter()
                            .chain(iter::repeat(&[1.0, 1.0, 1.0, 1.0])),
                    )
                    .map(|(position, color)| ColorVertex {
                        position,
                        color: *color,
                    })
                    .collect();
                PrimitiveNodeContent::Color {
                    buffer: VertexBuffer::new(device, &vertices, primitive.name.as_deref()),
                }
            }
            _ => {
                let color = diffuse_color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
                let vertices: Vec<_> = positions
                    .into_iter()
                    .map(|position| ColorVertex { position, color })
                    .collect();
                PrimitiveNodeContent::Color {
                    buffer: VertexBuffer::new(device, &vertices, primitive.name.as_deref()),
                }
            }
        };

        let primitive_topology = match primitive.mode {
            PrimitiveAssetMode::Points => PrimitiveTopology::PointList,
            PrimitiveAssetMode::LineList => PrimitiveTopology::LineList,
            PrimitiveAssetMode::LineStrip => PrimitiveTopology::LineStrip,
            PrimitiveAssetMode::TriangleList => PrimitiveTopology::TriangleList,
            PrimitiveAssetMode::TriangleStrip => PrimitiveTopology::TriangleStrip,
        };
        let pipeline_identifier = PipelineIdentifier {
            shader: match &content {
                PrimitiveNodeContent::Color { .. } => ShaderType::Color,
                PrimitiveNodeContent::Texture { .. } => ShaderType::Texture,
                PrimitiveNodeContent::ColorSkin { .. } => ShaderType::ColorSkin,
                PrimitiveNodeContent::TextureSkin { .. } => ShaderType::TextureSkin,
            },
            primitive_topology,
        };
        let pipeline = self.pipelines.get(device, self.state, pipeline_identifier);
        PrimitiveNode::new(indices, content, pipeline)
    }

    pub fn load_mesh(&mut self, device: &Device, queue: &Queue, mesh: MeshAsset) -> RenderNodeItem {
        let mut target_node = GroupNode::new(mesh.name);
        for primitive in mesh.primitives {
            let primitive = self.load_primitive(device, queue, primitive);
            target_node.push(RenderNodeItem::Primitive(Box::new(primitive)));
        }
        RenderNodeItem::Group(Box::new(target_node))
    }

    pub fn load_skin(
        &mut self,
        device: &Device,
        queue: &Queue,
        skin: SkinAsset,
    ) -> (RenderNodeItem, Vec<usize>, Vec<Mat4>) {
        let joint_asset_ids = skin.joint_ids;
        let mut joint_node_ids = vec![None; joint_asset_ids.len()];
        let joint_root = self.load_node_with_callback(
            device,
            queue,
            *skin.root_joint,
            &mut |asset_id, node_id| {
                if let Some(index) = joint_asset_ids.iter().position(|id| id == asset_id) {
                    joint_node_ids[index] = Some(node_id);
                }
            },
        );

        let joint_node_ids = joint_node_ids
            .into_iter()
            .map(|id| id.expect("Missing joint node id"))
            .collect();
        (joint_root, joint_node_ids, skin.inverse_bind_matrices)
    }

    pub fn load_camera(camera: CameraAsset) -> RenderNodeItem {
        let projection = match camera.projection {
            CameraProjectionAsset::Orthographic(asset) => CameraProjection::Orthographic {
                xmag: asset.xmag,
                ymag: asset.ymag,
                zfar: asset.zfar,
                znear: asset.znear,
            },
            CameraProjectionAsset::Perspective(asset) => CameraProjection::Perspective {
                aspect: asset.aspect_radio,
                yfov: asset.yfov.to_degrees(),
                znear: asset.znear,
                zfar: asset.zfar,
            },
        };
        RenderNodeItem::Camera(Box::new(CameraNode::new(projection, camera.label)))
    }

    pub fn load_node(&mut self, device: &Device, queue: &Queue, node: NodeAsset) -> RenderNodeItem {
        self.load_node_with_callback(device, queue, node, &mut |_, _| {})
    }

    pub fn load_node_with_callback(
        &mut self,
        device: &Device,
        queue: &Queue,
        node: NodeAsset,
        on_transform_applied: &mut impl FnMut(&NodeAssetId, usize),
    ) -> RenderNodeItem {
        let mut target_node = GroupNode::new(node.name);

        if let Some(mesh) = node.mesh {
            let mesh = self.load_mesh(device, queue, mesh);
            target_node.push(mesh);
        }

        if let Some(camera) = node.camera {
            let camera = Self::load_camera(camera);
            target_node.push(camera);
        }

        for child in node.children {
            let child = self.load_node_with_callback(device, queue, child, on_transform_applied);
            target_node.push(child);
        }

        match (node.skin, node.has_animation, node.transform) {
            (Some(skin), true, _) => {
                let (joint_root, joint_ids, inverse_bind_matrices) =
                    self.load_skin(device, queue, skin);
                let target_node = RenderNodeItem::Group(Box::new(target_node));

                let transform = TransformNode::new(target_node);
                on_transform_applied(&node.id, transform.id());
                self.animate_nodes.insert(node.id, transform.id());
                let target_node = RenderNodeItem::Transform(Box::new(transform));

                let joint =
                    JointGroupNode::new(target_node, joint_ids, joint_root, inverse_bind_matrices);
                RenderNodeItem::Joint(Box::new(joint))
            }
            (Some(skin), false, _) => {
                let (joint_root, joint_ids, inverse_bind_matrices) =
                    self.load_skin(device, queue, skin);
                on_transform_applied(&node.id, target_node.id());
                let target_node = RenderNodeItem::Group(Box::new(target_node));

                let joint =
                    JointGroupNode::new(target_node, joint_ids, joint_root, inverse_bind_matrices);
                RenderNodeItem::Joint(Box::new(joint))
            }
            (None, true, transform) => {
                let target_node = RenderNodeItem::Group(Box::new(target_node));
                let transform =
                    TransformNode::from_transform(transform.unwrap_or_default(), target_node);
                on_transform_applied(&node.id, transform.id());
                self.animate_nodes.insert(node.id, transform.id());
                RenderNodeItem::Transform(Box::new(transform))
            }
            (None, false, Some(transform)) => {
                let target_node = RenderNodeItem::Group(Box::new(target_node));
                let transform = TransformNode::from_transform(transform, target_node);
                on_transform_applied(&node.id, transform.id());
                RenderNodeItem::Transform(Box::new(transform))
            }
            (None, false, None) => {
                on_transform_applied(&node.id, target_node.id());
                RenderNodeItem::Group(Box::new(target_node))
            }
        }
    }

    pub fn load_scene(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: SceneAsset,
    ) -> RenderNodeItem {
        let mut target_node = GroupNode::new(scene.name);
        for node in scene.nodes {
            let node = self.load_node(device, queue, node);
            target_node.push(node);
        }
        RenderNodeItem::Group(Box::new(target_node))
    }

    pub fn load_scenes(
        &mut self,
        device: &Device,
        queue: &Queue,
        scenes: Vec<SceneAsset>,
        label: Option<String>,
    ) -> RenderNodeItem {
        let mut target_node = GroupNode::new(label);
        for scene in scenes {
            let scene = self.load_scene(device, queue, scene);
            target_node.push(scene);
        }
        RenderNodeItem::Group(Box::new(target_node))
    }

    pub fn load_animation_channel(&self, channel: AnimationChannelAsset) -> Option<AnimationNode> {
        let target_node = self.animate_nodes.get(&channel.target_id);
        let target_node = match target_node {
            Some(target_node) => *target_node,
            None => {
                warn!(
                    "Node not find when creating animation: {:?}",
                    channel.target_id
                );
                warn!("please check nodes are loaded before animations.");
                return None;
            }
        };
        Some(AnimationNode {
            id: new_node_id(),
            target_node,
            sampler: channel.sampler,
            length: Duration::from_millis((channel.length * 1000.0) as u64),
        })
    }

    pub fn load_animation(&self, animation: AnimationAsset) -> AnimationGroupNode {
        let nodes: Vec<AnimationNode> = animation
            .channels
            .into_iter()
            .filter_map(|channel| self.load_animation_channel(channel))
            .collect();
        let length = nodes
            .iter()
            .map(|channel| channel.length)
            .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
            .unwrap_or(Duration::ZERO);
        AnimationGroupNode {
            id: new_node_id(),
            label: animation.name,
            state: AnimationState::default(),
            length,
            nodes,
        }
    }

    pub fn load_animations(&self, animations: Vec<AnimationAsset>) -> Vec<AnimationGroupNode> {
        animations
            .into_iter()
            .map(|item| self.load_animation(item))
            .collect()
    }
}
