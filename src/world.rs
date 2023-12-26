use glam::Vec3;

const CHUNK_SIZE: u32 = 16;

struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    x: u32,
    y: u32,
    cubes_position: Vec<Vec3>,
}

enum CubeTypes {
    Dirt,
    Water,
    Wood,
}
struct Cube {
    model: crate::model::Model,
    cube_type: CubeTypes,
}

struct World {
    chunks: Vec<Chunk>,
    // This would translate to the for now hard coded edge vectors in the pnoise algo
    seed: u32,
    noise_data: Vec<f32>,
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: vec![],
            seed: 0,
            noise_data: crate::utils::perlin_noise::create_perlin_noise_data(1024, 1024, 0.05),
        }
    }
}

impl Chunk {
    // pub fn new(world: &World) -> Self {}
}
