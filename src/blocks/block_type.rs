use rand::random;

use super::block::{FaceDirections, TexturedBlock};

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
impl TexturedBlock for BlockType {
    fn get_texcoords(&self, face_dir: FaceDirections) -> [[f32; 2]; 4] {
        [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]
        // match self {
        //     BlockType::Grass => match face_dir {
        //         FaceDirections::Top => {}
        //         FaceDirections::Bottom => {}
        //         _ => {}
        //     },
        //     BlockType::Dirt => {}
        //     _ => {}
        // }
        // todo!()
    }
}
