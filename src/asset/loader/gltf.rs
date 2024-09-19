use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    iter,
    path::{Path, PathBuf},
    sync::Arc,
};

use glam::{Mat4, Quat, Vec3};
use gltf::{
    accessor::{DataType, Dimensions},
    animation::{Channel, Interpolation, Property, Sampler},
    camera::Projection,
    image::Format,
    mesh::Mode,
    scene::Transform,
    texture::{MagFilter, MinFilter, WrappingMode},
    Accessor, Animation, Camera, Document, Material, Mesh, Node, Primitive, Scene, Semantic, Skin,
    Texture,
};

use crate::asset::{
    animation::{
        AnimationAsset, AnimationChannelAsset, AnimationKeyFrame, AnimationKeyFrames,
        AnimationSampler,
    },
    camera::{CameraAsset, CameraProjectionAsset, OrthographicCameraAsset, PerspectiveCameraAsset},
    material::MaterialAsset,
    mesh::MeshAsset,
    node::{DecomposedTransform, MatrixNodeTransform, NodeAsset, NodeAssetId, NodeTransform},
    primitive::{PrimitiveAsset, PrimitiveAssetMode, PrimitiveSkin, TexCoords, VertexColor},
    scene::SceneAsset,
    skin::SkinAsset,
    texture::{
        SamplerAsset, TextureAsset, TextureAssetFormat, TextureAssetId, TextureMagFilter,
        TextureMinFilter, TextureMipmapFilter, TextureWrappingMode,
    },
};

use super::pad_color_vec3_to_vec4;

#[derive(Debug, Clone, PartialEq, Eq)]
enum AnimationPathType {
    Rotation,
    Translation,
    Scale,
}

#[derive(Debug, Clone)]
pub enum GltfIdentifier {
    Path(PathBuf),
}

impl From<(GltfIdentifier, usize)> for TextureAssetId {
    fn from(value: (GltfIdentifier, usize)) -> Self {
        match value {
            (GltfIdentifier::Path(path), index) => TextureAssetId::PathIndex(path, index),
        }
    }
}

impl From<(GltfIdentifier, usize)> for NodeAssetId {
    fn from(value: (GltfIdentifier, usize)) -> Self {
        match value {
            (GltfIdentifier::Path(path), index) => NodeAssetId::PathIndex(path, index),
        }
    }
}

#[derive(Debug)]
pub struct GltfData {
    id: GltfIdentifier,
    buffers: Vec<gltf::buffer::Data>,
    images: Vec<gltf::image::Data>,
}

fn load_texture_sampler(sampler: gltf::texture::Sampler) -> SamplerAsset {
    let (min_filter, mipmap_filter) = sampler
        .min_filter()
        .map(|filter| match filter {
            MinFilter::Nearest => (TextureMinFilter::Nearest, TextureMipmapFilter::default()),
            MinFilter::Linear => (TextureMinFilter::Linear, TextureMipmapFilter::default()),
            MinFilter::NearestMipmapNearest => {
                (TextureMinFilter::Nearest, TextureMipmapFilter::Nearest)
            }
            MinFilter::LinearMipmapNearest => {
                (TextureMinFilter::Linear, TextureMipmapFilter::Nearest)
            }
            MinFilter::NearestMipmapLinear => {
                (TextureMinFilter::Nearest, TextureMipmapFilter::Linear)
            }
            MinFilter::LinearMipmapLinear => {
                (TextureMinFilter::Linear, TextureMipmapFilter::Linear)
            }
        })
        .unwrap_or_default();

    fn wrapping_mode(mode: WrappingMode) -> TextureWrappingMode {
        match mode {
            WrappingMode::ClampToEdge => TextureWrappingMode::ClampToEdge,
            WrappingMode::MirroredRepeat => TextureWrappingMode::MirroredRepeat,
            WrappingMode::Repeat => TextureWrappingMode::Repeat,
        }
    }

    SamplerAsset {
        mag_filter: sampler
            .mag_filter()
            .map(|filter| match filter {
                MagFilter::Nearest => TextureMagFilter::Nearest,
                MagFilter::Linear => TextureMagFilter::Linear,
            })
            .unwrap_or_default(),
        min_filter,
        mipmap_filter,
        wrap_x: wrapping_mode(sampler.wrap_s()),
        wrap_y: wrapping_mode(sampler.wrap_t()),
    }
}

