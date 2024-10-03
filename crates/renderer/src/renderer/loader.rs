use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    iter,
    sync::Arc,
    time::Duration,
};

use log::warn;
use wgpu::{BindGroup, Device, PrimitiveTopology, Queue};

use crate::{
    asset::{
        animation::{AnimationAsset, AnimationChannelAsset},
        camera::{CameraAsset, CameraProjectionAsset},
        material::MaterialAlphaMode,
        mesh::MeshAsset,
        node::{NodeAsset, NodeAssetId},
        primitive::{PrimitiveAsset, PrimitiveAssetMode},
        scene::SceneAsset,
        skin::{SkinAsset, SkinAssetId},
        texture::{TextureAsset, TextureAssetId},
    },
    renderer::{
        index::IndexBuffer,
        pipeline::{PipelineIdentifier, Pipelines, ShaderType},
        texture::TextureItem,
        vertex::{ColorVertex, TextureVertex, VertexBuffer},
    },
};

use crate::renderer::node::{
    group::GroupNode,
    primitive::{PrimitiveNode, PrimitiveNodeContent},
    transform::TransformNode,
    RenderNodeItem,
};

use super::{
    animation::{AnimationGroupNode, AnimationNode},
    camera::CameraProjection,
    node::{
        camera::CameraNode,
        joint::JointNode,
        skin::{new_skin_id, SkinData, SkinNode},
        RenderNode,
    },
    pipeline::ShaderAlphaMode,
    tangent::calculate_tangent,
    vertex::{ColorSkinVertex, TextureSkinVertex},
    RendererBindGroupLayout,
};

pub struct RendererAssetLoader<'a> {
    bind_group_layouts: &'a RendererBindGroupLayout,
    texture_cache: HashMap<TextureAssetId, Arc<BindGroup>>,
    animate_nodes: HashMap<NodeAssetId, usize>,
    skins: HashMap<SkinAssetId, Arc<SkinData>>,
    pipelines: &'a mut Pipelines,
}

