

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>
}


@group(0) @binding(0) 
var<uniform> transform: mat4x4<f32>;
@group(0) @binding(1) 
var<uniform> projection: mat4x4<f32>;
@group(0) @binding(2) 
var<uniform> view: mat4x4<f32>;



@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = in.tex_coords;
    out.clip_position = projection * view * transform * vec4<f32>(in.position.xyz, 1.0);

    return out;
}


@group(1) @binding(0)
var diffuse: texture_2d<f32>;
@group(1) @binding(1)
var t_sampler: sampler;

struct FragmentInput {
    @location(0) tex_coords: vec2<f32>
}


@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(diffuse, t_sampler, in.tex_coords).xyz, 1.0);
}