fn load_camera(camera: Camera) -> CameraAsset {
    let data = match camera.projection() {
        Projection::Orthographic(orthographic) => {
            CameraProjectionAsset::Orthographic(OrthographicCameraAsset {
                xmag: orthographic.xmag(),
                ymag: orthographic.ymag(),
                zfar: orthographic.zfar(),
                znear: orthographic.znear(),
            })
        }
        Projection::Perspective(perspective) => {
            CameraProjectionAsset::Perspective(PerspectiveCameraAsset {
                aspect_radio: perspective.aspect_ratio(),
                yfov: perspective.yfov(),
                zfar: perspective.zfar(),
                znear: perspective.znear(),
            })
        }
    };
    CameraAsset {
        projection: data,
        label: camera.name().map(str::to_string),
    }
}

fn load_accessor(data: &GltfData, accessor: &Accessor) -> Vec<u8> {
    let view = if let Some(view) = accessor.view() {
        view
    } else {
        return iter::repeat(0).take(accessor.count()).collect();
    };
    let num_size: usize = match accessor.data_type() {
        DataType::I8 => 1,
        DataType::U8 => 1,
        DataType::I16 => 2,
        DataType::U16 => 2,
        DataType::U32 => 4,
        DataType::F32 => 4,
    };
    let item_size: usize = match accessor.dimensions() {
        Dimensions::Scalar => 1,
        Dimensions::Vec2 => 2,
        Dimensions::Vec3 => 3,
        Dimensions::Vec4 => 4,
        Dimensions::Mat2 => 4,
        Dimensions::Mat3 => 9,
        Dimensions::Mat4 => 16,
    };
    let item_length: usize = num_size * item_size;

    let count = accessor.count();
    let buffer = &data.buffers[view.buffer().index()];
    let offset = accessor.offset() + view.offset();
    let stride = view.stride().unwrap_or(item_length);
    let interval = stride - item_length;
    let full_length = count * (item_length + interval);

    assert!(offset + full_length <= buffer.len());

    if stride == 0 {
        (offset..offset + full_length)
            .map(|index| buffer.0[index])
            .collect()
    } else {
        let read_length = item_length * count;
        let mut result = Vec::new();
        let mut chunk_left = item_length;
        let mut index = offset;
        let mut read_amount = 0;
        while read_amount < read_length {
            result.push(buffer.0[index]);
            read_amount += 1;
            index += 1;
            chunk_left -= 1;
            if chunk_left == 0 {
                index += interval;
                chunk_left = item_length;
            }
        }
        result
    }
}

fn load_accessor_f32(data: &GltfData, accessor: &Accessor) -> Vec<f32> {
    assert_eq!(accessor.data_type(), DataType::F32);
    let data = load_accessor(data, accessor);
    data.chunks_exact(4)
        .map(|chunk| {
            let array = chunk.try_into().unwrap();
            f32::from_le_bytes(array)
        })
        .collect()
}

fn load_accessor_u8(data: &GltfData, accessor: &Accessor) -> Vec<u8> {
    assert_eq!(accessor.data_type(), DataType::U8);
    load_accessor(data, accessor)
}

fn normalize_u8(data: Vec<u8>) -> Vec<f32> {
    data.into_iter()
        .map(|item| item as f32 / u8::MAX as f32)
        .collect()
}

fn load_accessor_u16(data: &GltfData, accessor: &Accessor) -> Vec<u16> {
    assert_eq!(accessor.data_type(), DataType::U16);
    let data = load_accessor(data, accessor);
    data.chunks_exact(2)
        .map(|chunk| {
            let array = chunk.try_into().unwrap();
            u16::from_le_bytes(array)
        })
        .collect()
}

fn normalize_u16(data: Vec<u16>) -> Vec<f32> {
    data.into_iter()
        .map(|item| item as f32 / u16::MAX as f32)
        .collect()
}

fn load_accessor_u32(data: &GltfData, accessor: &Accessor) -> Vec<u32> {
    assert_eq!(accessor.data_type(), DataType::U32);
    let data = load_accessor(data, accessor);
    data.chunks_exact(4)
        .map(|chunk| {
            let array = chunk.try_into().unwrap();
            u32::from_le_bytes(array)
        })
        .collect()
}

fn normalize_u32(data: Vec<u32>) -> Vec<f32> {
    data.into_iter()
        .map(|item| item as f32 / u32::MAX as f32)
        .collect()
}

