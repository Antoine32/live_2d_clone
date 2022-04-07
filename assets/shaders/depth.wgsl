// Vertex shader
struct Camera {
    view_pos: vec4<f32>;
    proj_matrix: mat4x4<f32>;
    view_matrix: mat4x4<f32>;
    acne_bias: f32;
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
[[group(2), binding(1)]]
var<uniform> shadow_map: Camera;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
};
struct InstanceInput {
    [[location(5)]] model_matrix_0: vec4<f32>;
    [[location(6)]] model_matrix_1: vec4<f32>;
    [[location(7)]] model_matrix_2: vec4<f32>;
    [[location(8)]] model_matrix_3: vec4<f32>;
    [[location(9)]] normal_matrix_0: vec3<f32>;
    [[location(10)]] normal_matrix_1: vec3<f32>;
    [[location(11)]] normal_matrix_2: vec3<f32>;
    [[location(12)]] scale_vector: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] world_position: vec4<f32>;
    [[location(2)]] view_position: vec4<f32>;
    [[location(3)]] normal: vec3<f32>;
    [[location(4)]] shadow_pos: vec3<f32>;
    [[location(5)]] model_view_matrix: mat3x3<f32>;
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
    
    let rev_z = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, -1.0, -1.0,
        0.0, 0.0, 1.0, 0.0
    );

    let buf_matrix = camera.view_matrix * model_matrix;
    let model_view_matrix = mat3x3<f32>(buf_matrix[0].xyz, buf_matrix[1].xyz, buf_matrix[2].xyz);

    let world_position: vec4<f32> = model_matrix * vec4<f32>(model.position * instance.scale_vector, 1.0);
    let view_position: vec4<f32> = camera.view_matrix * world_position;
    let pos_from_light = shadow_map.view_matrix * (shadow_map.proj_matrix) * world_position;

    var world_normal: vec3<f32> = normalize(model.normal);

    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.proj_matrix * view_position;
    out.world_position = world_position;
    out.view_position = view_position;
    out.normal = world_normal;
    out.model_view_matrix = model_view_matrix; 
    out.shadow_pos = vec3<f32>(pos_from_light.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5), pos_from_light.z); //  
    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_shadow: texture_depth_2d;
[[group(0), binding(1)]]
var s_shadow: sampler_comparison;

fn get_average_visibility(homogeneous_coords: vec4<f32>) -> f32 {
    var result: f32 = 1.0;

    if (homogeneous_coords.w <= 0.0) {
        result = 1.0;
    }

    // compensate for the Y-flip difference between the NDC and texture coordinates
    let flip_correction = vec2<f32>(0.5, -0.5);
    // compute texture coordinates for shadow lookup
    let proj_correction = 1.0 / homogeneous_coords.w;
    let light_local = homogeneous_coords.xy * flip_correction * proj_correction + vec2<f32>(0.5, 0.5);
    // do the lookup, using HW PCF and comparison
    let b = homogeneous_coords.z * proj_correction;
    result = textureSampleCompare(t_shadow, s_shadow, light_local, b);
    

    return result;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    // derivations of the fragment position
    let pos_dx: vec3<f32> = dpdx(in.view_position.xyz);
    let pos_dy: vec3<f32> = dpdy(in.view_position.xyz);

    // derivations of the texture coordinate
    let texc_dx: vec2<f32> = dpdx(in.tex_coords);
    let texc_dy: vec2<f32> = dpdy(in.tex_coords);

    // tangent vector and binormal vector
    var t: vec3<f32> = texc_dy.y * pos_dx - texc_dx.y * pos_dy;
    var b: vec3<f32> = texc_dx.x * pos_dy - texc_dy.x * pos_dx;
    var n: vec3<f32> = in.normal;

    t = t - n * dot( t, n );
    b = b - n * dot( b, n );
    b = b - t * dot( b, t );

    var tbn: mat3x3<f32> = mat3x3<f32>(normalize(t), normalize(b), n);

    let tangent_matrix = in.model_view_matrix * tbn;
    
    let view_light_position = camera.view_matrix * vec4<f32>(light.position, 1.0);
    // let view_light_position = tangent_matrix * view_light_position.xyz;
    let light_dir = normalize((view_light_position - in.view_position).xyz);

    let light_color_intensity = light.color * light.intensity; 
    let dist_light = distance(in.view_position, view_light_position);

    //let r = (2.0 * near) / (far + near - depth * (far - near));
    let visibility = get_average_visibility(shadow_map.proj_matrix * in.world_position);
    return vec4<f32>(vec3<f32>(1.0, 1.0, 1.0) * visibility, 1.0);
}