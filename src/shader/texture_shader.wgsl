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

struct TextureVertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
}

struct TextureVertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
}

@vertex
fn vs_main(model: TextureVertexInput) -> TextureVertexOutput {
    var out: TextureVertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * instance.transform * vec4f(model.position, 1.0);
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
