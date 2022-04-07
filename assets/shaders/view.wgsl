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
    [[location(12)]] scale_vector: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] world_position: vec4<f32>;
    [[location(3)]] view_position: vec4<f32>;
    [[location(4)]] view_normal: vec3<f32>;
    [[location(5)]] normal: vec3<f32>;
    [[location(6)]] model_view_matrix: mat3x3<f32>;
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
    
    var world_normal: vec3<f32> = normalize(model.normal);

    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_normal = normal_matrix * model.normal;
    out.world_position = world_position;
    out.view_position = view_position; 
    out.view_normal = normalize((camera.view_matrix * model_matrix * vec4<f32>(world_normal, 1.0)).xyz);
    out.clip_position = camera.proj_matrix * view_position;
    out.normal = world_normal;
    out.model_view_matrix = model_view_matrix;

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

[[group(3), binding(0)]]
var t_shadow: texture_depth_2d;
[[group(3), binding(1)]]
var s_shadow: sampler_comparison;

fn get_average_visibility(shadow_pos: vec3<f32>) -> f32 {
    //let current_depth = length(frag_light_position) / 1.0; // far_plane
    //let dir_light = normalize(frag_light_position.xyz);
    let current_depth = shadow_pos.z;

    let oneOverShadowDepthTextureSize = 1.0 / 1024.0;

    var visibility = 0.0;
    for (var x: f32 = -2.0; x <= 2.0; x = x + 1.0) {
        for (var y: f32 = -2.0; y <= 2.0; y = y + 1.0) {
            //for (var z: f32 = -2.0; z <= 2.0; z = z + 1.0) {
                //let closest_depth = textureGatherCompare(t_shadow, s_shadow, shadow_pos.xy + vec2<f32>(x * 1024.0, y * 1024.0), 0.025).z;
                //visibility = visibility + current_depth - closest_depth;
            //}
        }
    }

    return visibility / 25.0;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
    let object_specular: vec4<f32> = textureSample(t_specular, s_specular, in.tex_coords);
    let object_ambient: vec4<f32> = textureSample(t_ambient, s_ambient, in.tex_coords);

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
    let light_dir = normalize((view_light_position - in.view_position).xyz);

    let tangent_normal = tangent_matrix * normalize((object_normal.xyz * 2.0) - 1.0);

    let light_color_intensity = light.color * light.intensity; 
    let dist_light = distance(in.view_position, view_light_position);
    let light_spread = pow(dist_light / light.spread, 2.0);

    let diffuse_strength = clamp(dot(tangent_normal, light_dir), 0.0, 1.0);
    let diffuse_light = (light_color_intensity * diffuse_strength) / light_spread;

    let camera_position_viewspace = vec3<f32>(0.0, 0.0, 0.0); // In view-space, the camera is in the center of the world, so it's position would be (0, 0, 0).
    let view_dir = normalize(in.view_position.xyz - camera_position_viewspace);
    let light_reflection = reflect(light_dir, tangent_normal);

    let specular_strength = pow(clamp(dot(view_dir, light_reflection), 0.0, 1.0), light.specular_lobe_factor); 
    let specular_light = (light_color_intensity * specular_strength) / light_spread;

    let visibility = 1.0;//clamp(get_average_visibility(in.shadow_pos), 0.01, 1.0);

    let result = (object_ambient.xyz * light.intensity) + ((object_color.xyz * diffuse_light) + (object_specular.xyz * specular_light)) * visibility;

    return vec4<f32>(result, object_color.a);
}