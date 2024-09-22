struct CameraUniform {
    view_proj: mat4x4f,
    view_pos: vec3f,
    aspect: f32,
}
const MAX_POINT_LIGHTS = 128;
const MAX_PARALLEL_LIGHTS = 16;
struct PointLightData {
    position: vec3f, // 12 pad to 16
    color: vec3f, //    28
    constant: f32, //   32
    linear: f32, //     36
    quadratic: f32, //  40
//                      pad to 48
}
struct ParallelLightData {
    direction: vec3f, // 12 pad to 16
    color: vec3f, //     28
    strength: f32, //    32
//                       pad to 32
}
struct LightUniform {
    point_length: u32, // 4
    parallel_length: u32, // 4 pad to 12
    point: array<PointLightData, MAX_POINT_LIGHTS>, // 6160
    parallel: array<ParallelLightData, MAX_PARALLEL_LIGHTS>, // 6672
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;
@group(0) @binding(1)
var<uniform> light: LightUniform;

struct InstanceUniform {
    transform: mat4x4f,
    normal: mat3x3f,
}
@group(1) @binding(0)
var<uniform> instance: InstanceUniform;

@group(2) @binding(0)
var diffuse_texture: texture_2d<f32>;
@group(2) @binding(1)
var diffuse_sampler: sampler;

// 64K of memory
const MAX_JOINTS = 512;
struct JointItem {
    transform: mat4x4f,
    normal: mat3x3f,
}
@group(3) @binding(0)
var<uniform> joints: array<JointItem, MAX_JOINTS>;

struct ColorVertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec4f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec3f,
}

struct ColorSkinVertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec4f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec3f,
    @location(4) joint_index: vec4u,
    @location(5) joint_weight: vec4f,
}

struct TextureVertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec3f,
}

struct TextureSkinVertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec3f,
    @location(4) joint_index: vec4u,
    @location(5) joint_weight: vec4f,
}

struct ColorVertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) position: vec3f,
    @location(1) color: vec4f,
    @location(2) normal: vec3f,
}

struct TextureVertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
    @location(2) normal: vec3f,
}

fn compute_skin_transform_matrix(joint_index: vec4u, joint_weight: vec4f) -> mat4x4f {
    let transform_maxtix_1: mat4x4f = joint_weight.x * joints[joint_index.x].transform;
    let transform_maxtix_2: mat4x4f = joint_weight.y * joints[joint_index.y].transform;
    let transform_maxtix_3: mat4x4f = joint_weight.z * joints[joint_index.z].transform;
    let transform_maxtix_4: mat4x4f = joint_weight.w * joints[joint_index.w].transform;
    return transform_maxtix_1 + transform_maxtix_2 + transform_maxtix_3 + transform_maxtix_4;
}

fn compute_skin_normal_matrix(joint_index: vec4u, joint_weight: vec4f) -> mat3x3f {
    let normal_maxtix_1: mat3x3f = joint_weight.x * joints[joint_index.x].normal;
    let normal_maxtix_2: mat3x3f = joint_weight.y * joints[joint_index.y].normal;
    let normal_maxtix_3: mat3x3f = joint_weight.z * joints[joint_index.z].normal;
    let normal_maxtix_4: mat3x3f = joint_weight.w * joints[joint_index.w].normal;
    return normal_maxtix_1 + normal_maxtix_2 + normal_maxtix_3 + normal_maxtix_4;
}

@vertex
fn color_vs_main(model: ColorVertexInput) -> ColorVertexOutput {
    var out: ColorVertexOutput;
    out.color = model.color;
    out.position = model.position;
    out.clip_position = camera.view_proj * instance.transform * vec4f(model.position, 1.0);
    out.normal = instance.normal * model.normal;
    return out;
}

