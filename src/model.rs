use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub _position: [f32; 3],
    pub _vertex_color: [f32; 4],
}
impl ModelVertex {
    pub fn new(_position: [f32; 3], _vertex_color: [f32; 4]) -> Self {
        Self {
            _position,
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

impl Mesh {
    pub fn recalculate_world_matrix(&mut self) {
        self._world_matrix =
            Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
                .to_cols_array_2d();
    }
}

const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

pub fn create_cube_mesh() -> Mesh {
    #[rustfmt::skip]
    let _vertices = vec![
        // Front
        ModelVertex::new([-0.5, -0.5, 0.0], BLUE),
        ModelVertex::new([-0.5, 0.5, 0.0], BLUE),
        ModelVertex::new([0.5,  -0.5, 0.0], BLUE),
        ModelVertex::new([0.5, 0.5, 0.0], BLUE),
        // Left
        ModelVertex::new([-0.5, -0.5, 0.0], RED),
        ModelVertex::new([-0.5, -0.5, 0.5], RED),
        ModelVertex::new([-0.5, 0.5, 0.0], RED),
        ModelVertex::new([-0.5, 0.5, 0.5], RED),
        // Back
        ModelVertex::new([-0.5, -0.5, 0.5], BLUE),
        ModelVertex::new([-0.5, 0.5, 0.5], BLUE),
        ModelVertex::new([ 0.5, -0.5, 0.5], BLUE),
        ModelVertex::new([ 0.5, 0.5, 0.5], BLUE),
        // Right
        ModelVertex::new([ 0.5, -0.5, 0.0], RED),
        ModelVertex::new([ 0.5, 0.5, 0.0], RED),
        ModelVertex::new([ 0.5, -0.5, 0.5], RED),
        ModelVertex::new([ 0.5, 0.5, 0.5], RED),
        // Top
        ModelVertex::new([-0.5, 0.5, 0.0], BLUE),
        ModelVertex::new([-0.5, 0.5, 0.5], BLUE),
        ModelVertex::new([ 0.5, 0.5, 0.0], BLUE),
        ModelVertex::new([ 0.5, 0.5, 0.5], BLUE),
        // Bottom
        ModelVertex::new([-0.5, -0.5, 0.0], RED),
        ModelVertex::new([-0.5, -0.5, 0.5], RED),
        ModelVertex::new([ 0.5, -0.5, 0.0], RED),
        ModelVertex::new([ 0.5, -0.5, 0.5], RED),

    ];
    #[rustfmt::skip]
    let _indices = vec![
        // Front
        0, 1, 2,
        2, 1, 3,
        // Left
        4, 5, 6,
        5, 6, 7,
        // Back
        8, 9, 10,
        10, 9, 11,
        // // Right
        12, 13, 14,
        14, 13, 15,
        // // Top
        16, 17, 18,
        18, 17, 19,
        // // Bottom
        20, 21, 22,
        22, 21, 23
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
