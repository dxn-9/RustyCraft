use std::rc::Rc;

use crate::{
    material::{Material, Texture},
    state::State,
};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;

pub trait PerVertex<T: Sized> {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
    fn size() -> usize {
        std::mem::size_of::<T>()
    }
}

impl VertexData {
    pub fn new(_position: [f32; 3], _tex_coords: [f32; 2]) -> Self {
        Self {
            _position,
            _tex_coords,
        }
    }
    pub fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}
impl PerVertex<Self> for VertexData {
    // This probably should be a macro so it would be less error prone
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::size() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
            ],
        }
    }
}

impl PerVertex<Self> for InstanceData {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::size() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 2,
            }],
        }
    }
}
pub type ModelMatrix = [[f32; 4]; 4];

impl Mesh {
    pub fn plane(w: f32, h: f32, state: &State) -> Self {
        let _vertex_data = vec![
            VertexData::new([-1.0 * w, -1.0 * h, 0.0], [0.0, 0.0]),
            VertexData::new([-1.0 * w, 1.0 * h, 0.0], [0.0, 1.0]),
            VertexData::new([1.0 * w, 1.0 * h, 0.0], [1.0, 1.0]),
            VertexData::new([1.0 * w, -1.0 * h, 0.0], [1.0, 0.0]),
        ];
        let _indices = vec![0, 1, 2, 0, 2, 3];

        let vertex_buffer = Some(state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("vertex_buffer-plane")),
                contents: bytemuck::cast_slice(&_vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));
        let index_buffer = Some(state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("index_buffer-plane")),
                contents: bytemuck::cast_slice(&_indices),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));

        let translation = Vec3::new(0.0, 0.0, 0.0);
        let scale = Vec3::new(1.0, 1.0, 1.0);
        let rotation = Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0);
        let _world_matrix =
            Mat4::from_scale_rotation_translation(scale, rotation, translation).to_cols_array_2d();

        let world_mat_buffer = Some(state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("world_buffer-plane")),
                contents: bytemuck::cast_slice(&[_world_matrix]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        ));

        Self {
            world_mat_buffer,
            index_buffer,
            vertex_buffer,
            _vertex_data,
            _indices,
            name: "plane".to_string(),
            material_id: 0,
            ..Default::default()
        }
    }
}
impl Default for Mesh {
    fn default() -> Self {
        let translation = Vec3::new(0.0, 0.0, 0.0);
        let scale = Vec3::new(1.0, 1.0, 1.0);
        let rotation = Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0);
        let _world_matrix =
            Mat4::from_scale_rotation_translation(scale, rotation, translation).to_cols_array_2d();
        Self {
            _indices: vec![],
            _vertex_data: vec![],
            _world_matrix,
            name: "default_mesh".to_string(),
            rotation,
            scale,
            translation,
            material_id: 0,
            index_buffer: None,
            vertex_buffer: None,
            world_mat_buffer: None,
        }
    }
}

impl Model {
    pub fn from_mesh_and_material(
        mesh: Mesh,
        material: Material,
        name: String,
        state: &State,
    ) -> Self {
        // Instances
        let instances = vec![InstanceData {
            _translate: glam::vec3(0.0, 0.0, 0.0).into(),
        }];
        let instances_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("instance_buffer-{name}")),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Self {
            name,
            instances,
            instances_buffer: Rc::new(instances_buffer),
            materials: vec![material],
            meshes: vec![mesh],
        }
    }

    // TODO: Refactor this into multiple meshes - also this supports only obj files for now

    pub fn from_path(
        path: &str,
        name: String,
        state: &State,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let obj = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;
        let (obj_models, obj_materials) = obj;

        let mut meshes: Vec<Mesh> = vec![];

        for model in obj_models.iter() {
            let o_mesh = &model.mesh;
            let mut _indices: Vec<u32> = vec![];
            let mut positions: Vec<[f32; 3]> = vec![];
            let mut tex_coords: Vec<[f32; 2]> = vec![];
            for i in 0..o_mesh.positions.len() / 3 {
                positions.push([
                    o_mesh.positions[i * 3 + 0],
                    o_mesh.positions[i * 3 + 1],
                    o_mesh.positions[i * 3 + 2],
                ]);
            }
            for index in o_mesh.indices.iter() {
                _indices.push(*index)
            }

            for i in 0..o_mesh.texcoords.len() / 2 {
                tex_coords.push([o_mesh.texcoords[i * 2 + 0], o_mesh.texcoords[i * 2 + 1]])
            }
            let material_id = o_mesh.material_id.unwrap_or(0) as u32;

            let _vertex_data: Vec<_> = (0..positions.len())
                .map(|i| VertexData::new(positions[i], tex_coords[i]))
                .collect();

            let vertex_buffer = Some(state.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("vertex_buffer-{name}-{}", model.name)),
                    contents: bytemuck::cast_slice(&_vertex_data),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
            let index_buffer = Some(state.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("index_buffer-{name}-{}", model.name)),
                    contents: bytemuck::cast_slice(&_indices),
                    usage: wgpu::BufferUsages::INDEX,
                },
            ));

            let translation = Vec3::new(0.0, 0.0, 0.0);
            let scale = Vec3::new(1.0, 1.0, 1.0);
            let rotation = Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0);
            let _world_matrix = Mat4::from_scale_rotation_translation(scale, rotation, translation)
                .to_cols_array_2d();

            let world_mat_buffer = Some(state.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("world_buffer-{name}-{}", model.name)),
                    contents: bytemuck::cast_slice(&[_world_matrix]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                },
            ));

            let mesh = Mesh {
                _indices,
                _vertex_data,
                material_id,
                name: format!("{name}-{}", model.name),
                index_buffer,
                vertex_buffer,
                world_mat_buffer,
                ..Default::default()
            };

            meshes.push(mesh);
        }

        let mut materials: Vec<Material> = vec![];

        let obj_mats = obj_materials?;
        for mat in obj_mats.iter() {
            materials.push(Material {
                diffuse: Texture::from_path(
                    &format!("assets/{}", mat.diffuse_texture),
                    "diffuse".to_string(),
                    state,
                )?,
            });
        }
        // Instances
        let instances = vec![InstanceData {
            _translate: glam::vec3(0.0, 0.0, 0.0).into(),
        }];
        let instances_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("instance_buffer-{name}")),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Ok(Self {
            materials,
            meshes,
            instances_buffer: Rc::new(instances_buffer),
            instances,
            name,
        })
    }
}

pub struct Model {
    pub name: String,
    pub instances: Vec<InstanceData>,
    pub instances_buffer: Rc<wgpu::Buffer>,
    // pub translation: Vec3,
    // pub scale: Vec3,
    // pub rotation: Quat,
    pub materials: Vec<Material>,
    pub meshes: Vec<Mesh>,
    // pub _world_matrix: ModelMatrix,
}

pub struct Mesh {
    pub translation: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
    pub name: String,
    pub material_id: u32,

    pub _indices: Vec<u32>,
    pub _world_matrix: ModelMatrix,
    pub _vertex_data: Vec<VertexData>,

    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub world_mat_buffer: Option<wgpu::Buffer>,
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub _translate: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexData {
    pub _position: [f32; 3],
    pub _tex_coords: [f32; 2],
}
