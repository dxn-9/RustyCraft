use super::block::{FaceDirections, TexturedBlock};
use crate::world::{RNG_SEED, WATER_HEIGHT_LEVEL};
use rand::{rngs::StdRng, Rng, SeedableRng};

#[derive(Clone, Copy, Debug)]
// This can be 1, 2 [because sometimes we want to reuse the same texture for the bottom as the top]
pub struct FaceTexture(u32);
#[derive(Clone, Copy, Debug)]
pub struct BlockTypeConfigs {
    pub id: u32,
    // Integers representing the nth texture to use.
    pub textures: [FaceTexture; 3], // 1: Lateral texture, 2: Top texture, 3: Bottom texture
    pub is_translucent: bool,
}

impl BlockTypeConfigs {
    pub fn get(block_type: BlockType) -> BlockTypeConfigs {
        match block_type {
            BlockType::Grass => BlockTypeConfigs {
                id: 0,
                textures: [FaceTexture(6), FaceTexture(7), FaceTexture(8)],
                is_translucent: false,
            },
            BlockType::Dirt => BlockTypeConfigs {
                id: 1,
                textures: [FaceTexture(0), FaceTexture(0), FaceTexture(0)],
                is_translucent: false,
            },

            BlockType::Water => BlockTypeConfigs {
                id: 2,
                textures: [FaceTexture(1), FaceTexture(1), FaceTexture(1)],
                is_translucent: true,
            },

            BlockType::Wood => BlockTypeConfigs {
                id: 3,
                textures: [FaceTexture(4), FaceTexture(5), FaceTexture(5)],
                is_translucent: false,
            },
            BlockType::Leaf => BlockTypeConfigs {
                id: 4,
                textures: [FaceTexture(2), FaceTexture(2), FaceTexture(2)],
                is_translucent: false,
            },
            BlockType::Stone => BlockTypeConfigs {
                id: 5,
                textures: [FaceTexture(3), FaceTexture(3), FaceTexture(3)],
                is_translucent: false,
            },
            BlockType::Sand => BlockTypeConfigs {
                id: 6,
                textures: [FaceTexture(9), FaceTexture(9), FaceTexture(9)],
                is_translucent: false,
            },
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockType {
    Grass,
    Dirt,
    Water,
    Wood,
    Leaf,
    Stone,
    Sand,
}
impl BlockType {
    pub const MAX_ID: u32 = 6;

    pub fn get_config(&self) -> BlockTypeConfigs {
        BlockTypeConfigs::get(*self)
    }
    pub fn to_id(&self) -> u32 {
        self.get_config().id
    }
    pub fn from_id(id: u32) -> BlockType {
        match id {
            0 => Self::Grass,
            1 => Self::Dirt,
            2 => Self::Water,
            3 => Self::Wood,
            4 => Self::Leaf,
            5 => Self::Stone,
            6 => Self::Sand,
            _ => panic!("Invalid id"),
        }
    }
}
fn calc_scalar(y: u32, t: Threshold) -> f32 {
    (y as f32 - t[0] as f32) / (t[1] as f32 - t[0] as f32)
}
// Threshold: ( lowerbound , upperbound )
type Threshold = [u32; 2];
const STONE_THRESHOLD: Threshold = [15, 24];
const SAND_THRESHOLD: Threshold = [WATER_HEIGHT_LEVEL as u32, WATER_HEIGHT_LEVEL as u32 + 2];
impl BlockType {
    pub fn from_position(x: u32, y: u32, z: u32) -> BlockType {
        let mut rng = StdRng::seed_from_u64(RNG_SEED + (y * x * z) as u64);

        if y <= SAND_THRESHOLD[0] {
            BlockType::Sand
        } else if y <= SAND_THRESHOLD[1] {
            let r = rng.gen::<f32>();
            let s = calc_scalar(y, SAND_THRESHOLD);
            if r + s > 1.0 {
                BlockType::Dirt
            } else {
                BlockType::Sand
            }
        } else if y < STONE_THRESHOLD[0] {
            BlockType::Dirt
        } else if y <= STONE_THRESHOLD[1] {
            let r = rng.gen::<f32>();
            let s = calc_scalar(y, STONE_THRESHOLD);
            if r + s >= 1.0 {
                BlockType::Stone
            } else {
                BlockType::Dirt
            }
        } else {
            BlockType::Stone
        }
    }
}

const TEXTURE_SIZE: u32 = 256;
const BLOCK_PER_ROW: u32 = 8;
// 32px per block
const BLOCK_OFFSET: u32 = TEXTURE_SIZE / BLOCK_PER_ROW;
const BLOCK_OFFSET_NORMALIZED: f32 = BLOCK_OFFSET as f32 / TEXTURE_SIZE as f32;

fn get_base_coords(config: &BlockTypeConfigs, face_dir: FaceDirections) -> glam::Vec2 {
    let face_offset = match face_dir {
        FaceDirections::Top => config.textures[1],
        FaceDirections::Bottom => config.textures[2],
        _ => config.textures[0],
    };
    let y_offset = (face_offset.0 / BLOCK_PER_ROW) as f32;
    let x_offset = (face_offset.0 % BLOCK_PER_ROW) as f32;

    let low_bound = y_offset * BLOCK_OFFSET_NORMALIZED + BLOCK_OFFSET_NORMALIZED;
    let left_bound = x_offset * BLOCK_OFFSET_NORMALIZED;
    glam::vec2(left_bound, low_bound)
}
fn get_tex_coords(config: &BlockTypeConfigs, face_dir: FaceDirections) -> [[f32; 2]; 4] {
    let bc = get_base_coords(config, face_dir);
    [
        [bc.x, bc.y],
        [bc.x, bc.y - BLOCK_OFFSET_NORMALIZED],
        [
            bc.x + BLOCK_OFFSET_NORMALIZED,
            bc.y - BLOCK_OFFSET_NORMALIZED,
        ],
        [bc.x + BLOCK_OFFSET_NORMALIZED, bc.y],
    ]
}

impl TexturedBlock for BlockType {
    fn get_texcoords(&self, face_dir: FaceDirections) -> [[f32; 2]; 4] {
        get_tex_coords(&self.get_config(), face_dir)
    }
}
