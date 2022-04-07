// Vertex shader

struct Camera {
    view_pos: vec4<f32>;
    proj_matrix: mat4x4<f32>;
    view_matrix: mat4x4<f32>; //
};
[[group(1), binding(0)]]
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>;
    intensity: f32;
    color: vec3<f32>;
    spread: f32;
    specular_lobe_factor: f32;
};
[[group(2), binding(0)]]
var<uniform> light: Light;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
    [[location(3)]] tangent: vec3<f32>;
    [[location(4)]] bitangent: vec3<f32>;
};
struct InstanceInput {
    [[location(5)]] model_matrix_0: vec4<f32>;
    [[location(6)]] model_matrix_1: vec4<f32>;
    [[location(7)]] model_matrix_2: vec4<f32>;
    [[location(8)]] model_matrix_3: vec4<f32>;
    [[location(9)]] normal_matrix_0: vec3<f32>;
    [[location(10)]] normal_matrix_1: vec3<f32>;
    [[location(11)]] normal_matrix_2: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] view_position: vec4<f32>;
    [[location(3)]] view_normal: vec3<f32>;
    [[location(4)]] tangent_position: vec3<f32>;
    [[location(5)]] tangent_light_position: vec3<f32>;
    [[location(6)]] tangent_view_position: vec3<f32>;
};

[[stage(vertex)]]
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

    // Construct the tangent matrix
    let world_normal = normalize(normal_matrix * model.normal);
    let world_tangent = normalize(normal_matrix * model.tangent);
    let world_bitangent = normalize(normal_matrix * model.bitangent);
    let tangent_matrix = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal,
    ));

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.proj_matrix * camera.view_matrix * world_position;
    out.tex_coords = model.tex_coords;
    out.tangent_position = tangent_matrix * world_position.xyz;
    out.tangent_view_position = tangent_matrix * camera.view_pos.xyz;
    out.tangent_light_position = tangent_matrix * light.position;
    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;
[[group(0), binding(2)]]
var t_normal: texture_2d<f32>;
[[group(0), binding(3)]]
var s_normal: sampler;
[[group(0), binding(4)]]
var t_specular: texture_2d<f32>;
[[group(0), binding(5)]]
var s_specular: sampler;
[[group(0), binding(6)]]
var t_ambient: texture_2d<f32>;
[[group(0), binding(7)]]
var s_ambient: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
    let object_specular: vec4<f32> = textureSample(t_specular, s_specular, in.tex_coords);
    let object_ambient: vec4<f32> = textureSample(t_ambient, s_ambient, in.tex_coords);
    
    // We don't need (or want) much ambient light, so 0.1 is fine
    let ambient_color = object_ambient.rgb * light.intensity;

    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    let light_dir = normalize(in.tangent_light_position - in.tangent_position);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
    let half_dir = normalize(view_dir + light_dir);

    let diffuse_strength = clamp(dot(tangent_normal, light_dir), 0.0, 1.0);
    let diffuse_color = light.color * diffuse_strength * light.intensity;

    //let light_reflection = reflect(light_dir, tangent_normal);

    let specular_strength = pow(clamp(dot(tangent_normal, half_dir), 0.0, 1.0), 32.0);
    let specular_color = object_specular.rgb * specular_strength * light.color * light.intensity;

    let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;

    return vec4<f32>(result, object_color.a);
}