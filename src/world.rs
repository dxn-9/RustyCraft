use glam::Vec3;
use std::ops::{Deref, DerefMut};
use std::sync::RwLock;
use std::{
    sync::{mpsc, Arc},
    thread,
};

use crate::persistance::Saveable;
use crate::utils::{ChunkFromPosition, RelativeFromAbsolute};
use crate::{blocks::block::Block, chunk::Chunk, player::Player, utils::threadpool::ThreadPool};

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_HEIGHT: u8 = u8::MAX;
pub const NOISE_SIZE: u32 = 1024;
pub const FREQUENCY: f32 = 1. / 128.;
pub const NOISE_CHUNK_PER_ROW: u32 = NOISE_SIZE / CHUNK_SIZE;
pub const MAX_TREES_PER_CHUNK: u32 = 3;
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

pub type WorldChunk = Arc<RwLock<Chunk>>;
pub struct World {
    pub chunks: Vec<WorldChunk>,
    pub thread_pool: Option<ThreadPool>,
    pub seed: u32,
    pub noise_data: Arc<NoiseData>,
    pub chunk_data_layout: Arc<wgpu::BindGroupLayout>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl World {
    // gets all the chunks except the one passed in the index
    pub fn get_other_chunks_by_index(&self, chunk_index: usize) -> Vec<WorldChunk> {
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
    // gets all the chunks except the one passed
    pub fn get_other_chunks(&self, chunk_ptr: WorldChunk) -> Vec<WorldChunk> {
        self.chunks
            .iter()
            .filter_map(|c| {
                return if !Arc::ptr_eq(&chunk_ptr, c) {
                    Some(c.clone())
                } else {
                    None
                };
            })
            .collect()
    }
    pub fn place_block(&mut self, block: Arc<RwLock<Block>>) {
        let mut chunks_to_rerender = vec![];

        let block_borrow = block.read().unwrap();
        let chunk_coords = block_borrow.get_chunk_coords();
        let chunk = self
            .chunks
            .iter()
            .find(|c| {
                let c = c.read().unwrap();
                c.x == chunk_coords.0 && c.y == chunk_coords.1
            })
            .expect("Cannot delete a block from unloaded chunk");

        chunks_to_rerender.push(chunk.clone());
        std::mem::drop(block_borrow);
        let mut chunk_lock = chunk.write().unwrap();
        chunk_lock.add_block(block.clone());

        let block_borrow = block.read().unwrap();
        let block_neighbour_chunks = block_borrow.get_neighbour_chunks_coords();
        std::mem::drop(chunk_lock);

        if block_neighbour_chunks.len() > 0 {
            for neighbour_chunk in block_neighbour_chunks {
                let neighbour_chunk = self
                    .chunks
                    .iter()
                    .find(|o| {
                        let c = o.read().unwrap();
                        return c.x == neighbour_chunk.0 && c.y == neighbour_chunk.1;
                    })
                    .expect("Cannot destroy a block without neighbour being loaded");

                chunks_to_rerender.push(neighbour_chunk.clone());
            }
        }

        self.render_chunks(&chunks_to_rerender);
    }
    pub fn remove_block(&mut self, block: Arc<RwLock<Block>>) {
        let mut chunks_to_rerender = vec![];

        let block_borrow = block.read().unwrap();
        let chunk_coords = block_borrow.get_chunk_coords();
        let chunk = self
            .chunks
            .iter()
            .find(|c| {
                let c = c.read().unwrap();
                c.x == chunk_coords.0 && c.y == chunk_coords.1
            })
            .expect("Cannot delete a block from unloaded chunk");

        let mut chunk_lock = chunk.write().unwrap();
        chunk_lock.remove_block(&(block_borrow.position));
        chunks_to_rerender.push(chunk.clone());
        // chunk_lock.build_mesh(self.get_other_chunks(chunk.clone()));
        let block_neighbour_chunks = block_borrow.get_neighbour_chunks_coords();
        // I hate this so much
        std::mem::drop(chunk_lock);

        if block_neighbour_chunks.len() > 0 {
            for neighbour_chunk in block_neighbour_chunks {
                let neighbour_chunk = self
                    .chunks
                    .iter()
                    .find(|o| {
                        let c = o.read().unwrap();
                        c.x == neighbour_chunk.0 && c.y == neighbour_chunk.1
                    })
                    .expect("Cannot destroy a block without neighbour being loaded");

                chunks_to_rerender.push(neighbour_chunk.clone());
            }
        }
        self.render_chunks(&chunks_to_rerender);
    }
    pub fn get_blocks_absolute(&self, position: &Vec3) -> Option<Arc<RwLock<Block>>> {
        let (chunk_x, chunk_y) = position.get_chunk_from_position_absolute();

        let chunk = self.chunks.iter().find(|c| {
            let c = c.read().unwrap();
            c.x == chunk_x && c.y == chunk_y
        })?;
        let chunk = chunk.read().unwrap();

        let relative_position = position.relative_from_absolute();
        let block = chunk.get_block_at_relative(&relative_position)?;

        return Some(block);
    }
    pub fn get_blocks_nearby(&self, player: &Player) -> Option<Vec<Arc<RwLock<Block>>>> {
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
                let chunk = chunk.read().unwrap();
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

                self.thread_pool.as_ref().unwrap().execute(move || {
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
                self.chunks.push(Arc::new(RwLock::new(chunk)));
            }
            self.handle_outside_blocks();
            self.render_chunks(&self.chunks[self.chunks.len() - chunks_added..]);
            // Re-render only the last inserted chunks
        }

        player.current_chunk = current_chunk;
    }
    pub fn dispose(&mut self) {
        self.thread_pool = None;
    }
    pub fn save_state(&self) {
        for chunk in self.chunks.iter() {
            let chunkbrw = chunk.read().unwrap();
            chunkbrw.save().expect("failed to save");
        }
    }
    pub fn init_chunks(&mut self) {
        let (sender, receiver) = mpsc::channel();

        let mut chunks = vec![];
        for chunk_x in LB..=UB {
            for chunk_y in LB..=UB {
                let sender = sender.clone();
                let noise_data = Arc::clone(&self.noise_data);
                let chunk_data_layout = Arc::clone(&self.chunk_data_layout);
                let device = Arc::clone(&self.device);
                let queue = Arc::clone(&self.queue);

                self.thread_pool.as_ref().unwrap().execute(move || {
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

        for _ in 0..CHUNKS_PER_ROW * CHUNKS_PER_ROW {
            let chunk = receiver.recv().expect("Some chunks are missing");
            chunks.push(Arc::new(RwLock::new(chunk)));
        }
        self.chunks.append(&mut chunks); // Add chunks to self

        self.handle_outside_blocks();
        self.render_chunks(&self.chunks);
    }
    // chunks: slice containing the chunk to re-render
    fn render_chunks(&self, chunks: &[WorldChunk]) {
        let (sender, receiver) = mpsc::channel();

        for chunk in chunks.iter() {
            let sender = sender.clone();
            let other = self.get_other_chunks(chunk.clone());
            let chunk = chunk.clone();

            self.thread_pool.as_ref().unwrap().execute(move || {
                let chunk_ptr = chunk.clone();
                let chunk = chunk.read().unwrap();
                let res = chunk.build_mesh(other);
                sender.send((res, chunk_ptr)).unwrap();
            });
        }
        for _ in chunks.iter() {
            let ((indices, vertex_buffer, index_buffer), chunk_ptr) =
                receiver.recv().expect("Some chunks didn't render");
            let mut chunk_mut = chunk_ptr.write().unwrap();
            chunk_mut.indices = indices;
            chunk_mut.chunk_vertex_buffer = Some(vertex_buffer);
            chunk_mut.chunk_index_buffer = Some(index_buffer);
        }
    }
    fn handle_outside_blocks(&mut self) {
        let mut blocks_to_add = vec![];
        for chunk in self.chunks.iter() {
            let mut chunkbrw = chunk.write().unwrap();
            blocks_to_add.append(&mut chunkbrw.outside_blocks);
        }

        let mut chunks_to_rerender: Vec<WorldChunk> = vec![];

        for block in blocks_to_add.iter() {
            let chunk_coords = block.read().unwrap().get_chunk_coords();
            if let Some(chunkptr) = self.chunks.iter().find(|c| {
                let c = c.read().unwrap();
                c.x == chunk_coords.0 && c.y == chunk_coords.1
            }) {
                let mut chunkbrw = chunkptr.write().unwrap();
                chunkbrw.add_block(block.clone());
                if let None = chunks_to_rerender.iter().find(|c| Arc::ptr_eq(c, chunkptr)) {
                    chunks_to_rerender.push(chunkptr.clone());
                };
            }
        }
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
            thread_pool: Some(thread_pool),
        }
    }
}