fn load_accessor_normalized(data: &GltfData, accessor: &Accessor) -> Vec<f32> {
    match accessor.data_type() {
        DataType::U8 => normalize_u8(load_accessor_u8(data, accessor)),
        DataType::U16 => normalize_u16(load_accessor_u16(data, accessor)),
        DataType::U32 => normalize_u32(load_accessor_u32(data, accessor)),
        DataType::F32 => load_accessor_f32(data, accessor),
        _ => unreachable!("Invalid normalized data"),
    }
}

fn chunk_vec3<T: Copy>(data: Vec<T>) -> Vec<[T; 3]> {
    data.chunks_exact(3)
        .map(|item| item.try_into().unwrap())
        .collect()
}

fn chunk_vec4<T: Copy>(data: Vec<T>) -> Vec<[T; 4]> {
    data.chunks_exact(4)
        .map(|item| item.try_into().unwrap())
        .collect()
}

fn chunk_and_clamp_vec3_to_vec4_f32(data: Vec<f32>) -> Vec<[f32; 4]> {
    data.chunks_exact(3)
        .map(|item| {
            let array: [f32; 3] = item.try_into().unwrap();
            array.map(|num| num.clamp(0.0, 1.0));
            pad_color_vec3_to_vec4(array)
        })
        .collect()
}

fn chunk_and_clamp_vec4_f32(data: Vec<f32>) -> Vec<[f32; 4]> {
    data.chunks_exact(4)
        .map(|item| {
            let array: [f32; 4] = item.try_into().unwrap();
            array.map(|num| num.clamp(0.0, 1.0))
        })
        .collect()
}

fn chunk_mat4(data: Vec<f32>) -> Vec<Mat4> {
    data.chunks_exact(16)
        .map(|item| {
            let array = item.try_into().unwrap();
            Mat4::from_cols_array(&array)
        })
        .collect()
}

struct GltfDocumentLoader<'a> {
    data: &'a GltfData,
    texture_cache: HashMap<usize, Arc<TextureAsset>>,
    animated_nodes: BTreeSet<usize>,
    joint_nodes: BTreeSet<usize>,
}

impl<'a> GltfDocumentLoader<'a> {
    fn new(data: &'a GltfData) -> Self {
        Self {
            data,
            texture_cache: HashMap::new(),
            animated_nodes: BTreeSet::new(),
            joint_nodes: BTreeSet::new(),
        }
    }

    fn load_texture(&mut self, texture: Texture) -> Arc<TextureAsset> {
        let sampler = load_texture_sampler(texture.sampler());
        let image = texture.source();
        let index: usize = image.index();
        let id: TextureAssetId = (self.data.id.clone(), index).into();

        if let Some(asset) = self.texture_cache.get(&index) {
            return asset.clone();
        }

        let image = &self.data.images[index];
        let format = match image.format {
            Format::R8 => TextureAssetFormat::Ru8,
            Format::R8G8 => TextureAssetFormat::Rgu8,
            Format::R8G8B8 => TextureAssetFormat::Rgbu8,
            Format::R8G8B8A8 => TextureAssetFormat::Rgbau8,
            Format::R16 => TextureAssetFormat::Ru16,
            Format::R16G16 => TextureAssetFormat::Rgu16,
            Format::R16G16B16 => TextureAssetFormat::Rgbu16,
            Format::R16G16B16A16 => TextureAssetFormat::Rgbau16,
            _ => todo!("Unsupported texture format"),
        };
        let asset = Arc::new(TextureAsset {
            id,
            size: (image.width, image.height),
            format,
            data: image.pixels.clone(),
            sampler,
        });
        self.texture_cache.insert(index, asset.clone());
        asset
    }

    fn load_material(&mut self, material: Material) -> MaterialAsset {
        let pbr_matallic_roughness = material.pbr_metallic_roughness();
        let diffuse_color = pbr_matallic_roughness.base_color_factor();
        let diffuse_texture = if let Some(texture) = pbr_matallic_roughness.base_color_texture() {
            let texture = texture.texture();
            let texture = self.load_texture(texture);
            Some(texture)
        } else {
            None
        };

        MaterialAsset {
            name: material.name().map(str::to_string),
            diffuse_color: Some(diffuse_color),
            diffuse_texture,
        }
    }

