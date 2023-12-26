use std::{cell::RefCell, rc::Rc};

use glam::{vec3, Vec3};

use crate::model::Model;

const CHUNK_SIZE: u32 = 16;

const NOISE_WIDTH: u32 = 1024;
const NOISE_HEIGHT: u32 = 1024;
const FREQUENCY: f32 = 0.05;

pub struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    pub x: u32,
    pub y: u32,
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
    pub fn init_world(model: Rc<RefCell<Model>>) -> Self {
        let mut world = Self {
            chunks: vec![],
            seed: 0,
            noise_data: crate::utils::perlin_noise::create_perlin_noise_data(
                NOISE_WIDTH,
                NOISE_HEIGHT,
                FREQUENCY,
            ),
        };
        let first_chunk = Chunk::new(0, 0, &world, model);
        world.chunks.push(first_chunk);

        world
    }
}

impl Chunk {
    pub fn new(x: u32, y: u32, world: &World, model: Rc<RefCell<Model>>) -> Self {
        let starting_point =
            ((y * NOISE_WIDTH * CHUNK_SIZE) + (x * CHUNK_SIZE)) % (NOISE_WIDTH * NOISE_HEIGHT);

        let mut cubes: Vec<CubeData> = vec![];

        for i in starting_point..starting_point + 16 {
            cubes.push(CubeData {
                position: vec3(0.0, world.noise_data[i as usize] * 10.0, 0.0),
                ctype: CubeType::Dirt,
                model: model.clone(),
            });
        }
        Self { x, y, cubes }
    }
}
