use glam::Mat4;

/// GLTF and VRM loader with `gltf` crate.
#[cfg(feature = "gltf")]
pub mod gltf;

/// OBJ loader with `tobj` crate.
#[cfg(feature = "obj")]
pub mod obj;

#[cfg(feature = "pmx")]
/// PMX 2.0 loader.
pub mod pmx;

pub(crate) mod texture;

#[inline]
fn pad_color_vec3_to_vec4(color: [f32; 3]) -> [f32; 4] {
    [color[0], color[1], color[2], 1.0]
}

#[inline]
fn pad_vec3_to_vec4(data: &[[f32; 3]], pad: f32) -> Vec<[f32; 4]> {
    data.iter().map(|[x, y, z]| [*x, *y, *z, pad]).collect()
}

#[inline]
fn clip_vec4_to_vec3(data: &[[f32; 4]]) -> Vec<[f32; 3]> {
    data.iter().map(|[x, y, z, _w]| [*x, *y, *z]).collect()
}

#[inline]
fn chunk_vec3<T: Copy>(data: &[T]) -> Vec<[T; 3]> {
    data.chunks_exact(3)
        .map(|item| item.try_into().unwrap())
        .collect()
}

#[inline]
fn chunk_vec4<T: Copy>(data: &[T]) -> Vec<[T; 4]> {
    data.chunks_exact(4)
        .map(|item| item.try_into().unwrap())
        .collect()
}

#[inline]
fn chunk_and_clamp_vec3_to_vec4_f32(data: &[f32]) -> Vec<[f32; 4]> {
    data.chunks_exact(3)
        .map(|item| {
            let array: [f32; 3] = item.try_into().unwrap();
            array.map(|num| num.clamp(0.0, 1.0));
            pad_color_vec3_to_vec4(array)
        })
        .collect()
}

#[inline]
fn chunk_and_clamp_vec4_f32(data: &[f32]) -> Vec<[f32; 4]> {
    data.chunks_exact(4)
        .map(|item| {
            let array: [f32; 4] = item.try_into().unwrap();
            array.map(|num| num.clamp(0.0, 1.0))
        })
        .collect()
}

#[inline]
fn chunk_mat4(data: &[f32]) -> Vec<Mat4> {
    data.chunks_exact(16)
        .map(|item| {
            let array = item.try_into().unwrap();
            Mat4::from_cols_array(&array)
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct AssetLoadParams {
    pub disable_unlit: bool,
    pub bundle_model_name: String,
    pub bundle_model_extension: bool,
}

impl Default for AssetLoadParams {
    fn default() -> Self {
        Self {
            disable_unlit: false,
            bundle_model_name: String::from("model"),
            bundle_model_extension: true,
        }
    }
}

impl AssetLoadParams {
    pub(crate) fn bundle_model_filename(&self, extension: &str) -> String {
        if self.bundle_model_extension {
            format!("{}.{}", self.bundle_model_name, extension)
        } else {
            self.bundle_model_name.clone()
        }
    }
}