    fn load_primitive(&mut self, primitive: Primitive) -> PrimitiveAsset {
        let material = self.load_material(primitive.material());

        let indices = primitive
            .indices()
            .map(|accessor| match accessor.data_type() {
                DataType::U8 => load_accessor_u8(self.data, &accessor)
                    .into_iter()
                    .map(|item| item as u32)
                    .collect(),
                DataType::U16 => load_accessor_u16(self.data, &accessor)
                    .into_iter()
                    .map(|item| item as u32)
                    .collect(),
                DataType::U32 => load_accessor_u32(self.data, &accessor),
                _ => unreachable!("Unsupported index type"),
            });

        let mut positions: Option<Vec<[f32; 3]>> = None;
        let mut tex_coords = Vec::new();
        let mut vertex_color = Vec::new();
        let mut joints = Vec::new();
        let mut weights = Vec::new();

        fn ensure_size<T>(vec: &mut Vec<Option<T>>, size: usize) {
            if size > vec.len() {
                vec.resize_with(size, || None);
            }
        }

        for (semantic, accessor) in primitive.attributes() {
            match semantic {
                Semantic::Positions => {
                    assert_eq!(accessor.dimensions(), Dimensions::Vec3);
                    assert_eq!(accessor.data_type(), DataType::F32);
                    let data = load_accessor_f32(self.data, &accessor);
                    positions = Some(chunk_vec3(data));
                }
                Semantic::Colors(index) => {
                    ensure_size(&mut vertex_color, index as usize + 1);
                    let data = load_accessor_normalized(self.data, &accessor);
                    let color = match accessor.dimensions() {
                        Dimensions::Vec3 => chunk_and_clamp_vec3_to_vec4_f32(data),
                        Dimensions::Vec4 => chunk_and_clamp_vec4_f32(data),
                        _ => unreachable!("Unsupported vertex color"),
                    };
                    vertex_color[index as usize] = Some(color);
                }
                Semantic::TexCoords(index) => {
                    ensure_size(&mut tex_coords, index as usize + 1);
                    assert_eq!(accessor.dimensions(), Dimensions::Vec2);
                    let data = load_accessor_normalized(self.data, &accessor);
                    let coords: Vec<_> = data
                        .chunks_exact(2)
                        .map(|chunk| chunk.try_into().unwrap())
                        .collect();
                    tex_coords[index as usize] = Some(coords);
                }
                Semantic::Joints(index) => {
                    ensure_size(&mut joints, index as usize + 1);
                    assert_eq!(accessor.dimensions(), Dimensions::Vec4);
                    let data = match accessor.data_type() {
                        DataType::U8 => load_accessor_u8(self.data, &accessor)
                            .into_iter()
                            .map(|num| num as u16)
                            .collect(),
                        DataType::U16 => load_accessor_u16(self.data, &accessor),
                        _ => unreachable!("Unsupported joints"),
                    };
                    let data = chunk_vec4(data);
                    joints[index as usize] = Some(data);
                }
                Semantic::Weights(index) => {
                    ensure_size(&mut weights, index as usize + 1);
                    assert_eq!(accessor.dimensions(), Dimensions::Vec4);
                    let data = load_accessor_normalized(self.data, &accessor);
                    let data = chunk_vec4(data);
                    weights[index as usize] = Some(data);
                }
                _ => (),
            }
        }

        let tex_coords: Vec<TexCoords> = tex_coords
            .into_iter()
            .map(|tex_coords| tex_coords.expect("Missing texture coordinates set"))
            .collect();
        let vertex_color: Vec<VertexColor> = vertex_color
            .into_iter()
            .map(|vertex_color| vertex_color.expect("Missing vertex color set"))
            .collect();
        let skin = joints
            .into_iter()
            .zip(weights)
            .map(|(joints, weights)| {
                let joints = joints.expect("Missing joints set");
                let weights = weights.expect("Missing weights set");
                PrimitiveSkin { joints, weights }
            })
            .collect();

        let mode = match primitive.mode() {
            Mode::Points => PrimitiveAssetMode::Points,
            Mode::Lines => PrimitiveAssetMode::LineList,
            Mode::LineStrip => PrimitiveAssetMode::LineStrip,
            Mode::Triangles => PrimitiveAssetMode::TriangleList,
            Mode::TriangleStrip => PrimitiveAssetMode::TriangleStrip,
            Mode::LineLoop => todo!("Unsupported primitive asset mode: LineLoop"),
            Mode::TriangleFan => todo!("Unsupported primitive asset mode: TriangleFan"),
        };

        PrimitiveAsset {
            name: None,
            positions: positions.expect("No positions in primitive"),
            tex_coords,
            vertex_color,
            indices,
            skin,
            material: Some(material),
            mode,
        }
    }

