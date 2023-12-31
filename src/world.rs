use std::{cell::RefCell, rc::Rc};

use glam::{vec3, Vec3};
use rand::random;
use wgpu::{util::DeviceExt, BindGroupLayout, BindGroupLayoutDescriptor};

use crate::{
    model::{InstanceData, Model},
    state::State,
};

const CHUNK_SIZE: u32 = 16;

const NOISE_SIZE: u32 = 1024;
const FREQUENCY: f32 = 1. / 128.;
const NOISE_CHUNK_PER_ROW: u32 = NOISE_SIZE / CHUNK_SIZE;
const WORLD_HEIGHT: u8 = u8::MAX;
// There will be a CHUNKS_PER_ROW * CHUNKS_PER_ROW region
pub const CHUNKS_PER_ROW: u32 = 15;
pub const CHUNKS_REGION: u32 = CHUNKS_PER_ROW * CHUNKS_PER_ROW;

#[derive(Debug)]
pub struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    pub x: i32,
    pub y: i32,
    pub blocks: Vec<BlockData>,
    pub chunk_bind_group: wgpu::BindGroup,
    // pub chunk_position_buffer: wgpu::Buffer,
    pub chunk_data_buffer: wgpu::Buffer,
    pub chunk_position_buffer: wgpu::Buffer,
}

#[derive(Debug)]
pub struct BlockData {
    pub btype: BlockType,
    pub position: [u32; 3],
    pub model: Rc<RefCell<Model>>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum BlockType {
    Grass = 5,
    Dirt = 4,
    Water = 3,
    Wood = 2,
    Leaf = 1,
    Stone = 0,
}

impl BlockType {
    const U_STONE_THRESHOLD: u32 = 20;
    const L_STONE_THRESHOLD: u32 = 1;

    pub fn from_y_position(y: u32) -> BlockType {
        if y > Self::U_STONE_THRESHOLD {
            let t: f32 = random();
            let scaler = (y as f32 - Self::U_STONE_THRESHOLD as f32) / 10.0;
            let res = t + scaler;
            if res > 1.0 {
                BlockType::Stone
            } else {
                BlockType::Dirt
            }
        } else if y < Self::L_STONE_THRESHOLD {
            BlockType::Stone
        } else {
            BlockType::Dirt
        }
    }
}

pub struct World {
    pub chunks: Vec<Chunk>,
    // This would translate to the for now hard coded edge vectors in the pnoise algo
    pub seed: u32,
    pub noise_data: Vec<f32>,
    pub chunk_data_layout: wgpu::BindGroupLayout,
}

impl World {
    pub fn update_current_chunk_buffer(&self, chunk: &Chunk, state: &State) {
        // todo!()
    }
    pub fn init_world(model: Rc<RefCell<Model>>, device: &wgpu::Device) -> Self {
        let noise_data =
            crate::utils::noise::create_world_noise_data(NOISE_SIZE, NOISE_SIZE, FREQUENCY);
        let mut chunks = vec![];

        let lb = (CHUNKS_PER_ROW / 2) as i32;
        let ub = if CHUNKS_PER_ROW % 2 == 0 {
            (CHUNKS_PER_ROW / 2 - 1) as i32
        } else {
            (CHUNKS_PER_ROW / 2) as i32
        };

        let chunk_data_layout = device.create_bind_group_layout(&Chunk::get_bind_group_layout());

        for j in -lb..=ub {
            for i in -lb..=ub {
                chunks.push(Chunk::new(
                    i,
                    j,
                    &noise_data,
                    model.clone(),
                    device,
                    &chunk_data_layout,
                ));
            }
        }

        Self {
            chunk_data_layout,
            chunks,
            noise_data,
            seed: 0,
        }
    }
}

impl Chunk {
    pub fn get_bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("chunk_bind_group"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        }
    }
    pub fn new(
        x: i32,
        y: i32,
        noise_data: &Vec<f32>,
        model: Rc<RefCell<Model>>,
        device: &wgpu::Device,
        chunk_data_layout: &BindGroupLayout,
    ) -> Self {
        let step = 4 as usize;

        let buffer_size =
            ((CHUNK_SIZE as usize * CHUNK_SIZE as usize) * WORLD_HEIGHT as usize) * step;

        // Cpu representation
        let mut blocks: Vec<BlockData> = vec![];
        // Data representation to send to gpu
        let mut block_data: Vec<u32> = vec![0; buffer_size as usize];

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut sample_x = (x * CHUNK_SIZE as i32) + i as i32 % NOISE_SIZE as i32;
                let mut sample_y = (y * CHUNK_SIZE as i32) + j as i32 % NOISE_SIZE as i32;
                // Wrap around if negative chunk coordinate
                if sample_x < 0 {
                    sample_x =
                        NOISE_SIZE as i32 + (sample_x % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
                }
                if sample_y < 0 {
                    sample_y =
                        NOISE_SIZE as i32 + (sample_y % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
                }

                let y_top =
                    (noise_data[((sample_y * NOISE_SIZE as i32) + sample_x) as usize] + 1.0) * 0.5;
                let y_top = (f32::powf(100.0, y_top) - 1.0) as u32;

                for y in 0..=y_top {
                    let mut block_type = match BlockType::from_y_position(y) {
                        BlockType::Dirt if y == y_top => BlockType::Grass,
                        b => b,
                    };

                    block_data[blocks.len() * step + 0] = i;
                    block_data[blocks.len() * step + 1] = y;
                    block_data[blocks.len() * step + 2] = j;
                    block_data[blocks.len() * step + 3] = block_type as u32;
                    blocks.push(BlockData {
                        position: [i, y, j],
                        btype: BlockType::Grass,
                        model: model.clone(),
                    });
                }
            }
        }

        let chunk_position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[x as i32, y as i32]),
            label: Some("chunk_position_buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let chunk_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&block_data),
            label: Some("chunk_data_buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let chunk_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: chunk_data_layout,
            label: None,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: chunk_position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: chunk_data_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            x,
            y,
            blocks,
            chunk_bind_group,
            chunk_data_buffer,
            chunk_position_buffer,
        }
    }
}
