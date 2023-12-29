use std::{cell::RefCell, rc::Rc};

use glam::{vec3, Vec3};
use wgpu::{util::DeviceExt, BindGroupLayout, BindGroupLayoutDescriptor};

use crate::{
    model::{InstanceData, Model},
    state::State,
};

const CHUNK_SIZE: u32 = 16;

const NOISE_SIZE: u32 = 526;
const FREQUENCY: f32 = 1. / 64.;
const CHUNK_PER_ROW: u32 = NOISE_SIZE / CHUNK_SIZE;
const WORLD_HEIGHT: u8 = u8::MAX;

pub const LOADED_CHUNK: u32 = 9;

#[derive(Debug)]
pub struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    pub x: i32,
    pub y: i32,
    pub cubes: Vec<CubeData>,
    pub chunk_bind_group: wgpu::BindGroup,
    pub current_chunk_offset_buffer: wgpu::Buffer,
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
    pub chunks_buffer: wgpu::Buffer,
    pub current_chunk_bind_group_layout: wgpu::BindGroupLayout,
}

impl World {
    pub fn update_current_chunk_buffer(&self, chunk: &Chunk, state: &State) {
        // println!("CHUNK {} {}", chunk.x, chunk.y);
        // state.queue.write_buffer(
        //     &self.current_chunk_offset_buffer,
        //     0,
        //     bytemuck::cast_slice(&[chunk.x, chunk.y]),
        // );
    }
    pub fn init_world(model: Rc<RefCell<Model>>, device: &wgpu::Device) -> Self {
        let noise_data =
            crate::utils::noise::create_perlin_noise_data(NOISE_SIZE, NOISE_SIZE, FREQUENCY);
        let mut chunks = vec![];

        // TODO: check if this works also work for non easy to sqrt nums (such as 9)
        let offset = ((f32::sqrt(LOADED_CHUNK as f32)) / 2.0) as i32;

        let current_chunk_bind_group_layout =
            device.create_bind_group_layout(&Chunk::get_bind_group_layout());

        for j in -offset..=offset {
            for i in -offset..=offset {
                chunks.push(Chunk::new(
                    i,
                    j,
                    &noise_data,
                    model.clone(),
                    device,
                    &current_chunk_bind_group_layout,
                ));
            }
        }

        // The idea is to create a storage buffer to store blocks data, each chunk will get it's own drawcall
        // - every instance of a block in a chunk has a 16 byte data [x:u32][y:u32][z:u32][block_type:u32]
        // - this also means that placing a block is simply a matter of adding a instance and updating the buffer at that point

        let chunk_region = ((CHUNK_SIZE as u64 * CHUNK_SIZE as u64) * WORLD_HEIGHT as u64) * 4 * 4;
        let size = chunk_region * LOADED_CHUNK as u64;

        let mut buffer: Vec<u32> = vec![0; size as usize];

        for j in 0..3 {
            for i in 0..3 {
                let chunk = &chunks[(j * 3) + i];
                let mem = ((j * 3) + i) * (chunk_region as usize);
                println!("CHUNGUS {} {} - MEM {}", chunk.x, chunk.y, mem);
                for (cube_index, cube) in chunk.cubes.iter().enumerate() {
                    buffer[(mem + cube_index * 4) + 0] = cube.position[0] as u32;
                    buffer[(mem + cube_index * 4) + 1] = cube.position[1] as u32;
                    buffer[(mem + cube_index * 4) + 2] = cube.position[2] as u32;
                    buffer[(mem + cube_index * 4) + 3] = cube.ctype as u32;
                }
            }
        }

        let chunks_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&buffer),
            label: Some("chunks_buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            current_chunk_bind_group_layout,
            chunks,
            chunks_buffer,
            noise_data,
            seed: 0,
        }
    }
}

impl Chunk {
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
    pub fn new(
        x: i32,
        y: i32,
        noise_data: &Vec<f32>,
        model: Rc<RefCell<Model>>,
        device: &wgpu::Device,
        layout: &BindGroupLayout,
    ) -> Self {
        let mut cubes: Vec<CubeData> = vec![];

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut sample_x = (CHUNK_SIZE as i32 * x + i as i32) % NOISE_SIZE as i32;
                let mut sample_y = (CHUNK_SIZE as i32 * y + j as i32) % NOISE_SIZE as i32;
                // Wrap around if negative chunk coordinate
                if sample_x < 0 {
                    sample_x = NOISE_SIZE as i32 + (sample_x % CHUNK_PER_ROW as i32);
                }
                if sample_y < 0 {
                    sample_y = NOISE_SIZE as i32 + (sample_y % CHUNK_PER_ROW as i32);
                }

                let y_offset =
                    (((noise_data[((sample_y * NOISE_SIZE as i32) + sample_x) as usize] + 1.0)
                        * 0.5)
                        * 10.0) as u32;
                for y in 0..=y_offset {
                    cubes.push(CubeData {
                        position: [i, y, j],
                        ctype: CubeType::Dirt,
                        model: model.clone(),
                    });
                    model.borrow_mut().instances += 1;
                }
            }
        }

        // TODO: Update name
        let current_chunk_offset_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&[x as i32, y as i32]),
                label: Some("current_chunk_buffer"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let chunk_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            label: None,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                // bind the first, if it changes per mesh we will update the bind group later
                resource: current_chunk_offset_buffer.as_entire_binding(),
            }],
        });

        Self {
            x,
            y,
            cubes,
            current_chunk_offset_buffer,
            chunk_bind_group,
        }
    }
}
