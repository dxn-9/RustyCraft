use std::{cell::RefCell, rc::Rc};

use glam::{vec3, Vec3};
use wgpu::util::DeviceExt;

use crate::{
    model::{InstanceData, Model},
    state::State,
};

const CHUNK_SIZE: u32 = 16;

const NOISE_WIDTH: u32 = 512;
const NOISE_HEIGHT: u32 = 512;
const FREQUENCY: f32 = 0.25;
const CHUNK_PER_ROW: u32 = NOISE_WIDTH / CHUNK_SIZE;

pub struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    pub x: i32,
    pub y: i32,
    pub cubes: Vec<CubeData>,
}

pub struct CubeData {
    pub ctype: CubeType,
    pub position: Vec3,
    pub model: Rc<RefCell<Model>>,
}

pub enum CubeType {
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
}

impl World {
    pub fn init_world(model: Rc<RefCell<Model>>, state: &State) -> Self {
        let mut world = Self {
            chunks: vec![],
            seed: 0,
            noise_data: crate::utils::noise::create_perlin_noise_data(128, 128, 1. / 64.),
        };
        // let first_chunk = Chunk::new(0, 0, &world, model.clone(), state);
        // let second = Chunk::new(1, 0, &world, model.clone(), state);
        // let third = Chunk::new(-1, 0, &world, model.clone(), state);
        // let fourth = Chunk::new(-1, 1, &world, model.clone(), state);
        // world.chunks.push(first_chunk);
        // world.chunks.push(second);
        // world.chunks.push(third);
        // world.chunks.push(fourth);
        for i in -1..=1 {
            for j in -1..=1 {
                world
                    .chunks
                    .push(Chunk::new(i, j, &world, model.clone(), state));
            }
        }

        world
    }
}

impl Chunk {
    pub fn new(x: i32, y: i32, world: &World, model: Rc<RefCell<Model>>, state: &State) -> Self {
        let mut cubes: Vec<CubeData> = vec![];
        let x = x * CHUNK_SIZE as i32;
        let y = y * CHUNK_SIZE as i32;

        for i in x..x + CHUNK_SIZE as i32 {
            for j in y..y + CHUNK_SIZE as i32 {
                let mut sample_i = i;
                if i < 0 {
                    sample_i = i32::abs(i * NOISE_WIDTH as i32);
                }

                cubes.push(CubeData {
                    position: vec3((i * 2) as f32, (j * 2) as f32, 0.0),
                    ctype: CubeType::Dirt,
                    model: model.clone(),
                });
            }
        }

        let mut model_m = model.borrow_mut();

        let previous_size = std::mem::size_of_val(&model_m.instances);

        model_m.instances.append(
            &mut cubes
                .iter()
                .map(|cube| InstanceData {
                    _translate: cube.position.into(),
                })
                .collect(),
        );

        let curr_size = std::mem::size_of_val(&model_m.instances);

        // if previous_size != curr_size {
        model_m.instances_buffer =
            state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("instance_buffer-cube")),
                    contents: bytemuck::cast_slice(&model_m.instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        Self { x, y, cubes }
    }
}
