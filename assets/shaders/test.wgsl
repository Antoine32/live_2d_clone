// Vertex shader

struct Camera {
    proj_matrix: mat4x4<f32>;
    view_matrix: mat4x4<f32>;
    ambient: vec3<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: Camera;

// 80
struct Light {
    //proj_matrix: mat4x4<f32>;
    view_matrix: mat3x3<f32>;
    position: vec3<f32>;
    color: vec4<f32>;
    intensity: f32;
    spread: f32;
    specular: f32; // specular_lobe_factor
    bias: f32; // acne_bias
};

struct Lights {
    data: [[stride(96)]] array<Light>;
};

// Used when storage types are not supported
struct LightsWithoutStorage {
    data: array<Light, 10>;
};

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
    //[[location(3)]] tangent: vec2<f32>;
    //[[location(4)]] bitangent: vec2<f32>;
};
struct InstanceInput {
    [[location(3)]] model_matrix_0: vec4<f32>;
    [[location(4)]] model_matrix_1: vec4<f32>;
    [[location(5)]] model_matrix_2: vec4<f32>;
    [[location(6)]] model_matrix_3: vec4<f32>;
    [[location(7)]] normal_matrix_0: vec3<f32>;
    [[location(8)]] normal_matrix_1: vec3<f32>;
    [[location(9)]] normal_matrix_2: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] world_position: vec4<f32>;
    [[location(2)]] view_position: vec4<f32>;
    [[location(3)]] model_view_matrix: mat3x3<f32>; // 3, 4, 5
    //[[location(6)]] tbn: mat3x3<f32>; // 6, 7, 8
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

    //let buf_matrix = camera.view_matrix * model_matrix; // camera.view_matrix * 
    //let model_view_matrix = mat3x3<f32>(buf_matrix.x.xyz, buf_matrix.y.xyz, buf_matrix.z.xyz);
 
    let world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 0.0, 1.0); // model_matrix * 
    let view_position: vec4<f32> = camera.view_matrix * world_position; // camera.view_matrix * 

    //let normal: vec3<f32> = normalize(normal_matrix * model.normal); 

    var out: VertexOutput;
    out.world_position = world_position;
    //out.tbn = mat3x3<f32>(normalize(model.tangent), normalize(model.bitangent), normal);
    out.tex_coords = model.tex_coords;
    //out.model_view_matrix = model_view_matrix;
    out.view_position = view_position;
    out.clip_position = camera.proj_matrix * view_position; // camera.proj_matrix * 

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

//[[group(2), binding(0)]]
//var<storage, read> s_lights: Lights;
//[[group(2), binding(0)]]
//var<uniform> u_lights: LightsWithoutStorage;
//[[group(2), binding(1)]]
//var t_shadow: texture_depth_2d_array;
//[[group(2), binding(2)]]
//var s_shadow: sampler_comparison;

//fn fetch_shadow(light_id: u32, homogeneous_coords: vec4<f32>) -> f32 {
//    if (homogeneous_coords.w <= 0.0) {
//        return 1.0;
//    }
//    // compensate for the Y-flip difference between the NDC and texture coordinates
//    let flip_correction = vec2<f32>(0.5, -0.5);
//    // compute texture coordinates for shadow lookup
//    let proj_correction = 1.0 / homogeneous_coords.w;
//    let light_local = homogeneous_coords.xy * flip_correction * proj_correction + vec2<f32>(0.5, 0.5);
//    // do the lookup, using HW PCF and comparison
//    
//    let range = 2.0;
//    let step = 1.0;
//
//    var count = 0.0;
//    var visibility = 0.0;
//    for (var x: f32 = -range; x <= range; x = x + step) {
//        for (var y: f32 = -range; y <= range; y = y + step) {
//            count = count + 1.0;
//            visibility = visibility + textureSampleCompareLevel(t_shadow, s_shadow, light_local + vec2<f32>(x / 1024.0, y / 1024.0), i32(light_id), homogeneous_coords.z * proj_correction);
//        }
//    }
//
//    return clamp(visibility / count, 0.01, 1.0);
//}

