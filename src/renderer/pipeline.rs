use std::{collections::HashMap, fmt::Debug, sync::Arc};

use wgpu::{
    include_wgsl, BlendComponent, BlendState, ColorTargetState, ColorWrites, CompareFunction,
    DepthBiasState, DepthStencilState, Device, Face, FragmentState, FrontFace, MultisampleState,
    PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, StencilState, TextureFormat,
    VertexState,
};

use super::{
    vertex::{ColorSkinVertex, ColorVertex, TextureSkinVertex, TextureVertex, Vertex},
    RendererState,
};

#[derive(Debug)]
pub struct RenderPipelineItem {
    render_pipeline: RenderPipeline,
    #[allow(unused)]
    #[cfg(debug_assertions)]
    label: Option<String>,
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
}

impl RenderPipelineItem {
    fn create_pipeline_layout(
        device: &Device,
        renderer_state: &RendererState,
        descriptor: &RenderPipelineItemDescriptor,
    ) -> PipelineLayout {
        let bind_group_layouts: &[_] = match descriptor.shader_type {
            ShaderType::Color => &[
                &renderer_state.global_uniform_layout,
                &renderer_state.instance_uniform_layout,
            ],
            ShaderType::Texture => &[
                &renderer_state.global_uniform_layout,
                &renderer_state.instance_uniform_layout,
                &renderer_state.texture_layout,
            ],
            ShaderType::ColorSkin => &[
                &renderer_state.global_uniform_layout,
                &renderer_state.instance_uniform_layout,
                &renderer_state.joint_layout,
            ],
            ShaderType::TextureSkin => &[
                &renderer_state.global_uniform_layout,
                &renderer_state.instance_uniform_layout,
                &renderer_state.texture_layout,
                &renderer_state.joint_layout,
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
        renderer_state: &RendererState,
        pipeline_layout: &PipelineLayout,
        descriptor: &RenderPipelineItemDescriptor,
    ) -> RenderPipeline {
        let vertex_descriptor = match descriptor.shader_type {
            ShaderType::Color => ColorVertex::desc(),
            ShaderType::Texture => TextureVertex::desc(),
            ShaderType::ColorSkin => ColorSkinVertex::desc(),
            ShaderType::TextureSkin => TextureSkinVertex::desc(),
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
                    blend: Some(BlendState {
                        color: BlendComponent::REPLACE,
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: PrimitiveState {
                topology: descriptor.primitive_topology,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: renderer_state.depth_texture.format(),
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
        renderer_state: &RendererState,
        descriptor: &RenderPipelineItemDescriptor,
    ) -> Self {
        let pipeline_layout = Self::create_pipeline_layout(device, renderer_state, descriptor);
        let render_pipeline =
            Self::create_render_pipeline(device, renderer_state, &pipeline_layout, descriptor);
        Self {
            render_pipeline,
            #[cfg(debug_assertions)]
            label: descriptor.label.map(|label| label.to_string()),
        }
    }

    pub fn render_pipeline(&self) -> &RenderPipeline {
        &self.render_pipeline
    }
}

pub struct Pipelines {
    color_shader: ShaderModule,
    texture_shader: ShaderModule,
    color_skin_shader: ShaderModule,
    texture_skin_shader: ShaderModule,
    target_texture_format: TextureFormat,
    items: HashMap<PipelineIdentifier, Arc<RenderPipelineItem>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineIdentifier {
    pub shader: ShaderType,
    pub primitive_topology: PrimitiveTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Color,
    Texture,
    ColorSkin,
    TextureSkin,
}

impl Pipelines {
    pub fn new(device: &Device, target_texture_format: TextureFormat) -> Self {
        let color_shader =
            device.create_shader_module(include_wgsl!("../shader/color_shader.wgsl"));
        let texture_shader =
            device.create_shader_module(include_wgsl!("../shader/texture_shader.wgsl"));
        let color_skin_shader =
            device.create_shader_module(include_wgsl!("../shader/color_skin_shader.wgsl"));
        let texture_skin_shader =
            device.create_shader_module(include_wgsl!("../shader/texture_skin_shader.wgsl"));
        Self {
            color_shader,
            texture_shader,
            color_skin_shader,
            texture_skin_shader,
            target_texture_format,
            items: HashMap::new(),
        }
    }

    fn new_pipeline(
        &mut self,
        device: &Device,
        renderer_state: &RendererState,
        identifier: PipelineIdentifier,
    ) -> RenderPipelineItem {
        let shader_module = match identifier.shader {
            ShaderType::Color => &self.color_shader,
            ShaderType::Texture => &self.texture_shader,
            ShaderType::ColorSkin => &self.color_skin_shader,
            ShaderType::TextureSkin => &self.texture_skin_shader,
        };
        RenderPipelineItem::new(
            device,
            renderer_state,
            &RenderPipelineItemDescriptor {
                label: Some(&format!("{:?}", identifier)),
                shader_module,
                vertex_entry_name: "vs_main",
                fragment_entry_name: "fs_main",
                target_texture_format: self.target_texture_format,
                primitive_topology: identifier.primitive_topology,
                shader_type: identifier.shader,
            },
        )
    }

    pub fn get(
        &mut self,
        device: &Device,
        renderer_state: &RendererState,
        identifier: PipelineIdentifier,
    ) -> Arc<RenderPipelineItem> {
        if let Some(item) = self.items.get(&identifier) {
            return item.clone();
        }
        let item = Arc::new(self.new_pipeline(device, renderer_state, identifier));
        self.items.insert(identifier, item.clone());
        item
    }
}
