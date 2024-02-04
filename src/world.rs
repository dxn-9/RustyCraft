use std::{
    borrow::Borrow,
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
};

// use env_logger::fmt::termcolor::Buffer;
use glam::{vec3, Vec3};
use rand::random;
use wgpu::{
    util::DeviceExt, BindGroupLayout, BindGroupLayoutDescriptor, BufferUsages, TextureViewDimension,
};

use crate::{
    blocks::{
        block::{Block, BlockFace, BlockVertexData, FaceDirections, CUBE_VERTEX},
        block_type::BlockType,
    },
    camera::{Camera, Player},
    model::{InstanceData, Model},
    state::State,
    utils::threadpool::ThreadPool,
};

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_HEIGHT: u8 = u8::MAX;
pub const NOISE_SIZE: u32 = 1024;
pub const FREQUENCY: f32 = 1. / 128.;
pub const NOISE_CHUNK_PER_ROW: u32 = NOISE_SIZE / CHUNK_SIZE;
// There will be a CHUNKS_PER_ROW * CHUNKS_PER_ROW region
pub const CHUNKS_PER_ROW: u32 = 15;

pub const CHUNKS_REGION: u32 = CHUNKS_PER_ROW * CHUNKS_PER_ROW;

// Lower bound of chunk
pub const LB: i32 = -((CHUNKS_PER_ROW / 2) as i32);
// Upper boound of chunk
pub const UB: i32 = if CHUNKS_PER_ROW % 2 == 0 {
    (CHUNKS_PER_ROW / 2 - 1) as i32
} else {
    (CHUNKS_PER_ROW / 2) as i32
};

type BlockVec = Vec<Vec<Arc<Mutex<Block>>>>;
type NoiseData = Vec<f32>;
pub struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    pub x: i32,
    pub y: i32,
    pub blocks: BlockVec,
    pub indices: u32,
    pub chunk_bind_group: wgpu::BindGroup,
    pub chunk_position_buffer: wgpu::Buffer,
    // pub chunk_vertex_buffer: wgpu::Buffer,
    pub chunk_index_buffer: wgpu::Buffer,
    pub chunk_vertex_buffer: wgpu::Buffer,
}

pub struct World {
    pub chunks: Vec<Chunk>,
    pub thread_pool: ThreadPool,
    pub seed: u32,
    pub noise_data: Arc<NoiseData>,
    pub chunk_data_layout: Arc<wgpu::BindGroupLayout>,
}

impl World {
    pub fn update(
        &mut self,
        player: &mut Player,
        queue: Arc<wgpu::Queue>,
        device: Arc<wgpu::Device>,
    ) {
        // Check if the player has moved to a new chunk, if so, generate the new chunks
        let current_chunk = player.calc_current_chunk();
        if current_chunk != player.current_chunk {
            let delta = (
                current_chunk.0 - player.current_chunk.0,
                current_chunk.1 - player.current_chunk.1,
            );

            let o = if CHUNKS_PER_ROW % 2 == 0 { 1 } else { 0 };
            let p = CHUNKS_PER_ROW as i32 / 2;

            let new_chunks_offset = if delta.1 > 0 || delta.0 > 0 {
                p - o
            } else {
                -p
            };
            let old_chunks_offset = if delta.1 > 0 || delta.0 > 0 {
                -p
            } else {
                p - o
            };

            let mut new_chunks_positions: Vec<(i32, i32)> = vec![];

            let chunk_y_remove = player.current_chunk.1 + old_chunks_offset;
            let chunk_x_remove = player.current_chunk.0 + old_chunks_offset;
            if delta.0 != 0 {
                for i in LB + current_chunk.1..=UB + current_chunk.1 {
                    new_chunks_positions.push((current_chunk.0 + new_chunks_offset, i));
                }
            }
            if delta.1 != 0 {
                for i in LB + current_chunk.0..=UB + current_chunk.0 {
                    new_chunks_positions.push((i, current_chunk.1 + new_chunks_offset));
                }
            }

            // Filter out the duplicate ones (in case we moved diagonally)
            new_chunks_positions = new_chunks_positions
                .iter()
                .filter_map(|c| {
                    if new_chunks_positions.contains(c) {
                        Some(c.clone())
                    } else {
                        None
                    }
                })
                .collect();

            for i in 0..self.chunks.len() {
                while let Some(chunk) = self.chunks.get(i) {
                    if (delta.1 != 0 && chunk.y == chunk_y_remove)
                        || (delta.0 != 0 && chunk.x == chunk_x_remove)
                    {
                        self.chunks.remove(i);
                    } else {
                        break;
                    }
                }
            }

            let chunks_added = new_chunks_positions.len();
            let (sender, receiver) = mpsc::channel();

            for i in 0..new_chunks_positions.len() {
                let new_chunk_pos = new_chunks_positions[i];
                let sender = sender.clone();
                let noise_data = Arc::clone(&self.noise_data);
                let chunk_data_layout = Arc::clone(&self.chunk_data_layout);
                let device = Arc::clone(&device);
                let queue = Arc::clone(&queue);

                self.thread_pool.execute(move || {
                    let chunk = Chunk::new(
                        new_chunk_pos.0,
                        new_chunk_pos.1,
                        noise_data,
                        device,
                        queue,
                        chunk_data_layout,
                    );
                    sender.send(chunk).unwrap()
                })
            }

            for _ in 0..chunks_added {
                let chunk = receiver.recv().unwrap();
                self.chunks.push(chunk);
            }
        }

        player.current_chunk = current_chunk;
    }
    pub fn init_world(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let noise_data = Arc::new(crate::utils::noise::create_world_noise_data(
            NOISE_SIZE, NOISE_SIZE, FREQUENCY,
        ));
        let chunk_data_layout =
            Arc::new(device.create_bind_group_layout(&Chunk::get_bind_group_layout()));

        let thread_pool = ThreadPool::new(8);
        let (sender, receiver) = mpsc::channel();
        for chunk_x in LB..=UB {
            for chunk_y in LB..=UB {
                println!("SENDING CHUNK {chunk_x} {chunk_y}");
                let sender = sender.clone();
                let noise_data = Arc::clone(&noise_data);
                let chunk_data_layout = Arc::clone(&chunk_data_layout);
                let device = Arc::clone(&device);
                let queue = Arc::clone(&queue);
                thread_pool.execute(move || {
                    let chunk = Chunk::new(
                        chunk_x,
                        chunk_y,
                        noise_data,
                        device,
                        queue,
                        chunk_data_layout,
                    );
                    sender.send(chunk).unwrap();
                });
            }
        }

        let mut chunks = vec![];
        for _ in 0..CHUNKS_PER_ROW * CHUNKS_PER_ROW {
            let chunk = receiver.recv().unwrap();
            println!("Received chunk {} {}", chunk.x, chunk.y);
            chunks.push(chunk);
        }

        return Self {
            chunk_data_layout,
            chunks,
            noise_data,
            seed: 0,
            thread_pool,
        };
    }
}

