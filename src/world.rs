use std::{
    borrow::Borrow,
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
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
};

pub const CHUNK_SIZE: u32 = 16;
pub const CHUNK_HEIGHT: u8 = u8::MAX;
pub const NOISE_SIZE: u32 = 1024;
pub const FREQUENCY: f32 = 1. / 128.;
pub const NOISE_CHUNK_PER_ROW: u32 = NOISE_SIZE / CHUNK_SIZE;
// There will be a CHUNKS_PER_ROW * CHUNKS_PER_ROW region
pub const CHUNKS_PER_ROW: u32 = 3;

pub const CHUNKS_REGION: u32 = CHUNKS_PER_ROW * CHUNKS_PER_ROW;

// Lower bound of chunk
pub const LB: i32 = -((CHUNKS_PER_ROW / 2) as i32);
// Upper boound of chunk
pub const UB: i32 = if CHUNKS_PER_ROW % 2 == 0 {
    (CHUNKS_PER_ROW / 2 - 1) as i32
} else {
    (CHUNKS_PER_ROW / 2) as i32
};

type BlockMap = HashMap<i32, HashMap<i32, HashMap<i32, Rc<RefCell<Block>>>>>;
type BlockVec = Vec<Rc<RefCell<Block>>>;
pub struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    pub x: i32,
    pub y: i32,
    pub blocks: BlockVec,
    pub blocks_map: BlockMap,
    pub indices: u32,
    pub chunk_bind_group: wgpu::BindGroup,
    pub chunk_position_buffer: wgpu::Buffer,
    // pub chunk_vertex_buffer: wgpu::Buffer,
    pub chunk_index_buffer: wgpu::Buffer,
    pub chunk_vertex_buffer: wgpu::Buffer,
}

pub struct World {
    pub chunks: Vec<Chunk>,
    pub seed: u32,
    pub noise_data: Vec<f32>,
    pub chunk_data_layout: wgpu::BindGroupLayout,
}

impl World {
    pub fn update(&mut self, player: &mut Player, queue: &wgpu::Queue) {
        // Check if the player has moved to a new chunk, if so, generate the new chunks
        let current_chunk = player.calc_current_chunk();
        if current_chunk != player.current_chunk {
            // Player has moved to a new chunk, we will transfer resources from the chunks out of player's range to the new chunks
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

            // Remove the chunks
            let mut old_chunks: Vec<Chunk> = vec![];
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
                        old_chunks.push(self.chunks.remove(i));
                    } else {
                        break;
                    }
                }
            }
            let chunks_added = old_chunks.len();
            while let Some(chunk) = old_chunks.pop() {
                let new_chunk_pos = new_chunks_positions.pop().unwrap();
                let new_chunk = Chunk::from_chunk(
                    chunk,
                    new_chunk_pos.0,
                    new_chunk_pos.1,
                    &self.noise_data,
                    queue,
                );
                self.chunks.push(new_chunk);
            }

            let mut indices_added: Vec<u32> = vec![];
            // Since the chunks at the end of the chunks vector are the new one, we will update those
            for i in 0..chunks_added {
                let new_chunk = &self.chunks[self.chunks.len() - 1 - i as usize];
                indices_added.push(new_chunk.build_mesh(queue, &self));
            }
            for i in 0..chunks_added {
                let len = self.chunks.len();
                let new_chunk = &mut self.chunks[len - 1 - i as usize];
                new_chunk.indices = indices_added[i as usize];
            }

            player.current_chunk = current_chunk;
        }
    }
    pub fn init_world(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let noise_data =
            crate::utils::noise::create_world_noise_data(NOISE_SIZE, NOISE_SIZE, FREQUENCY);
        let mut chunks = vec![];

        let chunk_data_layout = device.create_bind_group_layout(&Chunk::get_bind_group_layout());

        for j in LB..=UB {
            for i in LB..=UB {
                chunks.push(Chunk::new(i, j, &noise_data, device, &chunk_data_layout));
            }
        }

        let mut world = Self {
            chunk_data_layout,
            chunks,
            noise_data,
            seed: 0,
        };

        let mut indices_added: Vec<u32> = vec![];
        for chunk in world.chunks.iter() {
            indices_added.push(chunk.build_mesh(queue, &world));
        }
        // borrow checker workaround :\
        for (i, chunk) in world.chunks.iter_mut().enumerate() {
            chunk.indices = indices_added[i];
        }

        return world;
    }
}

