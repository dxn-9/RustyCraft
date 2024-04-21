

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) ao: f32,

}
struct InstanceInput {
    // @location(2) instance_transform: vec3<f32>,
    @builtin(instance_index) instance_index: u32,
};
 

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normals: vec3<f32>,
    @location(2) chunk_position: vec2<i32>,
    @location(3) block_type: u32,
    @location(4) ao: f32,
    @location(5) fog: f32
}


@group(0) @binding(0)
var<uniform> projection: mat4x4<f32>;
@group(0) @binding(1) 
var<uniform> view: mat4x4<f32>;
@group(0) @binding(2)
var <uniform> chunks_per_row: u32;
@group(0) @binding(3)
var diffuse: texture_2d<f32>;
@group(0) @binding(4)
var t_sampler: sampler;
@group(1) @binding(0)
var <uniform> current_chunk: vec2<i32>;
@group(2) @binding(0)
var <uniform> player_position: vec3<f32>;


@vertex
fn vs_main(in: VertexInput, instance_data: InstanceInput) -> VertexOutput {
    var out: VertexOutput;


    let chunk_offset = vec3<f32>(f32(current_chunk.x) * 16.0, 0.0, f32(current_chunk.y) * 16.0);
    let block_position = in.position + chunk_offset;

    let player_dist = distance(player_position, block_position);

    let r = (f32(chunks_per_row) - 1.0) * 8.0;
    out.fog = 1.0 - clamp((r - player_dist) / 8.0, 0.0, 1.0);

    // out.fog = min(pow(player_dist / 80.0, 6.0), 1.0);
    out.clip_position = projection * view * (vec4<f32>(block_position, 1.0));
    out.normals = in.normal;
    out.tex_coords = in.tex_coords;
    out.ao = in.ao;

    return out;
}



struct FragmentInput {
        @location(0) tex_coords: vec2<f32>,
        @location(1) normals: vec3<f32>,
        @location(2) current_chunk: vec2<i32>,
        @location(3) block_type: u32,
        @location(4) ao: f32,
        @location(5) fog: f32
}

const light_direction = vec3<f32>(0.25, 1.0, -0.5);
const ambient_light = 0.005;

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    var color: vec4<f32>;

    color = textureSample(diffuse, t_sampler, in.tex_coords);
    color *= max(dot(in.normals, normalize(light_direction)), 0.2);
    color += vec4<f32>(vec3<f32>(ambient_light), 0.0);
    color *= 1.0 - (in.ao * 0.9);
    color = mix(color, vec4<f32>(0.03, 0.64, 0.97, 1.0), in.fog);

    return color;
}
