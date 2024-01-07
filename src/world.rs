use std::{cell::RefCell, collections::HashMap, rc::Rc};

use glam::{vec3, Vec3};
use rand::random;
use wgpu::{util::DeviceExt, BindGroupLayout, BindGroupLayoutDescriptor, BufferUsages};

use crate::{
    model::{InstanceData, Model},
    state::State,
};

const CHUNK_SIZE: u32 = 16;
const CHUNK_HEIGHT: u8 = u8::MAX;

const NOISE_SIZE: u32 = 1024;
const FREQUENCY: f32 = 1. / 128.;
const NOISE_CHUNK_PER_ROW: u32 = NOISE_SIZE / CHUNK_SIZE;
// There will be a CHUNKS_PER_ROW * CHUNKS_PER_ROW region
pub const CHUNKS_PER_ROW: u32 = 4;
pub const CHUNKS_REGION: u32 = CHUNKS_PER_ROW * CHUNKS_PER_ROW;

type BlockMap = HashMap<i32, HashMap<i32, HashMap<i32, Rc<RefCell<Block>>>>>;
type BlockVec = Vec<Rc<RefCell<Block>>>;

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
pub struct Chunk {
    // probably there needs to be a cube type with more info ( regarding type, etc. )
    pub x: i32,
    pub y: i32,
    pub blocks: BlockVec,
    pub blocks_map: BlockMap,
    pub indices: u32,
    pub chunk_bind_group: wgpu::BindGroup,
    pub chunk_position_buffer: wgpu::Buffer,
    pub chunk_vertex_buffer: wgpu::Buffer,
    pub chunk_index_buffer: wgpu::Buffer,
}

pub struct Block {
    pub faces: Option<Vec<BlockFace>>,
    pub position: glam::Vec3,
    pub block_type: BlockType,
}

pub struct BlockFace {
    pub face_direction: FaceDirections,
    pub block: Rc<RefCell<Block>>,
    pub is_visible: bool,
}

#[derive(Clone, Copy)]
pub enum FaceDirections {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}
impl FaceDirections {
    fn all() -> [FaceDirections; 6] {
        [
            FaceDirections::Back,
            FaceDirections::Bottom,
            FaceDirections::Top,
            FaceDirections::Front,
            FaceDirections::Left,
            FaceDirections::Right,
        ]
    }
    fn get_normal_vector(&self) -> glam::Vec3 {
        match self {
            FaceDirections::Back => glam::vec3(0.0, 0.0, 1.0),
            FaceDirections::Bottom => glam::vec3(0.0, -1.0, 0.0),
            FaceDirections::Top => glam::vec3(0.0, 1.0, 0.0),
            FaceDirections::Front => glam::vec3(0.0, 0.0, -1.0),
            FaceDirections::Left => glam::vec3(1.0, 0.0, 0.0),
            FaceDirections::Right => glam::vec3(-1.0, 0.0, 0.0),
        }
    }
    fn get_indices(&self) -> [u32; 6] {
        match self {
            FaceDirections::Back => [7, 4, 5, 7, 5, 6],
            FaceDirections::Front => [0, 3, 2, 0, 2, 1],
            FaceDirections::Left => [7, 3, 2, 7, 2, 6],
            FaceDirections::Right => [4, 0, 1, 4, 1, 5],
            FaceDirections::Top => [1, 2, 6, 1, 6, 5],
            FaceDirections::Bottom => [0, 3, 7, 0, 7, 4],
        }
    }
}

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

pub struct World {
    pub chunks: Vec<Chunk>,
    // This would translate to the for now hard coded edge vectors in the pnoise algo
    pub seed: u32,
    pub noise_data: Vec<f32>,
    pub chunk_data_layout: wgpu::BindGroupLayout,
}

