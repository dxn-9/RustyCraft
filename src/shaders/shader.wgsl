

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec3<f32>,
    
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
    @location(3) block_type: u32
}


// @group(0) @binding(0) 
// var<uniform> transform: mat4x4<f32>;
@group(0) @binding(0) 
var<uniform> projection: mat4x4<f32>;
@group(0) @binding(1) 
var<uniform> view: mat4x4<f32>;
@group(1) @binding(0)
var <uniform> current_chunk: vec2<i32>;
// @group(2) @binding(1)
// var <storage, read> chunk_data: vec4<u32>;


@vertex
fn vs_main(in: VertexInput, instance_data: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    let chunk_offset = vec4<f32>(f32(current_chunk.x) * 16.0, 0.0, f32(current_chunk.y) * 16.0, 0.0);

    out.clip_position = projection * view * (vec4<f32>(in.position.xyz, 1.0) + chunk_offset);
    out.normals = in.normal;

    return out;
}


// @group(1) @binding(0)
// var diffuse: texture_2d<f32>;
// @group(1) @binding(1)
// var t_sampler: sampler;

struct FragmentInput {
        @location(0) tex_coords: vec2<f32>,
        @location(1) normals: vec3<f32>,
        @location(2) current_chunk: vec2<i32>,
        @location(3) block_type: u32
}



    

const light_direction = vec3<f32>(0.3, 0.7, 0.0);
const ambient_light = 0.05;

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    var color: vec4<f32>;

    color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    color *= dot(in.normals, light_direction);
    color += vec4<f32>(vec3<f32>(ambient_light), 0.0);

    return color;
}
