use std::any::Any;
use std::ffi::c_void;
use std::ops::Deref;
use std::sync::Weak;
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};
use wgpu::util::DeviceExt;

use crate::utils::{ChunkFromPosition, RelativeFromAbsolute};
use crate::{blocks::block::Block, chunk::Chunk, player::Player, utils::threadpool::ThreadPool};

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
    pub chunks: Vec<Arc<Mutex<Chunk>>>,
    pub thread_pool: ThreadPool,
    pub seed: u32,
    pub noise_data: Arc<NoiseData>,
    pub chunk_data_layout: Arc<wgpu::BindGroupLayout>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl World {
    // gets all the chunks except the one passed in the index
    pub fn get_other_chunks(&self, chunk_index: usize) -> Vec<Arc<Mutex<Chunk>>> {
        self.chunks
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                return if i != chunk_index {
                    Some(c.clone())
                } else {
                    None
                };
            })
            .collect()
    }
    pub fn place_block(&mut self, block: Arc<Mutex<Block>>) {
        let block_borrow = block.lock().unwrap();
        let chunk_coords = block_borrow.get_chunk_coords();
        let (chunk_index, chunk) = self
            .chunks
            .iter()
            .enumerate()
            .find(|(i, c)| {
                let c = c.lock().unwrap();
                return c.x == chunk_coords.0 && c.y == chunk_coords.1;
            })
            .expect("Cannot delete a block from unloaded chunk");

        let mut chunk_lock = chunk.lock().unwrap();
        std::mem::drop(block_borrow);
        chunk_lock.add_block(block.clone());
        chunk_lock.build_mesh(self.get_other_chunks(chunk_index));

        let block_borrow = block.lock().unwrap();
        let block_neighbour_chunks = block_borrow.get_neighbour_chunks_coords();
        std::mem::drop(chunk_lock);

        if block_neighbour_chunks.len() > 0 {
            for neighbour_chunk in block_neighbour_chunks {
                let (neighbour_index, neighbour_chunk) = self
                    .chunks
                    .iter()
                    .enumerate()
                    .find_map(|(i, o)| {
                        let c = o.lock().unwrap();
                        return if c.x == neighbour_chunk.0 && c.y == neighbour_chunk.1 {
                            Some((i, o))
                        } else {
                            None
                        };
                    })
                    .expect("Cannot destroy a block without neighbour being loaded");

                let mut neighbour_chunk = neighbour_chunk.lock().unwrap();

                neighbour_chunk.build_mesh(self.get_other_chunks(neighbour_index));
            }
        }

        println!("2");
    }
    pub fn remove_block(&mut self, block: Arc<Mutex<Block>>) {
        let block_borrow = block.lock().unwrap();
        let chunk_coords = block_borrow.get_chunk_coords();
        let (chunk_index, chunk) = self
            .chunks
            .iter()
            .enumerate()
            .find(|(i, c)| {
                let c = c.lock().unwrap();
                return c.x == chunk_coords.0 && c.y == chunk_coords.1;
            })
            .expect("Cannot delete a block from unloaded chunk");

        let mut chunk_lock = chunk.lock().unwrap();
        chunk_lock.remove_block(&(block_borrow.position));

        chunk_lock.build_mesh(self.get_other_chunks(chunk_index));
        let block_neighbour_chunks = block_borrow.get_neighbour_chunks_coords();

        // I hate this so much
        std::mem::drop(chunk_lock);

        if block_neighbour_chunks.len() > 0 {
            for neighbour_chunk in block_neighbour_chunks {
                let (neighbour_index, neighbour_chunk) = self
                    .chunks
                    .iter()
                    .enumerate()
                    .find_map(|(i, o)| {
                        let c = o.lock().unwrap();
                        return if c.x == neighbour_chunk.0 && c.y == neighbour_chunk.1 {
                            Some((i, o))
                        } else {
                            None
                        };
                    })
                    .expect("Cannot destroy a block without neighbour being loaded");

                let mut neighbour_chunk = neighbour_chunk.lock().unwrap();

                neighbour_chunk.build_mesh(self.get_other_chunks(neighbour_index));
            }
        }
    }
    pub fn get_blocks_absolute(&self, position: &glam::Vec3) -> Option<Arc<Mutex<Block>>> {
        let (chunk_x, chunk_y) = position.get_chunk_from_position_absolute();

        let chunk = self.chunks.iter().find(|c| {
            let c = c.lock().unwrap();
            return c.x == chunk_x && c.y == chunk_y;
        })?;
        let chunk = chunk.lock().unwrap();

        let relative_position = position.relative_from_absolute();
        let block = chunk.get_block_at_relative(&relative_position)?;
        return Some(block);
    }
    pub fn get_blocks_nearby(&self, player: &Player) -> Option<Vec<Arc<Mutex<Block>>>> {
        let mut positions = vec![];
        let mut nearby_blocks = vec![];

        for i in -10..=10 {
            for j in -10..=10 {
                for h in -10..=10 {
                    positions.push(player.camera.eye + glam::vec3(i as f32, h as f32, j as f32));
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

            let mut indices_to_remove: Vec<usize> = vec![];
            for (i, chunk) in self.chunks.iter().enumerate() {
                let chunk = chunk.lock().unwrap();
                if (delta.1 != 0 && chunk.y == chunk_y_remove)
                    || (delta.0 != 0 && chunk.x == chunk_x_remove)
                {
                    indices_to_remove.push(i);
                }
            }

            for (o, index) in indices_to_remove.iter().enumerate() {
                self.chunks.remove(index - o);
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
                let other_chunks = self.chunks.iter().map(|c| c.clone()).collect::<Vec<_>>();

                self.thread_pool.execute(move || {
                    let chunk = Chunk::new(
                        new_chunk_pos.0,
                        new_chunk_pos.1,
                        noise_data,
                        device,
                        queue,
                        chunk_data_layout,
                        other_chunks,
                    );
                    sender.send(chunk).unwrap()
                })
            }

            for _ in 0..chunks_added {
                let chunk = receiver.recv().unwrap();
                self.chunks.push(Arc::new(Mutex::new(chunk)));
            }
        }

        player.current_chunk = current_chunk;
    }
    pub fn init_chunks(&mut self) {
        let (sender, receiver) = mpsc::channel();

        for chunk_x in LB..=UB {
            for chunk_y in LB..=UB {
                let sender = sender.clone();
                let noise_data = Arc::clone(&self.noise_data);
                let chunk_data_layout = Arc::clone(&self.chunk_data_layout);
                let device = Arc::clone(&self.device);
                let queue = Arc::clone(&self.queue);

                self.thread_pool.execute(move || {
                    let chunk = Chunk::new(
                        chunk_x,
                        chunk_y,
                        noise_data,
                        device,
                        queue,
                        chunk_data_layout,
                        vec![],
                    );
                    sender.send(chunk).unwrap();
                });
            }
        }
        println!("END CHUNK");

        let mut chunks = vec![];
        for _ in 0..CHUNKS_PER_ROW * CHUNKS_PER_ROW {
            let chunk = receiver.recv().unwrap();
            chunks.push(Arc::new(Mutex::new(chunk)));
        }
        self.chunks.append(&mut chunks);
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

        World {
            chunk_data_layout,
            chunks: vec![],
            noise_data,
            device,
            queue,
            seed: 0,
            thread_pool,
        }
    }
}