impl World {
    pub fn update_current_chunk_buffer(&self, chunk: &Chunk, state: &State) {
        // todo!()
    }
    pub fn init_world(device: &wgpu::Device) -> Self {
        let noise_data =
            crate::utils::noise::create_world_noise_data(NOISE_SIZE, NOISE_SIZE, FREQUENCY);
        let mut chunks = vec![];

        let lb = (CHUNKS_PER_ROW / 2) as i32;
        let ub = if CHUNKS_PER_ROW % 2 == 0 {
            (CHUNKS_PER_ROW / 2 - 1) as i32
        } else {
            (CHUNKS_PER_ROW / 2) as i32
        };

        let chunk_data_layout = device.create_bind_group_layout(&Chunk::get_bind_group_layout());

        // println!("lb {lb} {ub}");
        for j in -lb..=ub {
            for i in -lb..=ub {
                chunks.push(Chunk::new(i, j, &noise_data, device, &chunk_data_layout));
            }
        }
        // chunks.push(Chunk::new(0, 1, &noise_data, device, &chunk_data_layout));

        Self {
            chunk_data_layout,
            chunks,
            noise_data,
            seed: 0,
        }
    }
}

impl Chunk {
    pub fn exists_block_at(&self, position: &glam::Vec3) -> bool {
        match self.blocks_map.get(&(position.y as i32)) {
            Some(x_map) => match x_map.get(&(position.x as i32)) {
                Some(z_map) => match z_map.get(&(position.z as i32)) {
                    Some(_) => true,
                    None => false,
                },
                None => false,
            },
            None => false,
        }
    }
    pub fn build_blocks_map(blocks: &BlockVec) -> BlockMap {
        let mut blocks_map: BlockMap = HashMap::new();
        for block in blocks.iter() {
            let blockbrw = block.borrow();
            let x_map = match blocks_map.get_mut(&(blockbrw.position.y as i32)) {
                Some(x_map) => x_map,
                None => {
                    blocks_map.insert(blockbrw.position.y as i32, HashMap::new());
                    blocks_map.get_mut(&(blockbrw.position.y as i32)).unwrap()
                }
            };
            let z_map = match x_map.get_mut(&(blockbrw.position.x as i32)) {
                Some(z_map) => z_map,
                None => {
                    x_map.insert(blockbrw.position.x as i32, HashMap::new());
                    x_map.get_mut(&(blockbrw.position.x as i32)).unwrap()
                }
            };
            match z_map.get_mut(&(blockbrw.position.z as i32)) {
                Some(_) => panic!("Cannot have more than 1 block in the same place"),
                None => {
                    z_map.insert(blockbrw.position.z as i32, block.clone());
                }
            }
        }
        blocks_map
    }
    pub fn build_mesh(&mut self, player_pos: &glam::Vec3, queue: &wgpu::Queue) {
        let chunk_position = glam::vec3(self.x as f32 * 16.0, 0.0, self.y as f32 * 16.0);
        let mut vertices: Vec<f32> = vec![];
        let mut indices: Vec<u32> = vec![];
        for block in self.blocks.iter() {
            {
                let mut blockbrw = block.as_ref().borrow_mut();
                let cube_pos = blockbrw.position.clone();
                let faces = blockbrw.faces.as_mut().unwrap();
                for face in faces.iter_mut() {
                    let face_world_pos = face.face_direction.get_normal_vector() + cube_pos;
                    if self.exists_block_at(&face_world_pos) {
                        face.is_visible = false
                    } else {
                        let face_to_player_dir =
                            (player_pos.clone() - (face_world_pos + chunk_position)).normalize();
                        if face
                            .face_direction
                            .get_normal_vector()
                            .dot(face_to_player_dir)
                            < 0.0
                        {
                            face.is_visible = false
                        } else {
                            face.is_visible = true
                        }
                    }
                }
            }
            let blockbrw = block.as_ref().borrow();
            let visible_faces: Vec<_> = blockbrw
                .faces
                .as_ref()
                .unwrap()
                .iter()
                .filter(|face| face.is_visible)
                .collect();

            for visible_face in visible_faces.iter() {
                let offset = visible_face.block.as_ref().borrow().position;
                for i in visible_face.face_direction.get_indices().iter() {
                    let v_x = CUBE_VERTEX[(*i * 3 + 0) as usize] + offset.x;
                    let v_y = CUBE_VERTEX[(*i * 3 + 1) as usize] + offset.y;
                    let v_z = CUBE_VERTEX[(*i * 3 + 2) as usize] + offset.z;

                    let index_len = indices.len();
                    // This might be better as a hashmap instead of a linear search?
                    for vi in 0..vertices.len() / 3 {
                        if vertices[vi * 3 + 0] == v_x
                            && vertices[vi * 3 + 1] == v_y
                            && vertices[vi * 3 + 2] == v_z
                        {
                            indices.push(vi as u32);
                            break;
                        }
                    }
                    if index_len == indices.len() {
                        vertices.push(v_x);
                        vertices.push(v_y);
                        vertices.push(v_z);
                        indices.push(((vertices.len() / 3) - 1) as u32)
                    }
                }
            }
        }

        // Update indices
        self.indices = indices.len() as u32;

        // println!("V {} I {}", vertices.len() * 4, indices.len() * 4);
        // Maybe i should write the vertex_buffer every frame but
        // instead write it all once and only update the index buffer?

        queue.write_buffer(
            &self.chunk_vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices),
        );
        queue.write_buffer(&self.chunk_index_buffer, 0, bytemuck::cast_slice(&indices));
    }
    pub fn get_bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("chunk_bind_group"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // wgpu::BindGroupLayoutEntry {
                //     binding: 1,
                //     visibility: wgpu::ShaderStages::VERTEX,
                //     ty: wgpu::BindingType::Buffer {
                //         ty: wgpu::BufferBindingType::Storage { read_only: true },
                //         has_dynamic_offset: false,
                //         min_binding_size: None,
                //     },
                //     count: None,
                // },
            ],
        }
    }
    pub fn new(
        x: i32,
        y: i32,
        noise_data: &Vec<f32>,
        device: &wgpu::Device,
        chunk_data_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let step = 4 as usize;

        // Cpu representation
        let mut blocks: BlockVec = vec![];
        // Data representation to send to gpu
        // let mut block_data: Vec<u32> = vec![0; buffer_size as usize];

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut sample_x = (x * CHUNK_SIZE as i32) + i as i32 % NOISE_SIZE as i32;
                let mut sample_y = (y * CHUNK_SIZE as i32) + j as i32 % NOISE_SIZE as i32;
                // Wrap around if negative chunk coordinate
                if sample_x < 0 {
                    sample_x =
                        NOISE_SIZE as i32 + (sample_x % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
                }
                if sample_y < 0 {
                    sample_y =
                        NOISE_SIZE as i32 + (sample_y % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
                }

                let y_top =
                    (noise_data[((sample_y * NOISE_SIZE as i32) + sample_x) as usize] + 1.0) * 0.5;
                let y_top = (f32::powf(100.0, y_top) - 1.0) as u32;

                for y in 0..=y_top {
                    let mut block_type = match BlockType::from_y_position(y) {
                        BlockType::Dirt if y == y_top => BlockType::Grass,
                        b => b,
                    };

                    let block = Rc::new(RefCell::new(Block {
                        faces: None,
                        position: glam::vec3(i as f32, y as f32, j as f32),
                        block_type: BlockType::Dirt,
                    }));

                    let face_directions = FaceDirections::all()
                        .iter()
                        .map(|face_dir| BlockFace {
                            block: block.clone(),
                            face_direction: *face_dir,
                            is_visible: true,
                        })
                        .collect::<Vec<_>>();
                    block.borrow_mut().faces = Some(face_directions);
                    blocks.push(block)
                }
            }
        }
        let blocks_map = Self::build_blocks_map(&blocks);

        let chunk_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            // This is probably more than needed
            size: (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * CHUNK_HEIGHT as u32) as u64 * 3,
            label: Some(&format!("chunk-vertex-{x}-{y}")),
            mapped_at_creation: false,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let chunk_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            // This is probably more than needed
            size: (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * CHUNK_HEIGHT as u32) as u64 * 3,
            label: Some(&format!("chunk-index-{x}-{y}")),
            mapped_at_creation: false,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        });

        let chunk_position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[x, y]),
            label: Some(&format!("chunk-position-{x}-{y}")),
            usage: BufferUsages::UNIFORM,
        });

        let chunk_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: chunk_data_layout,
            label: Some(&format!("chunk-bg-{x}-{y}")),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: chunk_position_buffer.as_entire_binding(),
            }],
        });

        Self {
            x,
            y,
            chunk_bind_group,
            chunk_position_buffer,
            chunk_index_buffer,
            chunk_vertex_buffer,
            blocks,
            blocks_map,
            indices: 0,
        }
    }
}