@vertex
fn color_skin_vs_main(model: ColorSkinVertexInput) -> ColorVertexOutput {
    var out: ColorVertexOutput;
    out.color = model.color;
    out.position = model.position;
    let skin_matrix = compute_skin_transform_matrix(model.joint_index, model.joint_weight);
    out.clip_position = camera.view_proj * skin_matrix * vec4f(model.position, 1.0);
    let normal_matrix = compute_skin_normal_matrix(model.joint_index, model.joint_weight);
    out.normal = normal_matrix * model.normal;
    return out;
}

@vertex
fn texture_vs_main(model: TextureVertexInput) -> TextureVertexOutput {
    var out: TextureVertexOutput;
    out.tex_coords = model.tex_coords;
    out.position = model.position;
    out.clip_position = camera.view_proj * instance.transform * vec4f(model.position, 1.0);
    out.normal = instance.normal * model.normal;
    return out;
}

@vertex
fn texture_skin_vs_main(model: TextureSkinVertexInput) -> TextureVertexOutput {
    var out: TextureVertexOutput;
    out.tex_coords = model.tex_coords;
    out.position = model.position;
    let skin_matrix = compute_skin_transform_matrix(model.joint_index, model.joint_weight);
    out.clip_position = camera.view_proj * skin_matrix * vec4f(model.position, 1.0);
    let normal_matrix = compute_skin_normal_matrix(model.joint_index, model.joint_weight);
    out.normal = normal_matrix * model.normal;
    return out;
}

const OUTLINE_SIZE = 0.003;

@vertex
fn color_outline_vs_main(model: ColorVertexInput) -> ColorVertexOutput {
    var out: ColorVertexOutput;
    out.color = model.color;

    let normal = normalize(instance.normal * model.normal);
    let tangent = normalize(instance.normal * model.tangent);
    let ndc_tangent = normalize((camera.view_proj * vec4f(tangent, 0.0)).xyz);
    let outline_size = vec3f(OUTLINE_SIZE / camera.aspect, OUTLINE_SIZE, 0.0);
    let outline_position = outline_size * ndc_tangent;

    out.position = model.position;
    out.normal = normal;
    out.clip_position = camera.view_proj * instance.transform * vec4f(model.position, 1.0) + vec4f(outline_position, 0.0);

    return out;
}

@vertex
fn color_outline_skin_vs_main(model: ColorSkinVertexInput) -> ColorVertexOutput {
    var out: ColorVertexOutput;
    out.color = model.color;

    let normal_matrix = compute_skin_normal_matrix(model.joint_index, model.joint_weight);
    let normal = normalize(normal_matrix * model.normal);
    let tangent = normalize(normal_matrix * model.tangent);
    let ndc_tangent = normalize((camera.view_proj * vec4f(tangent, 0.0)).xyz);
    let outline_size = vec3f(OUTLINE_SIZE / camera.aspect, OUTLINE_SIZE, 0.0);
    let outline_position = outline_size * ndc_tangent;

    let skin_matrix = compute_skin_transform_matrix(model.joint_index, model.joint_weight);
    let position = skin_matrix * vec4f(model.position, 1.0);

    out.position = position.xyz;
    out.normal = normal;
    out.clip_position = camera.view_proj * position + vec4f(outline_position, 0.0);

    return out;
}

@vertex
fn texture_outline_vs_main(model: TextureVertexInput) -> TextureVertexOutput {
    var out: TextureVertexOutput;
    out.tex_coords = model.tex_coords;

    let normal = normalize(instance.normal * model.normal);
    let tangent = normalize(instance.normal * model.tangent);
    let ndc_tangent = normalize((camera.view_proj * vec4f(tangent, 0.0)).xyz);
    let outline_size = vec3f(OUTLINE_SIZE / camera.aspect, OUTLINE_SIZE, 0.0);
    let outline_position = outline_size * ndc_tangent;

    out.position = model.position;
    out.normal = normal;
    out.clip_position = camera.view_proj * instance.transform * vec4f(model.position, 1.0) + vec4f(outline_position, 0.0);

    return out;
}

