use std::sync::Arc;

use crate::{
    index::AssetIndex,
    texture::{
        NormalTextureInfo, OcclusionTextureInfo, ShadingShiftTextureInfo, TextureAsset, TextureInfo,
    },
};

#[derive(Debug, Clone)]
pub enum PmxEnvironmentBlendMode {
    Disabled,
    Multiply,
    Additive,
    AdditionalVec4,
}

#[derive(Debug, Clone)]
pub enum PmxToonReference {
    Texture(Arc<TextureAsset>),
    Internal { index: u8 },
}

/// Define lighting parameters for the material.
#[derive(Debug, Clone)]
pub enum MaterialAssetData {
    /// The standard lighting model for GLTF.
    Pbr {
        base_color_factor: [f32; 4],
        base_color_texture: Option<TextureInfo>,
        metallic_factor: f32,
        roughness_factor: f32,
        metallic_roughness_texture: Option<TextureInfo>,
    },
    /// Materials in MTL file. Basic blinn-phone lighting model.
    BlinnPhong {
        ambient_color: [f32; 3],
        diffuse_color: [f32; 3],
        specular_color: [f32; 3],
        shininess: f32,
        dissolve: f32,
        optical_density: f32,
        ambient_texture: Option<TextureInfo>,
        diffuse_texture: Option<TextureInfo>,
        specular_texture: Option<TextureInfo>,
        shininess_texture: Option<TextureInfo>,
        dissolve_texture: Option<TextureInfo>,
    },
    /// GLTF KHR_materials_unlit. The simplest lighting model.
    Unlit {
        base_color_factor: [f32; 4],
        base_color_texture: Option<TextureInfo>,
    },
    /// Lighting model for VRM.
    MTone {
        base_color_factor: [f32; 4],
        base_color_texture: Option<TextureInfo>,
        transparent_with_z_write: bool,
        render_queue_offset_number: isize,
        shade_color_factor: [f32; 3],
        shade_multiply_texture: Option<TextureInfo>,
        shading_shift_factor: f32,
        shading_shift_texture: Option<ShadingShiftTextureInfo>,
        shading_toony_factor: f32,
        gi_equalization_factor: f32,
        matcap_factor: [f32; 3],
        matcap_texture: Option<TextureInfo>,
        parametric_rim_color_factor: [f32; 3],
        parametric_rim_fresnel_power_factor: f32,
        parametric_rim_lift_factor: f32,
        rim_multiply_texture: Option<TextureInfo>,
        rim_lighting_mix_factor: f32,
        outline_width_mode: OutlineWidthMode,
        outline_width_factor: f32,
        outline_width_multiply_texture: Option<TextureInfo>,
        outline_color_factor: [f32; 3],
        outline_lighting_mix_factor: f32,
    },
    /// Lighting model in MikuMikuDance.
    Pmx {
        no_cull: bool,
        ground_shadow: bool,
        draw_shadow: bool,
        receive_shadow: bool,
        has_edge: bool,
        ambient_color: [f32; 3],
        diffuse_color: [f32; 4],
        specular_color: [f32; 3],
        specular_strength: f32,
        edge_color: [f32; 4],
        edge_scale: f32,
        texture: Option<TextureInfo>,
        environment: Option<TextureInfo>,
        environment_blend_mode: PmxEnvironmentBlendMode,
        toon_reference: PmxToonReference,
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MaterialAlphaMode {
    #[default]
    Opaque,
    // Alpha cutoff
    Mask(f32),
    Blend,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum OutlineWidthMode {
    #[default]
    None,
    WorldCoordinates,
    ScreenCoordinates,
}

#[derive(Debug, Clone, Default)]
pub struct UvAnimation {
    pub mask_texture: Option<Arc<TextureAsset>>,
    pub scroll_x_speed_factor: f32,
    pub scroll_y_speed_factor: f32,
    pub rotation_speed_factor: f32,
}

#[derive(Debug, Clone)]
pub struct MaterialAsset {
    pub id: AssetIndex,
    pub name: Option<String>,
    pub data: MaterialAssetData,
    pub normal_texture: Option<NormalTextureInfo>,
    pub occlusion_texture: Option<OcclusionTextureInfo>,
    pub emissive_texture: Option<TextureInfo>,
    pub emissive_factor: [f32; 3],
    pub alpha_mode: MaterialAlphaMode,
    pub double_sided: bool,
    pub uv_animation: Option<UvAnimation>,
}
