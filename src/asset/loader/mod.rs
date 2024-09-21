use glam::Mat4;

pub mod gltf;
pub mod obj;
pub mod texture;

fn pad_color_vec3_to_vec4(color: [f32; 3]) -> [f32; 4] {
    [color[0], color[1], color[2], 1.0]
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
