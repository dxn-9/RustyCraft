pub mod ao {
    use crate::blocks::block_type::BlockType;
    use crate::chunk::BlockVec;
    use crate::perf;
    use crate::utils::{ChunkFromPosition, RelativeFromAbsolute};
    use crate::world::CHUNK_SIZE;
    use glam::vec3;
    use std::time::Instant;

    // https://0fps.net/2013/07/03/ambient-occlusion-for-minecraft-like-worlds/
    pub(crate) fn calc_vertex_ao(side1: bool, side2: bool, up: bool) -> u8 {
        if side1 && side2 {
            return 0;
        }
        return 3 - (side1 as u8 + side2 as u8 + up as u8);
    }
    pub(crate) fn from_vertex_position(
        vertex_position: &glam::Vec3,
        blocks_positions: &Vec<((i32, i32), BlockVec)>,
    ) -> u8 {
        let side1_position = *vertex_position + glam::vec3(1.0, 1.0, 0.0);
        let side2_position = *vertex_position + glam::vec3(0.0, 1.0, 1.0);
        let corner_position = *vertex_position + glam::vec3(1.0, 1.0, 1.0);

        let side1_chunk = side1_position.get_chunk_from_position_absolute();
        let side1_position = side1_position.relative_from_absolute();

        let side2_chunk = side2_position.get_chunk_from_position_absolute();
        let side2_position = side2_position.relative_from_absolute();

        let corner_chunk = corner_position.get_chunk_from_position_absolute();
        let corner_position = corner_position.relative_from_absolute();

        let mut has_side1 = false;
        let mut has_side2 = false;
        let mut has_corner = false;

        for (position, chunk, val) in [
            (side1_position, side1_chunk, &mut has_side1),
            (side2_position, side2_chunk, &mut has_side2),
            (corner_position, corner_chunk, &mut has_corner),
        ] {
            if let Some(blocks) = blocks_positions.iter().find_map(|c| {
                if c.0 == chunk {
                    Some(c.1.clone())
                } else {
                    None
                }
            }) {
                let blocks = blocks.read().unwrap();
                let ycol = &blocks[((position.x * CHUNK_SIZE as f32) + position.z) as usize];
                if let Some(place) = ycol.get(position.y as usize) {
                    if let Some(block) = place {
                        if block.read().unwrap().block_type != BlockType::Water {
                            *val = true
                        }
                    }
                }
            }
        }
        return calc_vertex_ao(has_side1, has_side2, has_corner);
    }
    // ao -> 1 (max)
    // ao -> 0 (min)
    pub(crate) fn convert_ao_u8_to_f32(ao: u8) -> f32 {
        1.0 - (ao as f32 / 3.0)
    }

    mod test {
        use super::*;
        use crate::blocks::block::Block;
        use crate::blocks::block_type::BlockType;
        use std::sync::{Arc, RwLock};

        // #[test]
        // fn should_calculate_the_correct_ao() {
        //     let vertex_position = vec3(0.5, 0.5, 0.5); // Belongs to voxel 0,0,0
        //     let block_vec: BlockVec = Arc::new(RwLock::new(vec![
        //         vec![];
        //         (CHUNK_SIZE * CHUNK_SIZE) as usize
        //     ]));
        //     let neighbour_voxels = [vec3(1.0, 1.0, 0.0), vec3(1.0, 1.0, 1.0)];
        //     for voxel in &neighbour_voxels {
        //         let mut block_write = block_vec.write().unwrap();
        //         let region = &mut block_write[((voxel.x * CHUNK_SIZE as f32) + voxel.z) as usize];

        //         for _ in region.len()..=voxel.y as usize {
        //             region.push(None);
        //         }

        //         region[voxel.y as usize] = Some(Arc::new(RwLock::new(Block::new(
        //             voxel.clone(),
        //             voxel.get_chunk_from_position_absolute(),
        //             BlockType::dirt(),
        //         ))));
        //     }
        //     let chunk_blocks = vec![((0, 0), block_vec)];

        //     let vao = from_vertex_position(&vertex_position, &chunk_blocks);

        //     assert_eq!(vao, 1);

        //     let vertex_position = vec3(0.5, 0.5, -0.5); // Belongs to voxel 0,0,0

        //     let vao = from_vertex_position(&vertex_position, &chunk_blocks);

        //     assert_eq!(vao, 2);

        //     let vertex_position = vec3(-5.5, 5.5, -0.5); // Belongs to voxel 0,0,0

        //     let vao = from_vertex_position(&vertex_position, &chunk_blocks);

        //     assert_eq!(vao, 3);
        // }
    }
}
