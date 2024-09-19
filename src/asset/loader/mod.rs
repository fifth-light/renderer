pub mod gltf;
pub mod obj;
pub mod texture;

fn pad_color_vec3_to_vec4(color: [f32; 3]) -> [f32; 4] {
    [color[0], color[1], color[2], 1.0]
}
