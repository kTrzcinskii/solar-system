// Vertex shader

struct CameraUniform {
    view_projection_matrix: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct LightUniform {
    position: vec3<f32>,
    color: vec3<f32>,
}

@group(2) @binding(0)
var<uniform> light: LightUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
    @location(12) texture_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) texture_index: u32,
    @location(2) world_normal: vec3<f32>,
    @location(3) world_position: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.texture_index = instance.texture_index;
    out.world_normal = normal_matrix * model.normal;
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_projection_matrix * world_position;
    return out;
}

// Fragment shader

const EARTH_INDEX: u32 = 2;
const EARTH_NIGHT_INDEX: u32 = 8;

const AMBIENT_STRENGHT: f32 = 0.02;
const AMBIENT_STRENGHT_NIGHT: f32 = 0.6;

@group(0) @binding(0)
var textures: texture_2d_array<f32>;
@group(0) @binding(1)
var textures_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(light.position - in.world_position);

    let dot_product = dot(in.world_normal, light_dir);
    
    let faces_sun = dot_product >= 0.0;
    let texture_index_earth = select(EARTH_NIGHT_INDEX, EARTH_INDEX, faces_sun);
    let is_earth = in.texture_index == EARTH_INDEX;
    let texture_index = select(in.texture_index, texture_index_earth, is_earth);
    let object_color: vec4<f32> = textureSample(textures, textures_sampler, in.tex_coords, texture_index);

    let earth_at_night = is_earth && !faces_sun;
    let ambient_strength = select(AMBIENT_STRENGHT, AMBIENT_STRENGHT_NIGHT, earth_at_night);
    let ambient_color = light.color * ambient_strength;

    let diffuse_strength = max(dot_product, 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let result = (ambient_color + diffuse_color) * object_color.xyz;

    return vec4<f32>(result, object_color.a);
}
