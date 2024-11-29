use std::{collections::HashMap, fmt::Debug, sync::Arc};

use wgpu::{
    include_wgsl, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState,
    ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, Face, FragmentState,
    FrontFace, MultisampleState, PipelineLayout, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    StencilState, TextureFormat, VertexState,
};

use super::{
    buffer::vertex::{ColorSkinVertex, ColorVertex, TextureSkinVertex, TextureVertex, Vertex},
    RendererBindGroupLayout, DEPTH_TEXTURE_FORMAT,
};

#[derive(Debug)]
pub struct RenderPipelineItem {
    render_pipeline: RenderPipeline,
    #[allow(unused)]
    #[cfg(debug_assertions)]
    label: Option<String>,
    shader_type: ShaderType,
}

#[derive(Debug)]
pub struct RenderPipelineItemDescriptor<'a> {
    pub label: Option<&'a str>,
    pub shader_module: &'a ShaderModule,
    pub vertex_entry_name: &'a str,
    pub fragment_entry_name: &'a str,
    pub target_texture_format: TextureFormat,
    pub primitive_topology: PrimitiveTopology,
    pub shader_type: ShaderType,
    pub alpha_mode: ShaderAlphaMode,
    pub outline: bool,
}

impl RenderPipelineItem {
    fn create_pipeline_layout(
        device: &Device,
        bind_group_layouts: &RendererBindGroupLayout,
        descriptor: &RenderPipelineItemDescriptor,
    ) -> PipelineLayout {
        let bind_group_layouts: &[_] = match descriptor.shader_type {
            ShaderType::Light | ShaderType::Color => &[
                &bind_group_layouts.global_uniform_layout,
                &bind_group_layouts.instance_uniform_layout,
            ],
            ShaderType::Texture => &[
                &bind_group_layouts.global_uniform_layout,
                &bind_group_layouts.instance_uniform_layout,
                &bind_group_layouts.texture_layout,
            ],
            ShaderType::ColorSkin => &[
                &bind_group_layouts.global_uniform_layout,
                &bind_group_layouts.instance_uniform_layout,
                &bind_group_layouts.texture_layout,
                &bind_group_layouts.joint_layout,
            ],
            ShaderType::TextureSkin => &[
                &bind_group_layouts.global_uniform_layout,
                &bind_group_layouts.instance_uniform_layout,
                &bind_group_layouts.texture_layout,
                &bind_group_layouts.joint_layout,
            ],
        };
        device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: descriptor.label,
            bind_group_layouts,
            push_constant_ranges: &[],
        })
    }

    fn create_render_pipeline(
        device: &Device,
        pipeline_layout: &PipelineLayout,
        descriptor: &RenderPipelineItemDescriptor,
    ) -> RenderPipeline {
        let vertex_descriptor = match descriptor.shader_type {
            ShaderType::Light | ShaderType::Color => ColorVertex::desc(),
            ShaderType::Texture => TextureVertex::desc(),
            ShaderType::ColorSkin => ColorSkinVertex::desc(),
            ShaderType::TextureSkin => TextureSkinVertex::desc(),
        };
        // TODO: now the shader always see Opaque mode as Mask mode
        let blend_state = match descriptor.alpha_mode {
            ShaderAlphaMode::Opaque => BlendState {
                color: BlendComponent::REPLACE,
                alpha: BlendComponent::REPLACE,
            },
            ShaderAlphaMode::Mask => BlendState {
                color: BlendComponent::REPLACE,
                alpha: BlendComponent::REPLACE,
            },
            ShaderAlphaMode::Blend => BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::DstAlpha,
                    operation: BlendOperation::Max,
                },
            },
        };
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: descriptor.label,
            layout: Some(pipeline_layout),
            vertex: VertexState {
                module: descriptor.shader_module,
                entry_point: descriptor.vertex_entry_name,
                compilation_options: Default::default(),
                buffers: &[vertex_descriptor],
            },
            fragment: Some(FragmentState {
                module: descriptor.shader_module,
                entry_point: descriptor.fragment_entry_name,
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: descriptor.target_texture_format,
                    blend: Some(blend_state),
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: PrimitiveState {
                topology: descriptor.primitive_topology,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(if descriptor.outline {
                    Face::Front
                } else {
                    Face::Back
                }),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: DEPTH_TEXTURE_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }

    pub fn new(
        device: &Device,
        bind_group_layouts: &RendererBindGroupLayout,
        descriptor: &RenderPipelineItemDescriptor,
    ) -> Self {
        let pipeline_layout = Self::create_pipeline_layout(device, bind_group_layouts, descriptor);
        let render_pipeline = Self::create_render_pipeline(device, &pipeline_layout, descriptor);
        Self {
            render_pipeline,
            #[cfg(debug_assertions)]
            label: descriptor.label.map(|label| label.to_string()),
            shader_type: descriptor.shader_type,
        }
    }

    pub fn shader_type(&self) -> ShaderType {
        self.shader_type
    }

    pub fn render_pipeline(&self) -> &RenderPipeline {
        &self.render_pipeline
    }
}

#[derive(Debug)]
pub struct Pipelines {
    shader_module: ShaderModule,
    target_texture_format: TextureFormat,
    items: HashMap<(PipelineIdentifier, bool), Arc<RenderPipelineItem>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineIdentifier {
    pub shader: ShaderType,
    pub primitive_topology: PrimitiveTopology,
    pub alpha_mode: ShaderAlphaMode,
    pub lit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Light,
    Color,
    Texture,
    ColorSkin,
    TextureSkin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShaderAlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

impl Pipelines {
    pub fn new(device: &Device, target_texture_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(include_wgsl!("../shader/shader.wgsl"));
        Self {
            shader_module,
            target_texture_format,
            items: HashMap::new(),
        }
    }

    fn new_pipeline(
        &mut self,
        device: &Device,
        bind_group_layouts: &RendererBindGroupLayout,
        identifier: PipelineIdentifier,
        outline: bool,
    ) -> RenderPipelineItem {
        #[rustfmt::skip]
        let (vertex_entry_name, fragment_entry_name) = match (identifier.shader, identifier.lit, outline) {
            (ShaderType::Light, _, _) => ("color_vs_main", "light_fs_main"),
            (ShaderType::Color, false, false) => ("color_vs_main", "color_fs_main"),
            (ShaderType::Color, true, false) => ("color_vs_main", "color_light_fs_main"),
            (ShaderType::Texture, false, false) => ("texture_vs_main", "texture_fs_main"),
            (ShaderType::Texture, true, false) => ("texture_vs_main", "texture_light_fs_main"),
            (ShaderType::ColorSkin, false, false) => ("color_skin_vs_main", "color_fs_main"),
            (ShaderType::ColorSkin, true, false) => ("color_skin_vs_main", "color_light_fs_main"),
            (ShaderType::TextureSkin, false, false) => ("texture_skin_vs_main", "texture_fs_main"),
            (ShaderType::TextureSkin, true, false) => ("texture_skin_vs_main", "texture_light_fs_main"),
            (ShaderType::Color, false, true) => ("color_outline_vs_main", "color_outline_fs_main"),
            (ShaderType::Color, true, true) => ("color_outline_vs_main", "color_light_outline_fs_main"),
            (ShaderType::Texture, false, true) => ("texture_outline_vs_main", "texture_outline_fs_main"),
            (ShaderType::Texture, true, true) => ("texture_outline_vs_main", "texture_light_outline_fs_main"),
            (ShaderType::ColorSkin, false, true) => ("color_outline_skin_vs_main", "color_outline_fs_main"),
            (ShaderType::ColorSkin, true, true) => ("color_outline_skin_vs_main", "color_light_outline_fs_main"),
            (ShaderType::TextureSkin, false, true) => ("texture_outline_skin_vs_main", "texture_outline_fs_main"),
            (ShaderType::TextureSkin, true, true) => ("texture_outline_skin_vs_main", "texture_light_outline_fs_main"),
        };
        RenderPipelineItem::new(
            device,
            bind_group_layouts,
            &RenderPipelineItemDescriptor {
                label: Some(&format!("{:?}", identifier)),
                shader_module: &self.shader_module,
                vertex_entry_name,
                fragment_entry_name,
                target_texture_format: self.target_texture_format,
                primitive_topology: identifier.primitive_topology,
                shader_type: identifier.shader,
                alpha_mode: identifier.alpha_mode,
                outline,
            },
        )
    }

    pub fn get(
        &mut self,
        device: &Device,
        bind_group_layouts: &RendererBindGroupLayout,
        identifier: PipelineIdentifier,
        outline: bool,
    ) -> Arc<RenderPipelineItem> {
        if let Some(item) = self.items.get(&(identifier, outline)) {
            return item.clone();
        }
        let item = Arc::new(self.new_pipeline(device, bind_group_layouts, identifier, outline));
        self.items.insert((identifier, outline), item.clone());
        item
    }
}
