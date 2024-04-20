use crate::blocks::block_type::BlockType;
use crate::persistence::Saveable;
use crate::utils::{ChunkFromPosition, RelativeFromAbsolute};
use crate::{blocks::block::Block, chunk::Chunk, player::Player, utils::threadpool::ThreadPool};
use glam::Vec3;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::RwLock;
use std::{
    sync::{mpsc, Arc},
    thread,
};

pub const RNG_SEED: u64 = 0;
pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_HEIGHT: u8 = u8::MAX;
pub const NOISE_SIZE: u32 = 1024;
pub const FREQUENCY: f32 = 1. / 128.;
pub const NOISE_CHUNK_PER_ROW: u32 = NOISE_SIZE / CHUNK_SIZE;
pub const MAX_TREES_PER_CHUNK: u32 = 3;
pub const CHUNKS_PER_ROW: u32 = 20;
pub const CHUNKS_REGION: u32 = CHUNKS_PER_ROW * CHUNKS_PER_ROW;
pub const WATER_HEIGHT_LEVEL: u8 = 5;
// Lower bound of chunk
pub const LB: i32 = -((CHUNKS_PER_ROW / 2) as i32);
// Upper bound of chunk
pub const UB: i32 = if CHUNKS_PER_ROW % 2 == 0 {
    (CHUNKS_PER_ROW / 2 - 1) as i32
} else {
    (CHUNKS_PER_ROW / 2) as i32
};

pub type NoiseData = Vec<f32>;
pub type WorldChunk = Arc<RwLock<Chunk>>;
pub type ChunkMap = Arc<RwLock<HashMap<(i32, i32), WorldChunk>>>;

