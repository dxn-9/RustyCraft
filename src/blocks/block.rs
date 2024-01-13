use bytemuck::{Pod, Zeroable};

use super::block_type::BlockType;
use std::{cell::RefCell, rc::Rc};

pub struct BlockFace {
    pub face_direction: FaceDirections,
    pub block: Rc<RefCell<Block>>,
}
pub struct Block {
    pub position: glam::Vec3,
    pub faces: Option<Vec<BlockFace>>,
    pub block_type: BlockType,
    pub is_translucent: bool,
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

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
pub struct BlockVertexData {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl BlockFace {
    pub fn create_face_data(&self) -> (Vec<BlockVertexData>, Vec<u32>) {
        let face_direction = self.face_direction;
        let block = self.block.as_ref().borrow();

        let indices = face_direction.get_indices();
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

        let face_texcoords = block.block_type.get_texcoords(face_direction);
        let normals = face_direction.get_normal_vector();

        unique_indices.iter().enumerate().for_each(|(i, index)| {
            vertex_data.push(BlockVertexData {
                position: [
                    CUBE_VERTEX[(*index as usize * 3 + 0) as usize] + block.position.x,
                    CUBE_VERTEX[(*index as usize * 3 + 1) as usize] + block.position.y,
                    CUBE_VERTEX[(*index as usize * 3 + 2) as usize] + block.position.z,
                ],
                normal: normals.into(),
                tex_coords: face_texcoords[i],
            })
        });

        (vertex_data, indices_map)
    }
}
impl Block {
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
