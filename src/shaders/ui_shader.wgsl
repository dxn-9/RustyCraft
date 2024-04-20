struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>

}


struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,

}


@group(0) @binding(3)
var diffuse: texture_2d<f32>;
@group(0) @binding(4)
var t_sampler: sampler;
@group(1) @binding(0)
var<uniform> resolution: vec2<f32>;
@group(1) @binding(1)
var<uniform> blockid: u32;


@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.pos = vec2<f32>(in.position);
    out.uv = in.uv;

    return out;
}


struct FragmentInput {
        @builtin(position) clip_position: vec4<f32>,
        @location(0) pos: vec2<f32>,
        @location(1) uv: vec2<f32>,
}


@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    var color: vec4<f32>;
    // Normalize in range 0->1

    color = textureSample(diffuse, t_sampler, in.uv);
    // let norm = (in.pos + 1.0) * 0.5;
    // let coords = norm * vec3<f32>(resolution, 1.0);

    // let v = distance(coords, vec3<f32>(500.0, 500.0, 1.0));

    // color = vec4<f32>(vec3<f32>(v), 1.0);

    return color;
}
