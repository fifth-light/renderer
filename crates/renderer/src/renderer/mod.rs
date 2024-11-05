use std::{collections::HashMap, iter, time::Instant};

use animation::{AnimationGroupNode, AnimationState};
use camera::Camera;
use context::{GlobalContext, DEFAULT_LOCAL_CONTEXT};
use depth_texture::DepthTexture;
use glam::{Mat4, Vec3};
use image::GrayImage;
use log::warn;
use node::{
    group::GroupNode,
    light::{LightData, LightParam},
    RenderNode, RenderNodeItem,
};
use texture::{TextureItem, TextureTransform};
use uniform::{
    camera::CameraUniformBuffer,
    light::{GlobalLightParam, LightUniformBuffer},
    texture::TextureUniformBuffer,
    transform::InstanceUniformBuffer,
};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, Color, CommandEncoder,
    CommandEncoderDescriptor, Device, LoadOp, Operations, Queue, RenderPass,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    SamplerBindingType, ShaderStages, StoreOp, TextureSampleType, TextureView,
    TextureViewDimension,
};
use winit::dpi::PhysicalSize;

pub mod animation;
pub mod camera;
pub mod context;
mod depth_texture;
pub mod index;
pub mod loader;
pub mod node;
pub mod pipeline;
mod tangent;
pub mod texture;
pub mod uniform;
pub mod vertex;

pub use depth_texture::DEPTH_TEXTURE_FORMAT;

pub enum RenderBindGroups<'a> {
    Color,
    Texture { texture: &'a BindGroup },
}

pub struct OngoingRenderState<'a> {
    pub encoder: CommandEncoder,
    pub render_pass: RenderPass<'static>,
    instance_bind_group: &'a BindGroup,
    joint_bind_group: Option<&'a BindGroup>,
    empty_texture_bind_group: &'a BindGroup,
}

#[derive(Debug)]
pub struct RenderTarget<'a> {
    texture_view: &'a TextureView,
    background_color: Vec3,
}

impl<'a> OngoingRenderState<'a> {
    pub fn new(
        device: &Device,
        texture_view: &'a TextureView,
        renderer_state: &'a RendererState,
    ) -> Self {
        Self::new_with_target(
            device,
            Some(RenderTarget {
                texture_view,
                background_color: *renderer_state.background_color(),
            }),
            Some(&renderer_state.depth_texture),
            renderer_state,
        )
    }

    fn new_with_target(
        device: &Device,
        render_target: Option<RenderTarget<'a>>,
        depth_texture: Option<&DepthTexture>,
        renderer_state: &'a RendererState,
    ) -> Self {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        let mut render_pass = encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[render_target.map(|target| RenderPassColorAttachment {
                    view: target.texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: target.background_color.x as f64,
                            g: target.background_color.y as f64,
                            b: target.background_color.z as f64,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_texture.map(|depth_texture| {
                    RenderPassDepthStencilAttachment {
                        view: depth_texture.texture_view(),
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                ..Default::default()
            })
            .forget_lifetime();

        let default_instance_bind_group = renderer_state.global_defaults.instance_bind_group();
        render_pass.set_bind_group(0, &renderer_state.global_bind_group, &[]);
        render_pass.set_bind_group(1, default_instance_bind_group, &[]);

        Self {
            encoder,
            render_pass,
            instance_bind_group: default_instance_bind_group,
            joint_bind_group: None,
            empty_texture_bind_group: &renderer_state.global_defaults.empty_texture_group,
        }
    }

    pub fn set_instance(&mut self, bind_group: &'a BindGroup) -> &'a BindGroup {
        let orig_bind_group = self.instance_bind_group;
        self.instance_bind_group = bind_group;
        self.render_pass.set_bind_group(1, bind_group, &[]);
        orig_bind_group
    }

    pub fn set_joint(&mut self, bind_group: Option<&'a BindGroup>) {
        self.joint_bind_group = bind_group;
    }

    pub fn is_joint_bound(&self) -> bool {
        self.joint_bind_group.is_some()
    }

    pub fn bind_groups(&mut self, groups: RenderBindGroups) {
        match (groups, self.joint_bind_group) {
            (RenderBindGroups::Color, None) => (),
            (RenderBindGroups::Color, Some(joint_bind_group)) => {
                self.render_pass
                    .set_bind_group(2, self.empty_texture_bind_group, &[]);
                self.render_pass.set_bind_group(3, joint_bind_group, &[]);
            }
            (RenderBindGroups::Texture { texture }, None) => {
                self.render_pass.set_bind_group(2, texture, &[]);
            }
            (RenderBindGroups::Texture { texture }, Some(joint_bind_group)) => {
                self.render_pass.set_bind_group(2, texture, &[]);
                self.render_pass.set_bind_group(3, joint_bind_group, &[]);
            }
        }
    }

    pub fn finish(self, queue: &Queue) {
        drop(self.render_pass);
        queue.submit(iter::once(self.encoder.finish()));
    }
}

pub struct RendererBindGroupLayout {
    global_uniform_layout: BindGroupLayout,
    instance_uniform_layout: BindGroupLayout,
    texture_layout: BindGroupLayout,
    joint_layout: BindGroupLayout,
}

impl RendererBindGroupLayout {
    pub fn new(device: &Device) -> Self {
        let texture_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Texture Bind Group Layout"),
        });
        let global_uniform_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Global Uniform Bind Group Layout"),
        });
        let instance_uniform_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Instance Uniform Bind Group Layout"),
        });
        let joint_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Joint Uniform Bind Group Layout"),
        });
        Self {
            global_uniform_layout,
            instance_uniform_layout,
            texture_layout,
            joint_layout,
        }
    }

    pub fn global_uniform_layout(&self) -> &BindGroupLayout {
        &self.global_uniform_layout
    }

    pub fn instance_uniform_layout(&self) -> &BindGroupLayout {
        &self.instance_uniform_layout
    }

    pub fn texture_bind_layout(&self) -> &BindGroupLayout {
        &self.texture_layout
    }

    pub fn joint_layout(&self) -> &BindGroupLayout {
        &self.joint_layout
    }
}

