struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}


@group(0) @binding(0)
var<uniform> projection: mat4x4<f32>;
@group(0) @binding(1)
var<uniform> view: mat4x4<f32>;


@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = projection * view * vec4<f32>(in.position, 1.0);
    return out;
}


const light_direction = vec3<f32>(0.25, 1.0, -0.5);
const ambient_light = 0.005;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    var color: vec4<f32>;

    color = vec4<f32>(1.0, 0.0, 0.0, 0.2);

    return color;
}
