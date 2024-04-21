use bytemuck::{Pod, Zeroable};

use super::block_type::BlockType;
use crate::chunk::BlockVec;
use crate::collision::CollisionBox;
use crate::effects::ao::{convert_ao_u8_to_f32, from_vertex_position};
use crate::world::CHUNK_SIZE;
use glam::Vec3;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct Block {
    pub position: glam::Vec3,
    pub absolute_position: glam::Vec3,
    pub collision_box: CollisionBox,
    pub block_type: BlockType,
}

#[rustfmt::skip]
pub const CUBE_VERTEX: [f32; 24] = [
    -0.5, -0.5, -0.5,
    -0.5, 0.5, -0.5,
    0.5, 0.5, -0.5,
    0.5, -0.5, -0.5,
    -0.5, -0.5, 0.5,
    -0.5, 0.5, 0.5,
    0.5, 0.5, 0.5,
    0.5, -0.5, 0.5,
];

pub trait TexturedBlock {
    fn get_texcoords(&self, face_dir: FaceDirections) -> [[f32; 2]; 4];
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FaceDirections {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

impl FaceDirections {
    pub fn create_face_data(
        &self,
        block: Arc<RwLock<Block>>,
        blocks: &Vec<((i32, i32), BlockVec)>,
    ) -> (Vec<BlockVertexData>, Vec<u32>) {
        let indices = self.get_indices();

        let mut unique_indices: Vec<u32> = Vec::with_capacity(4);

        let mut vertex_data: Vec<BlockVertexData> = Vec::with_capacity(4);

        let mut indices_map: Vec<u32> = vec![0; 6];

        for ind in indices.iter() {
            if unique_indices.contains(ind) {
                continue;
            } else {
                unique_indices.push(*ind);
            }
        }
        for (i, indices_map) in indices_map.iter_mut().enumerate() {
            let index_of = unique_indices
                .iter()
                .enumerate()
                .find_map(|(k, ind)| if *ind == indices[i] { Some(k) } else { None })
                .unwrap();
            *indices_map = index_of as u32;
        }

        let block_read = block.read().unwrap();
        let face_texcoords = block_read.block_type.get_texcoords(*self);
        let normals = self.get_normal_vector();

        unique_indices.iter().enumerate().for_each(|(i, index)| {
            let vertex_position = glam::vec3(
                CUBE_VERTEX[*index as usize * 3_usize] + block_read.absolute_position.x,
                CUBE_VERTEX[*index as usize * 3 + 1] + block_read.absolute_position.y,
                CUBE_VERTEX[*index as usize * 3 + 2] + block_read.absolute_position.z,
            );

            vertex_data.push(BlockVertexData {
                position: [
                    CUBE_VERTEX[*index as usize * 3_usize] + block_read.position.x,
                    CUBE_VERTEX[*index as usize * 3 + 1] + block_read.position.y,
                    CUBE_VERTEX[*index as usize * 3 + 2] + block_read.position.z,
                ],
                ao: convert_ao_u8_to_f32(from_vertex_position(&vertex_position, blocks)),
                normal: normals.into(),
                tex_coords: face_texcoords[i],
            })
        });

        (vertex_data, indices_map)
    }
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
pub struct BlockVertexData {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub ao: f32,
}

impl Block {
    // Takes in relative position
    pub fn new(position: Vec3, chunk: (i32, i32), block_type: BlockType) -> Block {
        let absolute_position = glam::vec3(
            (chunk.0 * CHUNK_SIZE as i32 + position.x as i32) as f32,
            position.y,
            (chunk.1 * CHUNK_SIZE as i32 + position.z as i32) as f32,
        );
        let collision_box = CollisionBox::from_block_position(
            absolute_position.x,
            absolute_position.y,
            absolute_position.z,
        );
        Block {
            collision_box,
            position,
            block_type,
            absolute_position,
        }
    }
    pub fn get_neighbour_chunks_coords(&self) -> Vec<(i32, i32)> {
        let chunk = self.get_chunk_coords();
        let mut neighbour_chunks = vec![];

        if self.position.x == 15.0 {
            neighbour_chunks.push((chunk.0 + 1, chunk.1));
        }
        if self.position.x == 0.0 {
            neighbour_chunks.push((chunk.0 - 1, chunk.1));
        }
        if self.position.z == 15.0 {
            neighbour_chunks.push((chunk.0, chunk.1 + 1));
        }
        if self.position.z == 0.0 {
            neighbour_chunks.push((chunk.0, chunk.1 - 1));
        }
        neighbour_chunks
    }
    pub fn is_on_chunk_border(&self) -> bool {
        self.position.x == 0.0
            || self.position.x == 15.0
            || self.position.z == 0.0
            || self.position.z == 15.0
    }
    pub fn get_chunk_coords(&self) -> (i32, i32) {
        (
            (f32::floor(self.absolute_position.x / CHUNK_SIZE as f32)) as i32,
            (f32::floor(self.absolute_position.z / CHUNK_SIZE as f32)) as i32,
        )
    }
    pub fn get_vertex_data_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BlockVertexData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // Normals
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
                // Tex coords
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                },
                // Ao
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                },
            ],
        }
    }
}

impl FaceDirections {
    pub fn all() -> [FaceDirections; 6] {
        [
            FaceDirections::Back,
            FaceDirections::Bottom,
            FaceDirections::Top,
            FaceDirections::Front,
            FaceDirections::Left,
            FaceDirections::Right,
        ]
    }
    pub fn opposite(&self) -> FaceDirections {
        match self {
            FaceDirections::Back => FaceDirections::Front,
            FaceDirections::Bottom => FaceDirections::Top,
            FaceDirections::Top => FaceDirections::Bottom,
            FaceDirections::Front => FaceDirections::Back,
            FaceDirections::Left => FaceDirections::Right,
            FaceDirections::Right => FaceDirections::Left,
        }
    }
    pub fn get_normal_vector(&self) -> glam::Vec3 {
        match self {
            FaceDirections::Back => glam::vec3(0.0, 0.0, 1.0),
            FaceDirections::Bottom => glam::vec3(0.0, -1.0, 0.0),
            FaceDirections::Top => glam::vec3(0.0, 1.0, 0.0),
            FaceDirections::Front => glam::vec3(0.0, 0.0, -1.0),
            FaceDirections::Left => glam::vec3(-1.0, 0.0, 0.0),
            FaceDirections::Right => glam::vec3(1.0, 0.0, 0.0),
        }
    }
    pub fn get_indices(&self) -> [u32; 6] {
        match self {
            FaceDirections::Back => [7, 6, 5, 7, 5, 4],
            FaceDirections::Front => [0, 1, 2, 0, 2, 3],
            FaceDirections::Left => [4, 5, 1, 4, 1, 0],
            FaceDirections::Right => [3, 2, 6, 3, 6, 7],
            FaceDirections::Top => [1, 5, 6, 1, 6, 2],
            FaceDirections::Bottom => [4, 0, 3, 4, 3, 7],
        }
    }
}