pub struct RendererGlobalDefaults {
    texture_transform_uniform: TextureUniformBuffer,
    instance_bind_group: BindGroup,
    empty_texture_group: BindGroup,
}

impl RendererGlobalDefaults {
    pub fn new(
        device: &Device,
        queue: &Queue,
        bind_group_layout: &RendererBindGroupLayout,
    ) -> Self {
        let default_instance_buffer = InstanceUniformBuffer::new(device, Mat4::IDENTITY);
        let instance_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout.instance_uniform_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: default_instance_buffer.buffer().as_entire_binding(),
            }],
            label: Some("Default Instance Uniform Bind Group"),
        });

        let texture_transform_uniform =
            TextureUniformBuffer::new(device, TextureTransform::default());

        let empty_texture = TextureItem::empty(device, queue);
        let empty_texture_group = empty_texture.create_bind_group(
            device,
            &bind_group_layout.texture_layout,
            &texture_transform_uniform,
        );

        Self {
            texture_transform_uniform,
            instance_bind_group,
            empty_texture_group,
        }
    }

    pub fn texture_transform_uniform(&self) -> &TextureUniformBuffer {
        &self.texture_transform_uniform
    }

    pub fn instance_bind_group(&self) -> &BindGroup {
        &self.instance_bind_group
    }

    pub fn empty_texture_group(&self) -> &BindGroup {
        &self.empty_texture_group
    }
}

pub struct RendererState {
    view_aspect: f32,
    background_color: Vec3,

    free_camera: Camera,
    enabled_camera: Option<usize>,
    enabled_camera_data: Option<Camera>,
    camera_updated: bool,
    flashlight: Option<LightParam>,

    depth_texture: DepthTexture,

    global_bind_group: BindGroup,
    global_defaults: RendererGlobalDefaults,
    bind_group_layout: RendererBindGroupLayout,
    camera_uniform: CameraUniformBuffer,
    light_uniform: LightUniformBuffer,
}

