

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normals: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    
}
struct InstanceInput {
    // @location(2) instance_transform: vec3<f32>,
    @builtin(instance_index) instance_index: u32,
};
 

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normals: vec3<f32>,
    @location(2) chunk_position: vec2<i32>
}


@group(0) @binding(0) 
var<uniform> transform: mat4x4<f32>;
@group(0) @binding(1) 
var<uniform> projection: mat4x4<f32>;
@group(0) @binding(2) 
var<uniform> view: mat4x4<f32>;
@group(0) @binding(3)
var <uniform> chunks_per_row: u32;
@group(2) @binding(0)
var <uniform> current_chunk: vec2<i32>;
@group(2) @binding(1)
var <storage, read> chunk_data: vec4<u32>;


@vertex
fn vs_main(in: VertexInput, instance_data: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = in.tex_coords;
    var instance_transform: vec4<f32>;

    // let chunk_offset = i32(f32(chunks_per_row) / 2.0);
    // let cy = current_chunk.y + chunk_offset;
    // let cx = current_chunk.x + chunk_offset;
    // let chunk_region = ((16 * 16) * 255) * 4 ;


    // let co = ((cy * i32(chunks_per_row)) + cx) * chunk_region;


    instance_transform.x = f32(chunk_data[instance_data.instance_index * u32(4) + u32(0)]) + f32(current_chunk.x * 16);
    instance_transform.y = f32(chunk_data[instance_data.instance_index * u32(4) + u32(1)]);
    instance_transform.z = f32(chunk_data[instance_data.instance_index * u32(4) + u32(2)]) + f32(current_chunk.y * 16);
    instance_transform.w = 0.0;

    instance_transform.y -= 100.0;




    out.chunk_position = current_chunk;
    out.normals = in.normals;


    // let instance_transform = vec4<f32>(instance_data.instance_transform, 1.0);
    out.clip_position = projection * view * transform * (vec4<f32>(in.position.xyz, 1.0) + instance_transform);

    return out;
}


@group(1) @binding(0)
var diffuse: texture_2d<f32>;
@group(1) @binding(1)
var t_sampler: sampler;

struct FragmentInput {
    @location(0) tex_coords: vec2<f32>,
    @location(1) normals: vec3<f32>,
    @location(2) current_chunk: vec2<i32>
}


@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {

    let normals_add = dot(vec3<f32>(0.0, 1.0, 0.0), in.normals);
    let texture = vec4<f32>(textureSample(diffuse, t_sampler, in.tex_coords).xyz, 1.0);
    var result: vec4<f32>;

    result = texture * clamp(normals_add, 0.5, 1.0);

    return result;
//     return vec4<f32>((f32(in.current_chunk.x + 4) / 9.0), f32((in.current_chunk.y + 4)) / 9.0, 1.0, 1.0);
}
