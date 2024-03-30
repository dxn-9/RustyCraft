use rand::random;
use std::any::Any;

use super::block::{FaceDirections, TexturedBlock};

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
}
impl BlockType {
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
            _ => panic!("Invalid id"),
        }
    }
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
    return glam::vec2(left_bound, low_bound);
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
