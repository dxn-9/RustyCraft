use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub _position: [f32; 4],
    pub _vertex_color: [f32; 4],
}
impl ModelVertex {
    pub fn new(_position: [f32; 3], _vertex_color: [f32; 4]) -> Self {
        Self {
            _position: [_position[0], _position[1], _position[2], 1.0],
            _vertex_color,
        }
    }
}

pub type ModelMatrix = [[f32; 4]; 4];

pub struct Mesh {
    pub name: String,
    pub elements_count: u32,
    pub instances: u32,

    pub translation: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
    pub _world_matrix: ModelMatrix,
    pub _vertices: Vec<ModelVertex>,
    pub _indices: Vec<u32>,
}

fn vertex(pos: [f32; 3], vc: [f32; 4]) -> ModelVertex {
    ModelVertex::new(pos, vc)
}

const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

pub fn create_cube_mesh() -> Mesh {
    #[rustfmt::skip]
    let _vertices = vec![
        // Front
        vertex([-1.0, -1.0, 1.0],RED ),
        vertex([1.0, -1.0, 1.0],RED  ),
        vertex([1.0, 1.0, 1.0], RED ),
        vertex([-1.0, 1.0, 1.0], RED ),
        // bottom (0, 0, -1.0)
        vertex([-1.0, 1.0, -1.0], BLUE ),
        vertex([1.0, 1.0, -1.0], BLUE ),
        vertex([1.0, -1.0, -1.0], BLUE ),
        vertex([-1.0, -1.0, -1.0],BLUE  ),
        // right (1.0, 0, 0)
        vertex([1.0, -1.0, -1.0],RED  ),
        vertex([1.0, 1.0, -1.0],RED  ),
        vertex([1.0, 1.0, 1.0], RED ),
        vertex([1.0, -1.0, 1.0],RED  ),
        // left (-1.0, 0, 0)
        vertex([-1.0, -1.0, 1.0], BLUE),
        vertex([-1.0, 1.0, 1.0], BLUE),
        vertex([-1.0, 1.0, -1.0], BLUE),
        vertex([-1.0, -1.0, -1.0],BLUE ),
        // front (0, 1.0, 0)
        vertex([1.0, 1.0, -1.0],RED  ),
        vertex([-1.0, 1.0, -1.0],RED  ),
        vertex([-1.0, 1.0, 1.0],RED  ),
        vertex([1.0, 1.0, 1.0], RED ),
        // back (0, -1.0, 0)
        vertex([1.0, -1.0, 1.0],BLUE),
        vertex([-1.0, -1.0, 1.0], BLUE),
        vertex([-1.0, -1.0, -1.0],BLUE),
        vertex([1.0, -1.0, -1.0],BLUE),

    ];
    #[rustfmt::skip]
    let _indices = vec![
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    let translation = Vec3::new(0.0, 0.0, 0.0);
    let scale = Vec3::new(1.0, 1.0, 1.0);
    let rotation = Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0);

    let _world_matrix =
        Mat4::from_scale_rotation_translation(scale, rotation, translation).to_cols_array_2d();

    Mesh {
        translation,
        scale,
        rotation,
        name: "square_mesh".to_string(),
        elements_count: _indices.len() as u32,
        instances: 1,
        _world_matrix,
        _vertices,
        _indices,
    }
}
