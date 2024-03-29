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

@group(1) @binding(0)
var diffuse: texture_2d<f32>;
@group(1) @binding(1)
var t_sampler: sampler;

struct FragmentInput {
        @location(0) tex_coords: vec2<f32>,
        @location(1) normals: vec3<f32>,
        @location(2) current_chunk: vec2<i32>,
        @location(3) block_type: u32
}

const light_direction = vec3<f32>(0.25, 1.0, -0.5);
const ambient_light = 0.005;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    var color: vec4<f32>;

    color = vec4<f32>(1.0, 0.0, 0.0 , 0.2);

    return color;
}