impl Chunk {
    pub fn exists_block_at(&self, position: &glam::Vec3) -> bool {
        if let Some(y_blocks) = self
            .blocks
            .get(((position.x as u32 * CHUNK_SIZE) + position.z as u32) as usize)
        {
            if let Some(_) = y_blocks.get(position.y as usize) {
                return true;
            } else {
                return false;
            }
        } else {
            return false;
        };
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
    // Returns the number of indices added to the chunk - it would've been better to be a mutable method but i can't do it because of borrow checker
    pub fn build_mesh(&mut self, queue: Arc<wgpu::Queue>, noise_data: Arc<NoiseData>) {
        let mut vertex: Vec<BlockVertexData> = vec![];
        let mut indices: Vec<u32> = vec![];
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let region = &self.blocks[(x * CHUNK_SIZE + z) as usize];
                for y in 0..region.len() {
                    let block = &region[y];
                    let block = block.lock().unwrap();
                    let position = block.position;
                    let faces = block.faces.as_ref().unwrap();

                    for face in faces.iter() {
                        let mut is_visible = true;
                        let face_position = face.face_direction.get_normal_vector() + position;

                        if Chunk::is_outside_bounds(&face_position) {
                            is_visible = false;
                        } else if Chunk::is_outside_chunk(&face_position) {
                            let target_chunk_x =
                                self.x + (f32::floor(face_position.x / CHUNK_SIZE as f32) as i32);
                            let target_chunk_y =
                                self.y + (f32::floor(face_position.z / CHUNK_SIZE as f32) as i32);

                            let target_block = glam::vec3(
                                (face_position.x + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                                face_position.y,
                                (face_position.z + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                            );

                            // This probably needs to be looked at again when the blocks can be placed/destroyed
                            if face_position.y as u32
                                <= Chunk::get_height_value(
                                    target_chunk_x,
                                    target_chunk_y,
                                    target_block.x as u32,
                                    target_block.z as u32,
                                    noise_data.clone(),
                                )
                            {
                                is_visible = false
                            };
                        } else if self.exists_block_at(&face_position) {
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

        self.indices = indices.len() as u32;
        queue.write_buffer(&self.chunk_vertex_buffer, 0, bytemuck::cast_slice(&vertex));
        queue.write_buffer(&self.chunk_index_buffer, 0, bytemuck::cast_slice(&indices));
    }
    pub fn get_bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
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
                        block_type,
                        is_translucent: false,
                    }));

                    let face_directions = FaceDirections::all()
                        .iter()
                        .map(|face_dir| BlockFace {
                            block: Arc::downgrade(&block),
                            face_direction: *face_dir,
                        })
                        .collect::<Vec<_>>();

                    block.lock().unwrap().faces = Some(face_directions);
                    let curr = &mut blocks[((x * CHUNK_SIZE) + z) as usize];
                    curr.push(block.clone());
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
    ) -> Self {
        println!("CREATING DATA FOR {} {}", x, y);
        let blocks = Self::create_blocks_data(x, y, noise_data.clone());

        // let c = Chunk {blocks, }
        //
        let chunk_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            // This is more than needed but its the number of blocks * number of indices per face * number of faces
            size: (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * CHUNK_HEIGHT as u32) as u64 * 6 * 6,
            label: Some(&format!("chunk-index-{x}-{y}")),
            mapped_at_creation: false,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        });
        let chunk_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            // TODO: Calculate max possible size?
            size: (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * CHUNK_HEIGHT as u32) as u64 * 6 * 6,
            label: Some(&format!("chunk-index-{x}-{y}")),
            mapped_at_creation: false,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let chunk_position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[x, y]),
            label: Some(&format!("chunk-position-{x}-{y}")),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
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
            x,
            y,
            blocks,
            chunk_bind_group,
            chunk_index_buffer,
            chunk_position_buffer,
            chunk_vertex_buffer,
            indices: 0,
        };
        chunk.build_mesh(queue, noise_data);
        return chunk;
    }
}
