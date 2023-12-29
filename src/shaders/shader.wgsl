

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    
}
struct InstanceInput {
    // @location(2) instance_transform: vec3<f32>,
    @builtin(instance_index) instance_index: u32,
};
 

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
@group(0) @binding(3)
var <storage, read> chunks: vec4<u32>;
@group(2) @binding(0)
var <uniform> current_chunk: vec2<i32>;


@vertex
fn vs_main(in: VertexInput, instance_data: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = in.tex_coords;
    var instance_transform: vec4<f32>;

    let cy = current_chunk.y + 1;
    let cx = current_chunk.x + 1;
    let chunk_region = (16 * 16) * 255 * 4 * 4;

    let co = ((cy * 3) + cx) * chunk_region;

    instance_transform.x = f32(chunks[instance_data.instance_index * u32(4) + u32(0) + u32(co)]) + f32(current_chunk.x * 16) ;
    instance_transform.y = f32(chunks[instance_data.instance_index * u32(4) + u32(1) + u32(co)]);
    instance_transform.z = f32(chunks[instance_data.instance_index * u32(4) + u32(2) + u32(co)]) + f32(current_chunk.y * 16);
    instance_transform.w = 0.0;

    instance_transform.y -= 20.0;






    // let instance_transform = vec4<f32>(instance_data.instance_transform, 1.0);
    out.clip_position = projection * view * transform * (vec4<f32>(in.position.xyz, 1.0) + instance_transform);

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
