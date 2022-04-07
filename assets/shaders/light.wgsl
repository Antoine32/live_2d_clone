// Vertex shader

struct Camera {
    view_pos: vec4<f32>;
    proj_matrix: mat4x4<f32>;
    view_matrix: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>;
    light_intensity: f32;
    light_color: vec3<f32>;
    ambient_intensity: f32;
    ambient_color: vec3<f32>;
    specular_reflectivity: f32;
    spread: f32;
    specular_lobe_factor: f32;
};
[[group(1), binding(0)]]
var<uniform> light: Light;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    let scale = 0.25;
    var out: VertexOutput;
    out.clip_position = camera.proj_matrix * camera.view_matrix * vec4<f32>(model.position * scale + light.position, 1.0);
    out.color = light.light_color;
    return out;
}

// Fragment shader

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}