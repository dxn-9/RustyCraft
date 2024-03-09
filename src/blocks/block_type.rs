use rand::random;
use std::any::Any;

use super::block::{FaceDirections, TexturedBlock};

#[derive(Clone, Copy, Debug)]
// This can be 1, 2 [because sometimes we want to reuse the same texture for the bottom as the top]
pub struct FaceTexture(u32);
#[derive(Clone, Copy, Debug)]
pub struct BlockTypeConfigs {
    pub id: u32,
    // The amount to add to the block id (because some blocks have more than 1 texture)
    pub step: u32,
    // Different texture for top face
    pub top_texture: Option<FaceTexture>,
    // Different texture for bottom face
    pub bottom_texture: Option<FaceTexture>,
    pub is_translucent: bool,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum BlockType {
    Grass(BlockTypeConfigs),
    Dirt(BlockTypeConfigs),
    Water(BlockTypeConfigs),
    Wood(BlockTypeConfigs),
    Leaf(BlockTypeConfigs),
    Stone(BlockTypeConfigs),
}
impl BlockType {
    pub fn from_id(id: u32) -> BlockType {
        match id {
            0 => Self::dirt(),
            1 => Self::water(),
            2 => Self::leaf(),
            3 => Self::stone(),
            4 => Self::wood(),
            5 => Self::grass(),
            _ => panic!("Invalid id")
        }
    }
    pub fn to_id(&self) -> u32 {
        // meh
        match self {
            Self::Grass(f) => f.id,
            Self::Dirt(f) => f.id,
            Self::Water(f) => f.id,
            Self::Wood(f) => f.id,
            Self::Leaf(f) => f.id,
            Self::Stone(f) => f.id,
        }
    }

    pub fn dirt() -> Self {
        Self::Dirt(BlockTypeConfigs {
            id: 0,
            step: 0,
            bottom_texture: None,
            top_texture: None,
            is_translucent: false,
        })
    }
    pub fn water() -> Self {
        Self::Water(BlockTypeConfigs {
            id: 1,
            step: 0,
            bottom_texture: None,
            top_texture: None,
            is_translucent: true,
        })
    }
    pub fn leaf() -> Self {
        Self::Leaf(BlockTypeConfigs {
            id: 2,
            step: 0,
            bottom_texture: None,
            top_texture: None,
            is_translucent: false,
        })
    }
    pub fn stone() -> Self {
        Self::Stone(BlockTypeConfigs {
            id: 3,
            step: 0,
            bottom_texture: None,
            top_texture: None,
            is_translucent: false,
        })
    }
    pub fn wood() -> Self {
        Self::Wood(BlockTypeConfigs {
            id: 4,
            step: 0,
            bottom_texture: Some(FaceTexture(1)),
            top_texture: Some(FaceTexture(1)),
            is_translucent: false,
        })
    }

    pub fn grass() -> Self {
        Self::Grass(BlockTypeConfigs {
            id: 5,
            step: 1,
            bottom_texture: Some(FaceTexture(2)),
            top_texture: Some(FaceTexture(1)),
            is_translucent: false,
        })
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
                BlockType::stone()
            } else {
                BlockType::dirt()
            }
        } else if y < Self::L_STONE_THRESHOLD {
            BlockType::stone()
        } else {
            BlockType::dirt()
        }
    }
}

const TEXTURE_SIZE: u32 = 256;
const BLOCK_PER_ROW: u32 = 8;
const BLOCK_OFFSET: u32 = TEXTURE_SIZE / BLOCK_PER_ROW;
const BLOCK_OFFSET_NORMALIZED: f32 = BLOCK_OFFSET as f32 / TEXTURE_SIZE as f32;

fn get_base_coords(config: &BlockTypeConfigs, face_dir: FaceDirections) -> glam::Vec2 {
    let face_offset = match face_dir {
        FaceDirections::Top => config.top_texture.unwrap_or(FaceTexture(0)),
        FaceDirections::Bottom => config.bottom_texture.unwrap_or(FaceTexture(0)),
        _ => FaceTexture(0),
    };

    let position = config.id + config.step + face_offset.0;
    let wrap = position / BLOCK_PER_ROW;

    let low_bound = 1.0 - (BLOCK_OFFSET_NORMALIZED + (BLOCK_OFFSET_NORMALIZED * wrap as f32));
    let left_bound = (position as f32 % BLOCK_PER_ROW as f32) / BLOCK_PER_ROW as f32;
    glam::vec2(left_bound, low_bound)
}
fn get_tex_coords(config: &BlockTypeConfigs, face_dir: FaceDirections) -> [[f32; 2]; 4] {
    let bc = get_base_coords(config, face_dir);
    [
        [bc.x, bc.y],
        [bc.x, bc.y + BLOCK_OFFSET_NORMALIZED],
        [
            bc.x + BLOCK_OFFSET_NORMALIZED,
            bc.y + BLOCK_OFFSET_NORMALIZED,
        ],
        [bc.x + BLOCK_OFFSET_NORMALIZED, bc.y],
    ]
}

impl TexturedBlock for BlockType {
    fn get_texcoords(&self, face_dir: FaceDirections) -> [[f32; 2]; 4] {
        match self {
            BlockType::Grass(config) => get_tex_coords(config, face_dir),
            BlockType::Dirt(config) => get_tex_coords(config, face_dir),
            BlockType::Water(config) => get_tex_coords(config, face_dir),
            BlockType::Stone(config) => get_tex_coords(config, face_dir),
            BlockType::Wood(config) => get_tex_coords(config, face_dir),
            BlockType::Leaf(config) => get_tex_coords(config, face_dir),
        }
    }
}