    fn load_mesh(&mut self, mesh: Mesh) -> MeshAsset {
        let primitives = mesh
            .primitives()
            .map(|primitive| self.load_primitive(primitive))
            .collect();
        MeshAsset {
            name: mesh.name().map(str::to_string),
            primitives,
        }
    }

    fn load_skin(&mut self, skin: Skin) -> SkinAsset {
        let joint_ids: Vec<usize> = skin.joints().map(|joint| joint.index()).collect();
        let mut root_nodes: BTreeSet<usize> = joint_ids.iter().cloned().collect();
        let joint_ids = joint_ids
            .into_iter()
            .map(|index| (self.data.id.clone(), index).into())
            .collect();
        for node in skin.joints() {
            node.children().for_each(|node| {
                root_nodes.remove(&node.index());
            });
        }
        assert_eq!(root_nodes.len(), 1);
        let root_joint_id = root_nodes.into_iter().next().unwrap();
        let root_joint = skin
            .joints()
            .find(|node| node.index() == root_joint_id)
            .unwrap();
        let root_joint = self.load_node(root_joint);

        let inverse_bind_matrices = skin
            .inverse_bind_matrices()
            .map(|accessor| {
                let data = load_accessor_f32(self.data, &accessor);
                chunk_mat4(data)
            })
            .unwrap_or_default();
        SkinAsset {
            joint_ids,
            root_joint: Box::new(root_joint),
            inverse_bind_matrices,
        }
    }

    fn load_node(&mut self, node: Node) -> NodeAsset {
        let id = (self.data.id.clone(), node.index()).into();
        let transform = match node.transform() {
            Transform::Matrix { matrix } => {
                NodeTransform::Matrix(MatrixNodeTransform(Mat4::from_cols_array_2d(&matrix)))
            }
            Transform::Decomposed {
                translation,
                rotation,
                scale,
            } => NodeTransform::Decomposed(DecomposedTransform {
                translation: Vec3::from_array(translation),
                rotation: Quat::from_array(rotation),
                scale: Vec3::from_array(scale),
            }),
        };
        let mesh = node.mesh().map(|mesh| self.load_mesh(mesh));
        let skin = node.skin().map(|skin| self.load_skin(skin));
        let camera = node.camera().map(load_camera);
        let children = node.children().map(|child| self.load_node(child)).collect();
        let has_animation = self.animated_nodes.contains(&node.index());

        NodeAsset {
            id,
            name: node.name().map(str::to_string),
            transform: Some(transform),
            mesh,
            skin,
            camera,
            has_animation,
            children,
        }
    }

    fn load_scene(&mut self, scene: Scene) -> SceneAsset {
        let nodes: Vec<_> = scene
            .nodes()
            .filter(|node| !self.joint_nodes.contains(&node.index()))
            .collect();
        let (mut nodes, mut skinned_nodes) = nodes
            .into_iter()
            .map(|node| self.load_node(node))
            .partition(|node| node.skin.is_none());

        fn take_out_skinned_nodes(node: &mut NodeAsset, target: &mut Vec<NodeAsset>) {
            let mut i = 0;
            while i < node.children.len() {
                let child = &mut node.children[i];
                if child.skin.is_some() {
                    let child = node.children.remove(i);
                    target.push(child);
                } else {
                    for child in &mut child.children {
                        take_out_skinned_nodes(child, target);
                    }
                    i += 1;
                }
            }
        }
        for node in &mut nodes {
            take_out_skinned_nodes(node, &mut skinned_nodes);
        }

        SceneAsset {
            name: scene.name().map(str::to_string),
            nodes,
            skinned_nodes,
        }
    }

