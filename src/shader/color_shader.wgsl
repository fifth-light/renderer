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

struct ColorVertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec4f,
}

struct ColorVertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec4f
}

@vertex
fn vs_main(
    model: ColorVertexInput
) -> ColorVertexOutput {
    var out: ColorVertexOutput;
    out.color = model.color;
    out.clip_position = camera.view_proj * instance.transform * vec4f(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: ColorVertexOutput) -> @location(0) vec4f {
    return in.color;
}