// TODO: It should be better to unsafely pass the hashmap between threads, since we never modify it except when we're done
// and it will be save since every chunk has its own lock.
pub struct World {
    pub chunks: ChunkMap,
    pub thread_pool: Option<ThreadPool>,
    pub seed: u32,
    pub noise_data: Arc<NoiseData>,
    pub chunk_data_layout: Arc<wgpu::BindGroupLayout>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl World {
    pub fn place_block(&mut self, block: Arc<RwLock<Block>>) {
        let block_borrow = block.read().unwrap();
        let mut chunks_to_rerender = vec![block_borrow.get_chunk_coords()];
        chunks_to_rerender.append(&mut block_borrow.get_neighbour_chunks_coords());

        let chunk_map = self.chunks.read().unwrap();
        let chunk = chunk_map
            .get(&chunks_to_rerender[0])
            .expect("Cannot delete a block from unloaded chunk");

        {
            let mut chunk_lock = chunk.write().unwrap();
            chunk_lock.add_block(block.clone(), true);
            // Drop chunk lock write
        }

        self.render_chunks(chunks_to_rerender)
    }
    pub fn remove_block(&mut self, block: Arc<RwLock<Block>>) {
        let mut has_adjacent_water = false;
        let mut chunks_to_rerender = vec![];
        {
            let block_borrow = block.read().unwrap();
            chunks_to_rerender.push(block_borrow.get_chunk_coords());
            chunks_to_rerender.append(&mut block_borrow.get_neighbour_chunks_coords());

            let chunk_map = self.chunks.read().unwrap();
            let chunk = chunk_map
                .get(&chunks_to_rerender[0])
                .expect("Cannot delete a block from unloaded chunk");

            {
                let mut chunk_lock = chunk.write().unwrap();
                chunk_lock.remove_block(&(block_borrow.position));
                // Drop chunk lock write
            }

            for offset in [
                glam::vec3(1.0, 0.0, 0.0),
                glam::vec3(-1.0, 0.0, 0.0),
                glam::vec3(0.0, 0.0, 1.0),
                glam::vec3(0.0, 0.0, -1.0),
            ] {
                let position = block_borrow.absolute_position + offset;
                let chunk_pos = position.get_chunk_from_position_absolute();
                let chunk = chunk_map
                    .get(&chunk_pos)
                    .expect("Should be loaded chunk")
                    .read()
                    .unwrap();

                if chunk.block_type_at(&position.relative_from_absolute()) == Some(BlockType::Water)
                {
                    has_adjacent_water = true;
                }
            }
        }

        // if it has a nearby block of water, replace the removed block with a water block.
        if has_adjacent_water {
            let mut blockbrw = block.write().unwrap();
            blockbrw.block_type = BlockType::Water;
            std::mem::drop(blockbrw);
            self.place_block(block);
        } else {
            self.render_chunks(chunks_to_rerender);
        }
    }
    pub fn get_blocks_absolute(&self, position: &Vec3) -> Option<Arc<RwLock<Block>>> {
        let (chunk_x, chunk_y) = position.get_chunk_from_position_absolute();

        let chunk_map = self.chunks.read().unwrap();
        let chunk = chunk_map.get(&(chunk_x, chunk_y))?;
        let chunk = chunk.read().unwrap();

        let relative_position = position.relative_from_absolute();
        let block = chunk.get_block_at_relative(&relative_position)?;

        return Some(block);
    }
    pub fn get_blocks_nearby(&self, player: Arc<RwLock<Player>>) -> Vec<Arc<RwLock<Block>>> {
        let player = player.read().unwrap();
        let mut positions = vec![];
        let mut nearby_blocks = vec![];

        for i in -5..=5 {
            for j in -5..=5 {
                for h in -5..=5 {
                    positions.push(player.camera.eye + glam::vec3(i as f32, h as f32, j as f32));
                }
            }
        }

        for position in positions.iter() {
            if let Some(block) = self.get_blocks_absolute(position) {
                nearby_blocks.push(block)
            };
        }

        return nearby_blocks;
    }
    pub fn update(
        &mut self,
        player: Arc<RwLock<Player>>,
        queue: Arc<wgpu::Queue>,
        device: Arc<wgpu::Device>,
    ) {
        let mut player_write = player.write().unwrap();
        let current_chunk = player_write.calc_current_chunk();

        // Update loaded chunks based on player position
        if current_chunk != player_write.current_chunk {
            let delta = (
                current_chunk.0 - player_write.current_chunk.0,
                current_chunk.1 - player_write.current_chunk.1,
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
            let chunk_y_remove = player_write.current_chunk.1 + old_chunks_offset;
            let chunk_x_remove = player_write.current_chunk.0 + old_chunks_offset;

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

            let mut keys_to_remove = vec![];
            for key in self.chunks.read().unwrap().keys() {
                // let chunk = chunk.read().unwrap();
                if (delta.1 != 0 && key.1 == chunk_y_remove)
                    || (delta.0 != 0 && key.0 == chunk_x_remove)
                {
                    keys_to_remove.push(key.clone());
                }
            }

            // Save the unloaded chunks
            let (sender, receiver) = mpsc::channel();
            for key in keys_to_remove.iter() {
                let chunk = self
                    .chunks
                    .write()
                    .unwrap()
                    .remove(key)
                    .expect("Something went wrong");
                let sender = sender.clone();
                self.thread_pool.as_ref().unwrap().execute(move || {
                    let chunk = chunk.write().unwrap();
                    if chunk.modified {
                        chunk.save().unwrap();
                    }
                    sender.send(()).unwrap();
                })
            }

            for _ in keys_to_remove.iter() {
                receiver.recv().unwrap();
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
                self.chunks
                    .write()
                    .unwrap()
                    .insert((chunk.x, chunk.y), Arc::new(RwLock::new(chunk)));
            }
            self.handle_outside_blocks();
            // Re-render only the last inserted chunks
            self.render_chunks(new_chunks_positions);
        }

        player_write.current_chunk = current_chunk;
        std::mem::drop(player_write);
        // Update visible chunks based on player position and direction
        {
            let (sender, receiver) = mpsc::channel();
            for chunk in self.chunks.read().unwrap().values() {
                let chunk = Arc::clone(&chunk);
                let sender = sender.clone();
                let player = Arc::clone(&player);
                self.thread_pool.as_ref().unwrap().execute(move || {
                    let mut chunk = chunk.write().unwrap();
                    chunk.visible = chunk.is_visible(player);
                    sender.send(()).unwrap();
                });
            }
            for _ in self.chunks.read().unwrap().iter() {
                receiver.recv().unwrap();
            }
        }
    }
    pub fn dispose(&mut self) {
        self.thread_pool = None;
    }

    pub fn save_state(&self) {
        for chunk in self.chunks.read().unwrap().values() {
            let chunkbrw = chunk.read().unwrap();
            if chunkbrw.modified {
                chunkbrw.save().expect("failed to save");
            }
        }
    }
    pub fn init_chunks(&mut self, player: Arc<RwLock<Player>>) {
        let (sender, receiver) = mpsc::channel();
        let player = player.read().unwrap();

        for chunk_x in LB + player.current_chunk.0..=UB + player.current_chunk.0 {
            for chunk_y in LB + player.current_chunk.1..=UB + player.current_chunk.1 {
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
            self.chunks
                .write()
                .unwrap()
                .insert((chunk.x, chunk.y), Arc::new(RwLock::new(chunk)));
        }

        self.handle_outside_blocks();
        // this is kinda slow
        self.render_chunks(self.chunks.read().unwrap().keys().collect::<Vec<_>>());
    }
    // chunks: slice containing the chunk to re-render
    fn render_chunks<I>(&self, chunk_keys: Vec<I>)
    where
        I: Borrow<(i32, i32)>,
    {
        let (sender, receiver) = mpsc::channel();

        for key in chunk_keys.iter() {
            if let Some(chunk) = self.chunks.read().unwrap().get(key.borrow()) {
                let sender = sender.clone();
                // This is extremely slow O(n^2)
                // let other = self.get_other_chunks(chunk.clone());
                let chunk = chunk.clone();
                let chunk_map = self.chunks.clone();

                self.thread_pool.as_ref().unwrap().execute(move || {
                    let chunk_ptr = chunk.clone();
                    let chunk = chunk.read().unwrap();
                    let res = chunk.build_mesh(chunk_map);
                    sender.send((res, chunk_ptr)).unwrap();
                });
            }
        }
        for _ in chunk_keys.iter() {
            let (
                (
                    indices,
                    water_indices,
                    vertex_buffer,
                    index_buffer,
                    water_vertex_buffer,
                    water_index_buffer,
                ),
                chunk_ptr,
            ) = receiver.recv().expect("Some chunks didn't render");
            let mut chunk_mut = chunk_ptr.write().unwrap();
            chunk_mut.indices = indices;
            chunk_mut.chunk_vertex_buffer = Some(vertex_buffer);
            chunk_mut.chunk_index_buffer = Some(index_buffer);
            chunk_mut.water_indices = water_indices;
            chunk_mut.chunk_water_vertex_buffer = Some(water_vertex_buffer);
            chunk_mut.chunk_water_index_buffer = Some(water_index_buffer);
        }
    }
    fn handle_outside_blocks(&mut self) {
        let mut blocks_to_add = vec![];
        for chunk in self.chunks.read().unwrap().values() {
            let mut chunkbrw = chunk.write().unwrap();
            blocks_to_add.append(&mut chunkbrw.outside_blocks);
        }

        let mut chunks_to_rerender: Vec<WorldChunk> = vec![];

        for block in blocks_to_add.iter() {
            let chunk_coords = block.read().unwrap().get_chunk_coords();
            if let Some(chunkptr) = self.chunks.read().unwrap().get(&chunk_coords) {
                let mut chunkbrw = chunkptr.write().unwrap();
                chunkbrw.add_block(block.clone(), false);
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

        let threads = thread::available_parallelism().unwrap();
        // let threads = usize::max(usize::from(max_threads), 8);
        let thread_pool = ThreadPool::new(usize::from(threads));

        World {
            chunk_data_layout,
            chunks: Arc::new(RwLock::new(HashMap::new())),
            noise_data,
            device,
            queue,
            seed: 0,
            thread_pool: Some(thread_pool),
        }
    }
}
