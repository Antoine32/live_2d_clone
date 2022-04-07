// 176
struct Light {
    view_matrix: mat3x3<f32>;
};
[[group(0), binding(0)]]
var<uniform> light: Light;

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
    [[location(3)]] tangent: vec2<f32>;
    [[location(4)]] bitangent: vec2<f32>;
};
struct InstanceInput {
    [[location(5)]] model_matrix_0: vec3<f32>;
    [[location(6)]] model_matrix_1: vec3<f32>;
    [[location(7)]] model_matrix_2: vec3<f32>;
    [[location(8)]] normal_matrix_0: vec3<f32>;
    [[location(9)]] normal_matrix_1: vec3<f32>;
    [[location(10)]] normal_matrix_2: vec3<f32>;
};

[[stage(vertex)]]
fn vs_bake(
    model: VertexInput,
    instance: InstanceInput,
) -> [[builtin(position)]] vec4<f32> {
    let model_matrix = mat3x3<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
    );

    let world_position: vec3<f32> = model_matrix * vec3<f32>(model.position, 1.0);
    let view_position: vec3<f32> = light.view_matrix * world_position;

    return vec4<f32>(view_position, 1.0);
}