fn get_tangent_matrix(view_position: vec3<f32>, tex_coords: vec2<f32>, model_view_matrix: mat3x3<f32>, tbn: mat3x3<f32>) -> mat3x3<f32> {
    // derivations of the fragment position
    //let pos_dx: vec3<f32> = dpdx(view_position);
    //let pos_dy: vec3<f32> = dpdy(view_position);

    // derivations of the texture coordinate
    //let texc_dx: vec2<f32> = dpdx(tex_coords);
    //let texc_dy: vec2<f32> = dpdy(tex_coords);

    // tangent vector and binormal vector
    //var t: vec3<f32> = texc_dy.y * pos_dx - texc_dx.y * pos_dy;
    //var b: vec3<f32> = texc_dx.x * pos_dy - texc_dy.x * pos_dx;

    //t = t - n * dot( t, n ); // t = cross(cross(n, t), t);
    //b = b - n * dot( b, n ); // b = cross(n, t);
    //b = b - t * dot( b, t );

    //let tbn: mat3x3<f32> = mat3x3<f32>(normalize(t), normalize(b), n);

    let tangent_matrix: mat3x3<f32> = model_view_matrix * tbn;

    return tangent_matrix;
}

//fn calculate_light(light: Light, i: u32, view_position: vec3<f32>, world_position: vec3<f32>, tangent_normal: vec3<f32>, object_color: vec4<f32>, object_specular: vec4<f32>) -> vec3<f32> {
//    let light_position = camera.view_matrix * light.position;
//    let light_dir = normalize(-abs(view_position - light_position).xyz);
//
//    let light_color_intensity = light.color.xyz * light.intensity; 
//    let dist_light = distance(view_position, light_position);
//    let light_spread = pow(dist_light, light.spread);
//
//    let diffuse_strength = clamp(dot(tangent_normal, light_dir), 0.0, 1.0);
//    var diffuse_light: vec3<f32> = (light_color_intensity * diffuse_strength) / light_spread;
//
//    let camera_position_viewspace = vec3<f32>(0.0, 0.0, 0.0); // In view-space, the camera is in the center of the world, so it's position would be (0, 0, 0).
//    let view_dir = normalize(view_position.xyz - camera_position_viewspace);
//    let light_reflection = reflect(light_dir, tangent_normal);
//
//    let specular_strength = clamp(pow(clamp(dot(view_dir, light_reflection), 0.0, 1.0), light.specular), 0.0, 10.0); 
//    var specular_light: vec3<f32> = (light_color_intensity * specular_strength) / light_spread;
//
//    let shadow = 1.0;//fetch_shadow(i, light.proj_matrix * (light.view_matrix * world_position));
//
//    return ((object_color.xyz * diffuse_light) + (object_specular.xyz * specular_light)) * shadow;
//}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    //let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
    //let object_specular: vec4<f32> = textureSample(t_specular, s_specular, in.tex_coords);
    let object_ambient: vec4<f32> = textureSample(t_ambient, s_ambient, in.tex_coords) * 10.0;

    //let tangent_matrix: mat3x3<f32> = get_tangent_matrix(in.view_position.xyz, in.tex_coords, in.model_view_matrix, in.tbn);
    //let tangent_normal: vec3<f32> = tangent_matrix * normalize((object_normal.xyz * 2.0) - 1.0);

    var color: vec3<f32> = (camera.ambient * object_color.xyz) + (object_ambient.xyz * object_color.xyz);

    //for(var i = 0u; i < arrayLength(&s_lights.data); i = i + 1u) { // camera.num_lights
        //let light = s_lights.data[i];
        //color = color + calculate_light(light, i, in.view_position, in.world_position, tangent_normal, object_color, object_specular);
    //}

    return vec4<f32>(object_color.xyz, object_color.a);
}

// The fragment entrypoint used when storage buffers are not available for the lights
//[[stage(fragment)]]
//fn fs_main_without_storage(in: VertexOutput) -> [[location(0)]] vec4<f32> {
//    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
//    let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
//    let object_specular: vec4<f32> = textureSample(t_specular, s_specular, in.tex_coords);
//    let object_ambient: vec4<f32> = textureSample(t_ambient, s_ambient, in.tex_coords) * 100.0;
//
//    let tangent_matrix = get_tangent_matrix(in.view_position.xyz, in.tex_coords, in.model_view_matrix, in.tbn);
//    let tangent_normal = tangent_matrix * normalize((object_normal.xyz * 2.0) - 1.0);
//
//    var color: vec3<f32> = (camera.ambient * object_color.xyz) + (object_ambient.xyz * object_color.xyz);
//
//    for(var i = 0u; i < min(1u, camera.num_lights); i = i + 1u) {
//        let light = u_lights.data[i];
//        color = color + calculate_light(light, i, in.view_position, in.world_position, tangent_normal, object_color, object_specular);
//    }
//
//    return vec4<f32>(color, object_color.a);
//}