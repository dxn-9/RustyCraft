use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

use crate::{
    blocks::block::Block,
    camera::Player,
    chunk::{BlockVec, Chunk},
    collision::CollisionBox,
    utils::threadpool::ThreadPool,
};

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_HEIGHT: u8 = u8::MAX;
pub const NOISE_SIZE: u32 = 1024;
pub const FREQUENCY: f32 = 1. / 128.;
pub const NOISE_CHUNK_PER_ROW: u32 = NOISE_SIZE / CHUNK_SIZE;
// There will be a CHUNKS_PER_ROW * CHUNKS_PER_ROW region
pub const CHUNKS_PER_ROW: u32 = 9;

pub const CHUNKS_REGION: u32 = CHUNKS_PER_ROW * CHUNKS_PER_ROW;

// Lower bound of chunk
pub const LB: i32 = -((CHUNKS_PER_ROW / 2) as i32);
// Upper boound of chunk
pub const UB: i32 = if CHUNKS_PER_ROW % 2 == 0 {
    (CHUNKS_PER_ROW / 2 - 1) as i32
} else {
    (CHUNKS_PER_ROW / 2) as i32
};

pub type NoiseData = Vec<f32>;
pub struct World {
    pub chunks: Vec<Chunk>,
    pub thread_pool: ThreadPool,
    pub seed: u32,
    pub noise_data: Arc<NoiseData>,
    pub chunk_data_layout: Arc<wgpu::BindGroupLayout>,
}

impl World {
    pub fn get_blocks_absolute(&self, position: &glam::Vec3) -> Option<Arc<Mutex<Block>>> {
        let chunk_x = (f32::floor(f32::floor(position.x) / CHUNK_SIZE as f32)) as i32;
        let chunk_y = (f32::floor(f32::floor(position.z) / CHUNK_SIZE as f32)) as i32;

        let chunk = self
            .chunks
            .iter()
            .find(|c| c.x == chunk_x && c.y == chunk_y)?;

        let x =
            ((f32::floor(position.x) % CHUNK_SIZE as f32) + CHUNK_SIZE as f32) % CHUNK_SIZE as f32;
        let z =
            ((f32::floor(position.z) % CHUNK_SIZE as f32) + CHUNK_SIZE as f32) % CHUNK_SIZE as f32;
        let relative_position = glam::vec3(x, f32::max(f32::floor(position.y), 0.0), z);
        let block = chunk.get_block_at_relative(&relative_position)?;
        return Some(block);
    }
    pub fn get_blocks_nearby(&self, player: &Player) -> Option<Vec<Arc<Mutex<Block>>>> {
        let mut positions = vec![];
        let mut nearby_blocks = vec![];
        let offset_vec = glam::vec3(0.4, 0.0, 0.4);

        for i in -2..=2 {
            for j in -2..=2 {
                for h in -2..=2 {
                    positions.push(
                        player.camera.eye + offset_vec + glam::vec3(i as f32, h as f32, j as f32),
                    );
                }
            }
        }
        for position in positions.iter() {
            if let Some(block) = self.get_blocks_absolute(position) {
                nearby_blocks.push(block)
            };
        }

        return Some(nearby_blocks);
    }
    pub fn update(
        &mut self,
        player: &mut Player,
        queue: Arc<wgpu::Queue>,
        device: Arc<wgpu::Device>,
    ) {
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

        let max_threads = thread::available_parallelism().unwrap();
        let threads = usize::max(usize::from(max_threads), 8);
        let thread_pool = ThreadPool::new(threads);
        let (sender, receiver) = mpsc::channel();
        for chunk_x in LB..=UB {
            for chunk_y in LB..=UB {
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