    fn load_animation_sampler(
        &self,
        sampler: Sampler,
        path_type: AnimationPathType,
    ) -> (AnimationSampler, f32) {
        let time = load_accessor_f32(self.data, &sampler.input());

        fn read_keyframes<T: Debug + Clone>(
            time: Vec<f32>,
            data: Vec<T>,
            repeat_time: bool,
        ) -> (Vec<AnimationKeyFrame<T>>, f32) {
            let keyframes: Vec<AnimationKeyFrame<T>> = if repeat_time {
                time.into_iter()
                    .flat_map(|time| iter::repeat(time).take(3))
                    .zip(data)
                    .map(|(time, value)| AnimationKeyFrame { time, value })
                    .collect()
            } else {
                time.into_iter()
                    .zip(data)
                    .map(|(time, value)| AnimationKeyFrame { time, value })
                    .collect()
            };
            let length = keyframes
                .iter()
                .max_by(|x, y| x.time.total_cmp(&y.time))
                .map(|item| item.time)
                .unwrap_or(0.0);
            (keyframes, length)
        }

        fn split_keyframes<T: Debug + Clone>(
            keyframes: Vec<AnimationKeyFrame<T>>,
        ) -> Vec<AnimationKeyFrame<(T, T, T)>> {
            let split = keyframes
                .chunks_exact(3)
                .map(|chunks| AnimationKeyFrame {
                    time: chunks[0].time,
                    value: (
                        chunks[0].value.clone(),
                        chunks[1].value.clone(),
                        chunks[2].value.clone(),
                    ),
                })
                .collect();
            split
        }

        fn interpolate_frames<T: Debug + Clone>(
            keyframes: Vec<AnimationKeyFrame<T>>,
            interpolation: Interpolation,
        ) -> AnimationKeyFrames<T> {
            match interpolation {
                Interpolation::Linear => AnimationKeyFrames::Linear(keyframes),
                Interpolation::Step => AnimationKeyFrames::Step(keyframes),
                Interpolation::CubicSpline => {
                    AnimationKeyFrames::CubicSpline(split_keyframes(keyframes))
                }
            }
        }

        match path_type {
            AnimationPathType::Rotation => {
                let data = load_accessor_normalized(self.data, &sampler.output());
                let repeat_times = sampler.interpolation() == Interpolation::CubicSpline;
                let (keyframes, length) = read_keyframes(time, chunk_vec4(data), repeat_times);
                let keyframes = interpolate_frames(keyframes, sampler.interpolation());
                (AnimationSampler::Rotation(keyframes), length)
            }
            AnimationPathType::Translation => {
                let data = load_accessor_f32(self.data, &sampler.output());
                let repeat_times = sampler.interpolation() == Interpolation::CubicSpline;
                let (keyframes, length) = read_keyframes(time, chunk_vec3(data), repeat_times);
                let keyframes = interpolate_frames(keyframes, sampler.interpolation());
                (AnimationSampler::Translation(keyframes), length)
            }
            AnimationPathType::Scale => {
                let data = load_accessor_f32(self.data, &sampler.output());
                let repeat_times = sampler.interpolation() == Interpolation::CubicSpline;
                let (keyframes, length) = read_keyframes(time, chunk_vec3(data), repeat_times);
                let keyframes = interpolate_frames(keyframes, sampler.interpolation());
                (AnimationSampler::Scale(keyframes), length)
            }
        }
    }

    fn load_animation_channel(&mut self, channel: Channel) -> AnimationChannelAsset {
        let target = channel.target();
        let path_type = match target.property() {
            Property::Translation => AnimationPathType::Translation,
            Property::Rotation => AnimationPathType::Rotation,
            Property::Scale => AnimationPathType::Scale,
            Property::MorphTargetWeights => todo!(),
        };
        let (sampler, length) = self.load_animation_sampler(channel.sampler(), path_type);
        let target = target.node();
        self.animated_nodes.insert(target.index());
        let target_id = (self.data.id.clone(), target.index()).into();
        AnimationChannelAsset {
            sampler,
            length,
            target_id,
        }
    }

    fn load_animation(&mut self, animation: Animation) -> AnimationAsset {
        let channels = animation
            .channels()
            .map(|channel| self.load_animation_channel(channel))
            .collect();
        AnimationAsset {
            name: animation.name().map(str::to_string),
            channels,
        }
    }

    fn find_joint_nodes(&mut self, document: &Document) {
        for skin in document.skins() {
            for joint in skin.joints() {
                self.joint_nodes.insert(joint.index());
            }
        }
    }

    fn load(&mut self, document: Document) -> (Vec<SceneAsset>, Vec<AnimationAsset>) {
        self.find_joint_nodes(&document);
        let animations = document
            .animations()
            .map(|scene| self.load_animation(scene))
            .collect();
        let scenes = document
            .scenes()
            .map(|scene| self.load_scene(scene))
            .collect();
        (scenes, animations)
    }
}

pub fn load_from_path<P>(path: P) -> Result<(Vec<SceneAsset>, Vec<AnimationAsset>), gltf::Error>
where
    P: AsRef<Path>,
{
    let id = GltfIdentifier::Path(path.as_ref().to_path_buf());
    let (document, buffers, images) = gltf::import(path)?;
    let data = GltfData {
        id,
        buffers,
        images,
    };
    let mut loader = GltfDocumentLoader::new(&data);
    let result = loader.load(document);
    Ok(result)
}
