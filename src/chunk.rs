use glam::Vec3;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, Weak};

use wgpu::util::DeviceExt;

use crate::world::World;
use crate::{
    blocks::{
        block::{Block, BlockFace, BlockVertexData, FaceDirections},
        block_type::BlockType,
    },
    world::{NoiseData, CHUNK_HEIGHT, CHUNK_SIZE, NOISE_CHUNK_PER_ROW, NOISE_SIZE},
};

pub type BlockVec = Vec<Vec<Option<Arc<Mutex<Block>>>>>;

pub struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    pub x: i32,
    pub y: i32,
    pub blocks: BlockVec,
    pub indices: u32,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub noise_data: Arc<NoiseData>,
    pub chunk_bind_group: wgpu::BindGroup,
    pub chunk_position_buffer: wgpu::Buffer,
    pub chunk_index_buffer: Option<wgpu::Buffer>,
    pub chunk_vertex_buffer: Option<wgpu::Buffer>,
}

impl Chunk {
    pub fn remove_block(&mut self, block_r_position: &Vec3) {
        let y_blocks = self
            .blocks
            .get_mut(((block_r_position.x * CHUNK_SIZE as f32) + block_r_position.z) as usize)
            .expect("Cannot delete oob block");
        y_blocks[block_r_position.y as usize] = None;
    }
    pub fn exists_block_at(blocks: &BlockVec, position: &glam::Vec3) -> bool {
        if let Some(y_blocks) =
            blocks.get(((position.x as u32 * CHUNK_SIZE) + position.z as u32) as usize)
        {
            if let Some(block_opt) = y_blocks.get(position.y as usize) {
                if let Some(_) = block_opt {
                    return true;
                }
            }
        }
        return false;
    }
    pub fn get_block_at_relative(&self, position: &glam::Vec3) -> Option<Arc<Mutex<Block>>> {
        if let Some(y_blocks) = self
            .blocks
            .get(((position.x * CHUNK_SIZE as f32) + position.z) as usize)
        {
            if let Some(block) = y_blocks.get(position.y as usize)? {
                return Some(Arc::clone(block));
            }
        }
        return None;
    }
    pub fn is_outside_chunk(position: &glam::Vec3) -> bool {
        if position.x < 0.0
            || position.x >= CHUNK_SIZE as f32
            || position.z < 0.0
            || position.z >= CHUNK_SIZE as f32
        {
            true
        } else {
            false
        }
    }
    pub fn is_outside_bounds(position: &glam::Vec3) -> bool {
        if position.y < 0.0 {
            true
        } else {
            false
        }
    }
    pub fn build_mesh(&mut self, other_chunks: Vec<Arc<Mutex<Chunk>>>) {
        let mut vertex: Vec<BlockVertexData> = vec![];
        let mut indices: Vec<u32> = vec![];
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let region = &self.blocks[(x * CHUNK_SIZE + z) as usize];
                for y in 0..region.len() {
                    let block = &region[y];
                    if let Some(block) = block {
                        let block = block.lock().unwrap();
                        let position = block.position;
                        let faces = block.faces.as_ref().unwrap();

                        for face in faces.iter() {
                            let mut is_visible = true;
                            let face_position = face.get_normal_vector() + position;

                            if Chunk::is_outside_bounds(&face_position) {
                                is_visible = false;
                            } else if Chunk::is_outside_chunk(&face_position) {
                                let target_chunk_x = self.x
                                    + (f32::floor(face_position.x / CHUNK_SIZE as f32) as i32);
                                let target_chunk_y = self.y
                                    + (f32::floor(face_position.z / CHUNK_SIZE as f32) as i32);

                                let target_block = glam::vec3(
                                    (face_position.x + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                                    face_position.y,
                                    (face_position.z + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                                );

                                let target_chunk = other_chunks.iter().find(|c| {
                                    let c = c.lock().unwrap();
                                    c.x == target_chunk_x && c.y == target_chunk_y
                                });
                                match target_chunk {
                                    Some(chunk) => {
                                        let chunk = chunk.lock().unwrap();
                                        if Chunk::exists_block_at(&chunk.blocks, &target_block) {
                                            is_visible = false;
                                        }
                                    }
                                    None => {
                                        if face_position.y as u32
                                            <= Chunk::get_height_value(
                                            target_chunk_x,
                                            target_chunk_y,
                                            target_block.x as u32,
                                            target_block.z as u32,
                                            self.noise_data.clone(),
                                        )
                                        {
                                            is_visible = false
                                        };
                                    }
                                }
                            } else if Chunk::exists_block_at(&self.blocks, &face_position) {
                                is_visible = false;
                            }

                            if is_visible {
                                let (mut vertex_data, index_data) = face.create_face_data(&block);
                                vertex.append(&mut vertex_data);
                                let indices_offset = vertex.len() as u32 - 4;
                                indices.append(
                                    &mut index_data.iter().map(|i| i + indices_offset).collect(),
                                )
                            }
                        }
                    }
                }
            }
        }

        let chunk_vertex_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&vertex),
                    label: Some(&format!("chunk-vertex-{}-{}", self.x, self.y)),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        let chunk_index_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&indices),
                    label: Some(&format!("chunk-vertex-{}-{}", self.x, self.y)),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });

        self.indices = indices.len() as u32;
        self.chunk_vertex_buffer = Some(chunk_vertex_buffer);
        self.chunk_index_buffer = Some(chunk_index_buffer);
    }
    pub fn get_bind_group_layout() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("chunk_bind_group"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        }
    }

    pub fn get_height_value(
        chunk_x: i32,
        chunk_y: i32,
        x: u32,
        z: u32,
        noise_data: Arc<NoiseData>,
    ) -> u32 {
        let mut x = (chunk_x * CHUNK_SIZE as i32) + x as i32 % NOISE_SIZE as i32;
        let mut z = (chunk_y * CHUNK_SIZE as i32) + z as i32 % NOISE_SIZE as i32;

        if x < 0 {
            x = NOISE_SIZE as i32 + (x % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
        }
        if z < 0 {
            z = NOISE_SIZE as i32 + (z % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
        }

        let y_top = (noise_data[((z * NOISE_SIZE as i32) + x) as usize] + 1.0) * 0.5;
        return (f32::powf(100.0, y_top) - 1.0) as u32;
    }

    pub fn create_blocks_data(chunk_x: i32, chunk_y: i32, noise_data: Arc<NoiseData>) -> BlockVec {
        let size = (CHUNK_SIZE * CHUNK_SIZE) as usize;
        let mut blocks: BlockVec = Vec::with_capacity(size);

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                blocks.push(vec![]);

                let y_top = Chunk::get_height_value(chunk_x, chunk_y, x, z, noise_data.clone());

                for y in 0..=y_top {
                    let block_type = match BlockType::from_y_position(y) {
                        BlockType::Dirt(..) if y == y_top => BlockType::grass(),
                        b => b,
                    };

                    let block = Arc::new(Mutex::new(Block {
                        faces: None,
                        position: glam::vec3(x as f32, y as f32, z as f32),
                        absolute_position: glam::vec3(
                            (chunk_x * CHUNK_SIZE as i32 + x as i32) as f32,
                            y as f32,
                            (chunk_y * CHUNK_SIZE as i32 + z as i32) as f32,
                        ),
                        block_type,
                        is_translucent: false,
                    }));

                    let face_directions =
                        FaceDirections::all().iter().map(|f| *f).collect::<Vec<_>>();

                    block.lock().unwrap().faces = Some(face_directions);
                    let curr = &mut blocks[((x * CHUNK_SIZE) + z) as usize];
                    curr.push(Some(block.clone()));
                }
            }
        }

        blocks
    }

    pub fn new(
        x: i32,
        y: i32,
        noise_data: Arc<NoiseData>,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        chunk_data_layout: Arc<wgpu::BindGroupLayout>,
        other_chunks: Vec<Arc<Mutex<Chunk>>>,
    ) -> Self {
        let blocks = Self::create_blocks_data(x, y, noise_data.clone());

        let chunk_position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[x, y]),
            label: Some(&format!("chunk-position-{x}-{y}")),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let chunk_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: chunk_data_layout.as_ref(),
            label: Some(&format!("chunk-bg-{x}-{y}")),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: chunk_position_buffer.as_entire_binding(),
            }],
        });

        let mut chunk = Chunk {
            blocks,
            x,
            y,
            device,
            queue,
            noise_data,
            chunk_vertex_buffer: None,
            chunk_index_buffer: None,
            chunk_bind_group,
            chunk_position_buffer,
            indices: 0,
        };
        chunk.build_mesh(other_chunks);

        return chunk;
    }
}
