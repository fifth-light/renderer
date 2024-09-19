struct CameraUniform {
    view_proj: mat4x4f,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceUniform {
    transform: mat4x4f,
}
@group(1) @binding(0)
var<uniform> instance: InstanceUniform;

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(1)
var s_diffuse: sampler;

// 64K of memory
const MAX_JOINTS = 1024;
@group(3) @binding(0)
var<uniform> joints: array<mat4x4f, MAX_JOINTS>;

struct TextureSkinVertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
    // It is vec4<u16> actually, but there is no u16 in WGSL
    @location(2) joint_index: vec4<u32>,
    @location(3) joint_weight: vec4<f32>,
}

struct TextureVertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
}

@vertex
fn vs_main(model: TextureSkinVertexInput) -> TextureVertexOutput {
    var out: TextureVertexOutput;
    out.tex_coords = model.tex_coords;

    let skin_maxtix_1: mat4x4<f32> = model.joint_weight.x * joints[model.joint_index.x];
    let skin_maxtix_2: mat4x4<f32> = model.joint_weight.y * joints[model.joint_index.y];
    let skin_maxtix_3: mat4x4<f32> = model.joint_weight.z * joints[model.joint_index.z];
    let skin_maxtix_4: mat4x4<f32> = model.joint_weight.w * joints[model.joint_index.w];
    let skin_matrix = skin_maxtix_1 + skin_maxtix_2 + skin_maxtix_3 + skin_maxtix_4;

    out.clip_position = camera.view_proj * skin_matrix * vec4f(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: TextureVertexOutput) -> @location(0) vec4f {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if color.a == 0.0 {
        discard;
    }
    return color;
}
