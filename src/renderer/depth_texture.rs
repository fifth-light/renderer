use wgpu::{
    Device, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
};

pub const DEPTH_TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub struct DepthTexture {
    texture_view: TextureView,
}

impl DepthTexture {
    pub fn new(device: &Device, size: (u32, u32)) -> Self {
        let size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        };
        let desc = TextureDescriptor {
            label: Some("Depth texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEPTH_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture_view }
    }

    pub fn texture_view(&self) -> &TextureView {
        &self.texture_view
    }

    pub fn format(&self) -> TextureFormat {
        DEPTH_TEXTURE_FORMAT
    }
}