impl<'a> RendererAssetLoader<'a> {
    pub fn new(
        bind_group_layouts: &'a RendererBindGroupLayout,
        pipelines: &'a mut Pipelines,
    ) -> Self {
        Self {
            bind_group_layouts,
            texture_cache: HashMap::new(),
            animate_nodes: HashMap::new(),
            skins: HashMap::new(),
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
        let bind_group = Arc::new(
            texture.create_bind_group(device, self.bind_group_layouts.texture_bind_layout()),
        );
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
        let positions = primitive.positions;
        let tangent = calculate_tangent(primitive.mode, &positions, primitive.indices.as_deref());
        let indices = primitive
            .indices
            .map(|indices| IndexBuffer::new(device, &indices, None));
        let normals = primitive.normals;
        let vertex_color = primitive.vertex_color;
        let tex_coords = primitive.tex_coords;
        let skins = primitive.skin;
        let alpha_mode = primitive
            .material
            .as_ref()
            .map(|material| material.alpha_mode.unwrap_or_default())
            .unwrap_or_default();
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
                    .zip(normals)
                    .zip(tangent)
                    .zip(tex_coords)
                    .zip(&skin.joints)
                    .zip(&skin.weights)
                    .map(
                        |(
                            ((((position, normal), tangent), tex_coords), joint_index),
                            joint_weight,
                        )| {
                            TextureSkinVertex {
                                position,
                                normal,
                                tangent,
                                tex_coords: *tex_coords,
                                joint_index: *joint_index,
                                joint_weight: *joint_weight,
                            }
                        },
                    )
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
                    .zip(normals)
                    .zip(tangent)
                    .zip(&skin.joints)
                    .zip(&skin.weights)
                    .map(
                        |(((((position, color), normal), tangent), joint_index), joint_weight)| {
                            ColorSkinVertex {
                                position,
                                color: *color,
                                normal,
                                tangent,
                                joint_index: *joint_index,
                                joint_weight: *joint_weight,
                            }
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
                    .zip(normals)
                    .zip(tangent)
                    .zip(&skin.joints)
                    .zip(&skin.weights)
                    .map(
                        |(((((position, color), normal), tangent), joint_index), joint_weight)| {
                            ColorSkinVertex {
                                position,
                                color: *color,
                                normal,
                                tangent,
                                joint_index: *joint_index,
                                joint_weight: *joint_weight,
                            }
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
                    .zip(normals)
                    .zip(tangent)
                    .map(
                        |(((position, tex_coords), normal), tangent)| TextureVertex {
                            position,
                            tex_coords: *tex_coords,
                            normal,
                            tangent,
                        },
                    )
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
                    .zip(normals)
                    .zip(tangent)
                    .map(|(((position, color), normal), tangent)| ColorVertex {
                        position,
                        color: *color,
                        normal,
                        tangent,
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
                    .zip(normals)
                    .zip(tangent)
                    .map(|((position, normal), tangent)| ColorVertex {
                        position,
                        color,
                        normal,
                        tangent,
                    })
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
            alpha_mode: match alpha_mode {
                MaterialAlphaMode::Opaque => ShaderAlphaMode::Opaque,
                MaterialAlphaMode::Mask => ShaderAlphaMode::Mask,
                MaterialAlphaMode::Blend => ShaderAlphaMode::Blend,
            },
        };
        let pipeline =
            self.pipelines
                .get(device, self.bind_group_layouts, pipeline_identifier, false);
        let outline_pipeline =
            self.pipelines
                .get(device, self.bind_group_layouts, pipeline_identifier, true);
        PrimitiveNode::new(indices, content, pipeline, Some(outline_pipeline))
    }

    pub fn load_mesh(&mut self, device: &Device, queue: &Queue, mesh: MeshAsset) -> RenderNodeItem {
        let mut target_node = GroupNode::new(mesh.name);
        for primitive in mesh.primitives {
            let primitive = self.load_primitive(device, queue, primitive);
            target_node.push(RenderNodeItem::Primitive(Box::new(primitive)));
        }
        RenderNodeItem::Group(Box::new(target_node))
    }

    pub fn load_skin(&self, skin: &SkinAsset) -> Arc<SkinData> {
        let data = SkinData {
            id: new_skin_id(),
            inverse_bind_matrix: skin.inverse_bind_matrices.clone(),
        };
        Arc::new(data)
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

    pub fn load_node(
        &mut self,
        device: &Device,
        queue: &Queue,
        node: NodeAsset,
        skins: &HashMap<SkinAssetId, Arc<SkinAsset>>,
        joint_ids: &HashMap<NodeAssetId, BTreeSet<SkinAssetId>>,
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
            let child = self.load_node(device, queue, child, skins, joint_ids);
            target_node.push(child);
        }

        let mut target_node = RenderNodeItem::Group(Box::new(target_node));

        if let Some(joint_skins) = joint_ids.get(&node.id) {
            let skin_indexes = joint_skins
                .iter()
                .map(|skin_id| {
                    let skin_data = self.skins.get(skin_id).expect("Skin not found");
                    let skin = skins.get(skin_id).expect("Skin not found");
                    let joint_index = skin
                        .joint_ids
                        .iter()
                        .position(|id| *id == node.id)
                        .expect("Joint not found");
                    (skin_data.id, joint_index)
                })
                .collect();
            let joint_node = JointNode::new(skin_indexes, target_node);
            target_node = RenderNodeItem::Joint(Box::new(joint_node));
        }

        match (node.has_animation, node.transform) {
            (true, transform) => {
                let transform =
                    TransformNode::from_transform(transform.unwrap_or_default(), target_node);
                self.animate_nodes.insert(node.id, transform.id());
                RenderNodeItem::Transform(Box::new(transform))
            }
            (false, Some(transform)) => {
                let transform = TransformNode::from_transform(transform, target_node);
                RenderNodeItem::Transform(Box::new(transform))
            }
            (false, None) => target_node,
        }
    }

    pub fn load_scene(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: SceneAsset,
    ) -> RenderNodeItem {
        for (id, skin) in &scene.skins {
            self.skins.insert(id.clone(), self.load_skin(skin));
        }
        let mut target_node = GroupNode::new(scene.name);
        for node in scene.nodes {
            let node = self.load_node(device, queue, node, &scene.skins, &scene.joint_nodes);
            target_node.push(node);
        }
        for node in scene.skinned_nodes {
            let skin = node.skin.clone().unwrap();
            let node = self.load_node(device, queue, node, &scene.skins, &scene.joint_nodes);
            let skin = self.skins.get(&skin.id).unwrap();
            let skin_node = SkinNode::new(skin.clone(), node);
            target_node.push(RenderNodeItem::Skin(Box::new(skin_node)));
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
        Some(AnimationNode::new(
            target_node,
            channel.sampler,
            Duration::from_millis((channel.length * 1000.0) as u64),
        ))
    }

    pub fn load_animation(&self, animation: AnimationAsset) -> AnimationGroupNode {
        let nodes: Vec<AnimationNode> = animation
            .channels
            .into_iter()
            .filter_map(|channel| self.load_animation_channel(channel))
            .collect();
        let length = nodes
            .iter()
            .map(|node| *node.length())
            .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
            .unwrap_or(Duration::ZERO);
        AnimationGroupNode::new(nodes, length, animation.name)
    }

    pub fn load_animations(&self, animations: Vec<AnimationAsset>) -> Vec<AnimationGroupNode> {
        animations
            .into_iter()
            .map(|item| self.load_animation(item))
            .collect()
    }
}