impl Chunk {
    pub fn from_chunk(
        chunk: Chunk,
        x: i32,
        y: i32,
        noise_data: &Vec<f32>,
        queue: &wgpu::Queue,
    ) -> Chunk {
        println!(
            "[INFO] Creating new chunk {} {} from {} {}",
            x, y, chunk.x, chunk.y
        );
        let (blocks, blocks_map) = Chunk::create_blocks_data(x, y, noise_data);

        queue.write_buffer(
            &chunk.chunk_position_buffer,
            0,
            bytemuck::cast_slice(&[x, y]),
        );
        Chunk {
            blocks,
            blocks_map,
            x,
            y,
            chunk_bind_group: chunk.chunk_bind_group,
            chunk_index_buffer: chunk.chunk_index_buffer,
            chunk_position_buffer: chunk.chunk_position_buffer,
            chunk_vertex_buffer: chunk.chunk_vertex_buffer,
            indices: 0,
        }
    }
    pub fn exists_block_at(&self, position: &glam::Vec3) -> bool {
        match self.blocks_map.get(&(position.y as i32)) {
            Some(x_map) => match x_map.get(&(position.x as i32)) {
                Some(z_map) => match z_map.get(&(position.z as i32)) {
                    Some(_) => true,
                    None => false,
                },
                None => false,
            },
            None => false,
        }
    }
    // TODO: this probably can be removed
    pub fn build_blocks_map(blocks: &BlockVec) -> BlockMap {
        let mut blocks_map: BlockMap = HashMap::new();
        for block in blocks.iter() {
            let blockbrw = block.as_ref().borrow();
            let x_map = match blocks_map.get_mut(&(blockbrw.position.y as i32)) {
                Some(x_map) => x_map,
                None => {
                    blocks_map.insert(blockbrw.position.y as i32, HashMap::new());
                    blocks_map.get_mut(&(blockbrw.position.y as i32)).unwrap()
                }
            };
            let z_map = match x_map.get_mut(&(blockbrw.position.x as i32)) {
                Some(z_map) => z_map,
                None => {
                    x_map.insert(blockbrw.position.x as i32, HashMap::new());
                    x_map.get_mut(&(blockbrw.position.x as i32)).unwrap()
                }
            };
            match z_map.get_mut(&(blockbrw.position.z as i32)) {
                Some(_) => panic!("Cannot have more than 1 block in the same place"),
                None => {
                    z_map.insert(blockbrw.position.z as i32, block.clone());
                }
            }
        }
        blocks_map
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
    pub fn build_mesh(&self, queue: &wgpu::Queue, world: &World) -> u32 {
        let mut vertex: Vec<BlockVertexData> = vec![];
        let mut indices: Vec<u32> = vec![];

        for block in self.blocks.iter() {
            {
                let blockbrw = block.as_ref().borrow();
                let cube_pos = blockbrw.position.clone();
                let faces = blockbrw.faces.as_ref().unwrap();

                for face in faces.iter() {
                    // Check if each face is visible and if so, add it to the mesh
                    let mut is_visible = true;
                    let face_chunk_pos = face.face_direction.get_normal_vector() + cube_pos;

                    if Chunk::is_outside_bounds(&face_chunk_pos)
                        || self.exists_block_at(&face_chunk_pos)
                    {
                        is_visible = false
                    } else {
                        if Chunk::is_outside_chunk(&face_chunk_pos) {
                            let target_chunk_y =
                                self.y + (f32::floor(face_chunk_pos.z / CHUNK_SIZE as f32) as i32);
                            let target_chunk_x =
                                self.x + (f32::floor(face_chunk_pos.x / CHUNK_SIZE as f32) as i32);

                            let target_chunk_block = glam::vec3(
                                (face_chunk_pos.x + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                                face_chunk_pos.y,
                                (face_chunk_pos.z + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                            );
                            let target_chunk = world.chunks.iter().find(|chunk| {
                                chunk.x == target_chunk_x && chunk.y == target_chunk_y
                            });
                            match target_chunk {
                                Some(target_chunk) => {
                                    if target_chunk.exists_block_at(&target_chunk_block) {
                                        is_visible = false;
                                    }
                                }
                                None => {
                                    let sample_x = (target_chunk_x * CHUNK_SIZE as i32)
                                        + target_chunk_block.x as i32 % NOISE_SIZE as i32;
                                    let sample_z = (target_chunk_y * CHUNK_SIZE as i32)
                                        + target_chunk_block.z as i32 % NOISE_SIZE as i32;
                                    let sample_y = target_chunk_block.y as i32;

                                    is_visible = !exists_block_at(
                                        sample_x,
                                        sample_y,
                                        sample_z,
                                        &world.noise_data,
                                    );
                                }
                            }
                        }
                    }

                    if is_visible {
                        let (mut vertex_data, mut index_data) = face.create_face_data();
                        vertex.append(&mut vertex_data);
                        let indices_offset = (vertex.len() - 4) as u32;
                        indices.append(&mut index_data.iter().map(|i| i + indices_offset).collect())
                    }
                }
            }
        }

        queue.write_buffer(&self.chunk_vertex_buffer, 0, bytemuck::cast_slice(&vertex));
        queue.write_buffer(&self.chunk_index_buffer, 0, bytemuck::cast_slice(&indices));

        indices.len() as u32
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

    pub fn create_blocks_data(x: i32, y: i32, noise_data: &Vec<f32>) -> (BlockVec, BlockMap) {
        let mut blocks: BlockVec = vec![];

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let sample_x = (x * CHUNK_SIZE as i32) + i as i32 % NOISE_SIZE as i32;
                let sample_z = (y * CHUNK_SIZE as i32) + j as i32 % NOISE_SIZE as i32;

                let y_top = height_from_coords(sample_x, sample_z, noise_data);

                for y in 0..=y_top {
                    let block_type = match BlockType::from_y_position(y) {
                        BlockType::Dirt(..) if y == y_top => BlockType::grass(),
                        b => b,
                    };

                    let block = Rc::new(RefCell::new(Block {
                        faces: None,
                        position: glam::vec3(i as f32, y as f32, j as f32),
                        block_type,
                        is_translucent: false,
                    }));

                    let face_directions = FaceDirections::all()
                        .iter()
                        .map(|face_dir| BlockFace {
                            block: block.clone(),
                            face_direction: *face_dir,
                        })
                        .collect::<Vec<_>>();
                    block.borrow_mut().faces = Some(face_directions);
                    blocks.push(block)
                }
            }
        }
        let blocks_map = Self::build_blocks_map(&blocks);
        (blocks, blocks_map)
    }

    pub fn new(
        x: i32,
        y: i32,
        noise_data: &Vec<f32>,
        device: &wgpu::Device,
        chunk_data_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let (blocks, blocks_map) = Self::create_blocks_data(x, y, noise_data);

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
            layout: chunk_data_layout,
            label: Some(&format!("chunk-bg-{x}-{y}")),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: chunk_position_buffer.as_entire_binding(),
            }],
        });

        Self {
            x,
            y,
            chunk_bind_group,
            chunk_vertex_buffer,
            chunk_position_buffer,
            chunk_index_buffer,
            // chunk_vertex_buffer,
            blocks,
            blocks_map,
            indices: 0,
        }
    }
}

pub fn exists_block_at(x: i32, y: i32, z: i32, noise_data: &Vec<f32>) -> bool {
    let toph = height_from_coords(x, z, noise_data);
    return y <= toph as i32;
}
// X and Y are absolute values, meaning they if it belongs to chunk (1, 1) it will be (16+, 16+)
// Returns the heightmap value for a given coordinate
pub fn height_from_coords(mut x: i32, mut z: i32, noise_data: &Vec<f32>) -> u32 {
    if x < 0 {
        x = NOISE_SIZE as i32 + (x % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
    }
    if z < 0 {
        z = NOISE_SIZE as i32 + (z % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
    }

    let y_top = (noise_data[((z * NOISE_SIZE as i32) + x) as usize] + 1.0) * 0.5;
    return (f32::powf(100.0, y_top) - 1.0) as u32;
}
