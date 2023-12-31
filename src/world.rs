use std::{cell::RefCell, rc::Rc};

use glam::{vec3, Vec3};
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
    pub cubes: Vec<CubeData>,
    pub chunk_bind_group: wgpu::BindGroup,
    // pub chunk_position_buffer: wgpu::Buffer,
    pub chunk_data_buffer: wgpu::Buffer,
    pub chunk_position_buffer: wgpu::Buffer,
}

#[derive(Debug)]
pub struct CubeData {
    pub ctype: CubeType,
    pub position: [u32; 3],
    pub model: Rc<RefCell<Model>>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum CubeType {
    Empty,
    Dirt,
    Water,
    Wood,
    Stone,
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
        let mut cubes: Vec<CubeData> = vec![];
        // Data representation to send to gpu
        let mut cubes_data: Vec<u32> = vec![0; buffer_size as usize];

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

                let y_offset =
                    (noise_data[((sample_y * NOISE_SIZE as i32) + sample_x) as usize] + 1.0) * 0.5;
                let y_offset = (f32::powf(100.0, y_offset)) as u32;

                for y in 0..=y_offset {
                    cubes_data[cubes.len() * step + 0] = i;
                    cubes_data[cubes.len() * step + 1] = y;
                    cubes_data[cubes.len() * step + 2] = j;
                    cubes_data[cubes.len() * step + 3] = CubeType::Dirt as u32;
                    cubes.push(CubeData {
                        position: [i, y, j],
                        ctype: CubeType::Dirt,
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
            contents: bytemuck::cast_slice(&cubes_data),
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
            cubes,
            chunk_bind_group,
            chunk_data_buffer,
            chunk_position_buffer,
        }
    }
}