impl RendererState {
    pub fn new(device: &Device, queue: &Queue, size: PhysicalSize<u32>) -> Self {
        let view_aspect = size.width as f32 / size.height as f32;
        let camera = Camera::default();
        let camera_buffer = CameraUniformBuffer::new(device, &camera, view_aspect);
        let light_uniform = LightUniformBuffer::new(device, vec![], GlobalLightParam::default());

        let depth_texture = DepthTexture::new(device, (size.width, size.height));

        let bind_group_layout = RendererBindGroupLayout::new(device);
        let global_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout.global_uniform_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.buffer().as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: light_uniform.buffer().as_entire_binding(),
                },
            ],
            label: Some("Global Uniform Bind Group"),
        });

        let global_defaults = RendererGlobalDefaults::new(device, queue, &bind_group_layout);

        RendererState {
            camera_uniform: camera_buffer,
            bind_group_layout,
            depth_texture,
            view_aspect,
            free_camera: camera,
            camera_updated: false,
            enabled_camera: None,
            enabled_camera_data: None,
            global_bind_group,
            light_uniform,
            background_color: Vec3::new(0.8, 0.8, 1.0),
            flashlight: None,
            global_defaults,
        }
    }

    pub fn bind_group_layout(&self) -> &RendererBindGroupLayout {
        &self.bind_group_layout
    }

    pub fn global_defaults(&self) -> &RendererGlobalDefaults {
        &self.global_defaults
    }

    pub fn update_camera(&mut self, func: impl FnOnce(&mut Camera)) {
        func(&mut self.free_camera);
        self.camera_updated = true;
    }

    pub fn set_enabled_camera(&mut self, camera_id: Option<usize>) {
        self.enabled_camera = camera_id;
        self.enabled_camera_data = None;
    }

    pub fn enabled_camera(&self) -> Option<usize> {
        self.enabled_camera
    }

    pub fn set_enabled_camera_data(&mut self, camera: Camera) {
        self.enabled_camera_data = Some(camera);
    }

    pub fn set_global_light_param(&mut self, param: GlobalLightParam) {
        self.light_uniform.set_param(param);
    }

    pub fn global_light_param(&self) -> &GlobalLightParam {
        self.light_uniform.param()
    }

    pub fn set_background_color(&mut self, color: Vec3) {
        self.background_color = color;
    }

    pub fn background_color(&self) -> &Vec3 {
        &self.background_color
    }

    pub fn set_flashlight(&mut self, flashlight: Option<LightParam>) {
        self.flashlight = flashlight;
    }

    pub fn flashlight(&self) -> Option<&LightParam> {
        self.flashlight.as_ref()
    }

    pub fn resize(&mut self, device: &Device, size: PhysicalSize<u32>) {
        self.view_aspect = size.width as f32 / size.height as f32;
        self.free_camera
            .update_uniform(&mut self.camera_uniform, self.view_aspect);

        self.depth_texture = DepthTexture::new(device, (size.width, size.height));
    }

    pub fn set_lights(&mut self, lights: Vec<LightData>) {
        let mut lights = lights;
        if let Some(LightParam::Directional {
            color,
            constant,
            linear,
            quadratic,
            range_inner,
            range_outer,
        }) = self.flashlight
        {
            lights.push(LightData::Directional {
                position: self.free_camera.view.eye,
                color,
                direction: self.free_camera.view.front(),
                constant,
                linear,
                quadratic,
                range_inner,
                range_outer,
            });
        }
        self.light_uniform.items = lights;
    }

    fn prepare(&mut self, queue: &Queue) {
        if self.camera_updated {
            self.camera_updated = false;

            if self.enabled_camera.is_some() {
                if let Some(camera) = &mut self.enabled_camera_data {
                    camera.update_uniform(&mut self.camera_uniform, self.view_aspect);
                    self.camera_uniform.update(queue);
                } else {
                    warn!("There is a enabled camera, but no camera data is set in state.");
                    warn!("Is the camera in the node tree?");
                }
            } else {
                self.free_camera.update_aspect(self.view_aspect);
                self.free_camera
                    .update_uniform(&mut self.camera_uniform, self.view_aspect);
                self.camera_uniform.update(queue);
            }
        }
    }

    fn render<'a>(&'a self, ongoing_state: &mut OngoingRenderState<'a>, root_node: &'a GroupNode) {
        root_node.draw(self, ongoing_state);
    }

    pub fn dump_depth(&self, device: &Device, queue: &Queue) -> GrayImage {
        self.depth_texture.dump_depth(device, queue)
    }
}

pub struct Renderer {
    root_node: GroupNode,
    animation_groups: HashMap<usize, AnimationGroupNode>,
    pub state: RendererState,
}

impl Renderer {
    pub fn new(device: &Device, queue: &Queue, size: PhysicalSize<u32>) -> Self {
        let state = RendererState::new(device, queue, size);
        Self {
            root_node: GroupNode::new(Some("Root Node".to_string())),
            animation_groups: HashMap::new(),
            state,
        }
    }

    pub fn add_node(&mut self, node: RenderNodeItem) {
        self.root_node.push(node);
    }

    pub fn root_node(&self) -> &GroupNode {
        &self.root_node
    }

    pub fn add_animation_group(&mut self, group: AnimationGroupNode) {
        self.animation_groups.insert(group.id(), group);
    }

    pub fn set_animation_state(&mut self, id: usize, state: AnimationState) -> bool {
        let node = if let Some(node) = self.animation_groups.get_mut(&id) {
            node
        } else {
            return false;
        };
        node.set_state(state);
        true
    }

    pub fn animation_groups(&self) -> &HashMap<usize, AnimationGroupNode> {
        &self.animation_groups
    }

    pub fn prepare(&mut self, device: &Device, queue: &Queue, time: &Instant) {
        for animate_group in &mut self.animation_groups.values_mut() {
            animate_group.update(&mut self.root_node, time);
        }

        let mut global_context = GlobalContext::default();
        self.root_node
            .update(&DEFAULT_LOCAL_CONTEXT, &mut global_context, false);
        let light = global_context.finish();
        self.state.set_lights(light);
        self.state.light_uniform.update(queue);

        self.state.prepare(queue);
        self.root_node.prepare(device, queue, &mut self.state);
    }

    pub fn render<'a>(&'a self, ongoing_state: &mut OngoingRenderState<'a>) {
        self.state.render(ongoing_state, &self.root_node);
    }
}