@vertex
fn texture_outline_skin_vs_main(model: TextureSkinVertexInput) -> TextureVertexOutput {
    var out: TextureVertexOutput;
    out.tex_coords = model.tex_coords;

    let normal_matrix = compute_skin_normal_matrix(model.joint_index, model.joint_weight);
    let normal = normalize(normal_matrix * model.normal);
    let tangent = normalize(normal_matrix * model.tangent);
    let ndc_tangent = normalize((camera.view_proj * vec4f(tangent, 0.0)).xyz);
    let outline_size = vec3f(OUTLINE_SIZE / camera.aspect, OUTLINE_SIZE, 0.0);
    let outline_position = outline_size * ndc_tangent;

    let skin_matrix = compute_skin_transform_matrix(model.joint_index, model.joint_weight);
    let position = skin_matrix * vec4f(model.position, 1.0);

    out.position = position.xyz;
    out.normal = normal;
    out.clip_position = camera.view_proj * position + vec4f(outline_position, 0.0);

    return out;
}

@fragment
fn light_fs_main(in: ColorVertexOutput) -> @location(0) vec4f {
    return in.color;
}

const START_STRENGTH = 0.50;
const STOP_STRENGTH = 0.55;
const MAX_STRENGTH = 0.7;
fn strength_map(strength: f32) -> f32 {
    return smoothstep(START_STRENGTH, STOP_STRENGTH, strength) * MAX_STRENGTH;
}

fn point_light_process(in_color: vec3f, normal: vec3f, position: vec3f, point: PointLightData) -> vec3f {
    let light_pos = point.position;
    let light_color = point.color;
    let light_direction = normalize(light_pos - position);

    let diffuse_strength: f32 = strength_map(max(dot(normal, light_direction), 0.0));
    let diffuse = diffuse_strength * light_color;

    let distance = length(light_pos - position);
    let attenuation = 1.0 / (point.constant + point.linear * distance + point.quadratic * (distance * distance));

    return diffuse * in_color * attenuation;
}

fn parallel_light_process(in_color: vec3f, normal: vec3f, position: vec3f, parallel: ParallelLightData) -> vec3f {
    let light_direction = normalize(parallel.direction);
    let light_color = parallel.color;

    let diffuse_strength: f32 = strength_map(max(dot(normal, light_direction), 0.0));
    let diffuse = diffuse_strength * light_color;

    return diffuse * in_color * parallel.strength;
}

const AMBIENT_STRENGTH = 0.1;
fn light_process(in_color: vec4f, normal: vec3f, position: vec3f) -> vec4f {
    let alpha = in_color.a;
    let color = in_color.rgb;

    var result: vec3f = vec3f(0.0, 0.0, 0.0);

    // Ambient
    let ambient = AMBIENT_STRENGTH * color;
    result += ambient;

    // Point light
    for (var i: u32 = 0; i < light.point_length; i++) {
        let point = light.point[i];
        result += point_light_process(color, normal, position, point);
    }

    // Parallel light
    for (var i: u32 = 0; i < light.parallel_length; i++) {
        let parallel = light.parallel[i];
        result += parallel_light_process(color, normal, position, parallel);
    }

    return vec4f(result, alpha);
}

@fragment
fn color_fs_main(in: ColorVertexOutput) -> @location(0) vec4f {
    return light_process(in.color, in.normal, in.position);
}

@fragment
fn texture_fs_main(in: TextureVertexOutput) -> @location(0) vec4f {
    let color = textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
    if color.a == 0.0 {
        discard;
    }
    return light_process(color, in.normal, in.position);
}

fn outline_color_process(color: vec3f) -> vec3f {
    return color * 0.05;
}

@fragment
fn color_outline_fs_main(in: ColorVertexOutput) -> @location(0) vec4f {
    return vec4(outline_color_process(in.color.rgb), in.color.a);
}

@fragment
fn texture_outline_fs_main(in: TextureVertexOutput) -> @location(0) vec4f {
    let in_color = textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
    if in_color.a == 0.0 {
        discard;
    }
    return vec4(outline_color_process(in_color.rgb), in_color.a);
}
