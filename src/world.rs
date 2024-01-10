use std::{
    borrow::Borrow,
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

use glam::{vec3, Vec3};
use rand::random;
use wgpu::{
    util::DeviceExt, BindGroupLayout, BindGroupLayoutDescriptor, BufferUsages, TextureViewDimension,
};

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
pub const CHUNKS_PER_ROW: u32 = 20;
pub const CHUNKS_REGION: u32 = CHUNKS_PER_ROW * CHUNKS_PER_ROW;

type BlockMap = HashMap<i32, HashMap<i32, HashMap<i32, Rc<RefCell<Block>>>>>;
type BlockVec = Vec<Rc<RefCell<Block>>>;

type VMapValue = HashMap<FaceDirections, [u32; 6]>;
type VMap = HashMap<(u32, u32, u32), VMapValue>;
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
    // pub chunk_vertex_buffer: wgpu::Buffer,
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
    fn opposite(&self) -> FaceDirections {
        match self {
            FaceDirections::Back => FaceDirections::Front,
            FaceDirections::Bottom => FaceDirections::Top,
            FaceDirections::Top => FaceDirections::Bottom,
            FaceDirections::Front => FaceDirections::Back,
            FaceDirections::Left => FaceDirections::Right,
            FaceDirections::Right => FaceDirections::Left,
        }
    }
    fn get_normal_vector(&self) -> glam::Vec3 {
        match self {
            FaceDirections::Back => glam::vec3(0.0, 0.0, 1.0),
            FaceDirections::Bottom => glam::vec3(0.0, -1.0, 0.0),
            FaceDirections::Top => glam::vec3(0.0, 1.0, 0.0),
            FaceDirections::Front => glam::vec3(0.0, 0.0, -1.0),
            FaceDirections::Left => glam::vec3(-1.0, 0.0, 0.0),
            FaceDirections::Right => glam::vec3(1.0, 0.0, 0.0),
        }
    }
    fn get_indices(&self) -> [u32; 6] {
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
    pub chunk_vertex_buffer: wgpu::Buffer,
    // This would translate to the for now hard coded edge vectors in the pnoise algo
    pub vertex_map: VMap,
    pub seed: u32,
    pub noise_data: Vec<f32>,
    pub chunk_data_layout: wgpu::BindGroupLayout,
}

fn create_face_vertices(
    indices: [u32; 6],
    offset: &glam::Vec3,
    vertices: &mut Vec<[f32; 3]>,
) -> [u32; 6] {
    let mut i = 0;
    // There should be always 4 indices
    let mut unique_indices: Vec<u32> = Vec::with_capacity(4);
    let mut indices_map: [u32; 6] = [0, 0, 0, 0, 0, 0];

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

    let mut new_vertices: Vec<_> = unique_indices
        .iter()
        .map(|index| {
            [
                CUBE_VERTEX[(*index as usize * 3 + 0) as usize] + offset.x,
                CUBE_VERTEX[(*index as usize * 3 + 1) as usize] + offset.y,
                CUBE_VERTEX[(*index as usize * 3 + 2) as usize] + offset.z,
            ]
        })
        .collect();

    vertices.append(&mut new_vertices);

    // 4 Vertices added per face
    let vertex_offset = (vertices.len() - 4) as u32;
    indices_map.iter_mut().for_each(|i| *i += vertex_offset);

    indices_map
}

impl World {
    pub fn update_current_chunk_buffer(&self, chunk: &Chunk, state: &State) {
        // todo!()
    }
    // TODO: im generating much 4x~ more vertices than needed
    pub fn create_all_chunk_vertices() -> (Vec<[f32; 3]>, VMap) {
        let mut v_map: VMap = HashMap::new();
        let mut v: Vec<[f32; 3]> = vec![];

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_HEIGHT as u32 {
                    // Build all y coords

                    let mut lm: VMapValue = HashMap::new();
                    lm.insert(
                        FaceDirections::Top,
                        create_face_vertices(
                            FaceDirections::Top.get_indices(),
                            &glam::vec3(x as f32, y as f32, z as f32),
                            &mut v,
                        ),
                    );
                    lm.insert(
                        FaceDirections::Bottom,
                        create_face_vertices(
                            FaceDirections::Bottom.get_indices(),
                            &glam::vec3(x as f32, y as f32, z as f32),
                            &mut v,
                        ),
                    );

                    // Build rest of coords
                    let t = lm.get(&FaceDirections::Top).unwrap();
                    let b = lm.get(&FaceDirections::Bottom).unwrap();

                    let left_face = [b[0], t[1], t[0], b[0], t[0], b[1]];
                    let right_face = [b[2], t[5], t[2], b[2], t[2], b[5]];
                    let front_face = [b[1], t[0], t[5], b[1], t[5], b[2]];
                    let back_face = [b[5], t[2], t[1], b[5], t[1], b[0]];
                    lm.insert(FaceDirections::Left, left_face);
                    lm.insert(FaceDirections::Right, right_face);
                    lm.insert(FaceDirections::Front, front_face);
                    lm.insert(FaceDirections::Back, back_face);

                    v_map.insert((x, y, z), lm);
                }
            }
        }
        (v, v_map)
    }
    pub fn init_world(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
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

        for j in -lb..=ub {
            for i in -lb..=ub {
                chunks.push(Chunk::new(i, j, &noise_data, device, &chunk_data_layout));
            }
        }

        let (all_chunk_vertices, vertex_map) = Self::create_all_chunk_vertices();

        let mut indices_added: Vec<u32> = vec![];
        for chunk in chunks.iter() {
            indices_added.push(chunk.build_mesh(queue, &vertex_map, &chunks));
        }
        // borrow checker workaround :\
        for (i, chunk) in chunks.iter_mut().enumerate() {
            chunk.indices = indices_added[i];
        }

        let chunk_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&all_chunk_vertices),
            label: Some("chunk_vertices"),
            usage: BufferUsages::VERTEX,
        });
        // chunks.push(Chunk::new(0, 1, &noise_data, device, &chunk_data_layout));

        Self {
            chunk_data_layout,
            chunk_vertex_buffer,
            chunks,
            noise_data,
            vertex_map,
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
            let blockbrw = block.as_ref().borrow();
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
    pub fn is_outside_chunk(position: &glam::Vec3) -> bool {
        if position.x < 0.0
            || position.x >= CHUNK_SIZE as f32
            || position.z < 0.0
            || position.z >= CHUNK_SIZE as f32
        {
            true
        } else {
            false
        }
    }
    pub fn is_outside_bounds(position: &glam::Vec3) -> bool {
        if position.y < 0.0 {
            true
        } else {
            false
        }
    }
    // Returns the number of indices added to the chunk - it would've been better to be a mutable method but i can't do it because of borrow checker
    // TODO: Refactor this
    pub fn build_mesh(
        &self,
        queue: &wgpu::Queue,
        vertex_map: &VMap,
        all_chunks: &Vec<Chunk>,
    ) -> u32 {
        let mut indices: Vec<u32> = vec![];

        for block in self.blocks.iter() {
            {
                let mut blockbrw = block.as_ref().borrow_mut();
                let cube_pos = blockbrw.position.clone();
                let faces = blockbrw.faces.as_mut().unwrap();
                for face in faces.iter_mut() {
                    {
                        let face_chunk_pos = face.face_direction.get_normal_vector() + cube_pos;

                        if Chunk::is_outside_bounds(&face_chunk_pos)
                            || self.exists_block_at(&face_chunk_pos)
                        {
                            face.is_visible = false;
                        } else {
                            if Chunk::is_outside_chunk(&face_chunk_pos) {
                                let target_chunk_y = self.y
                                    + (f32::floor(face_chunk_pos.z / CHUNK_SIZE as f32) as i32);
                                let target_chunk_x = self.x
                                    + (f32::floor(face_chunk_pos.x / CHUNK_SIZE as f32) as i32);

                                let target_chunk_block = glam::vec3(
                                    (face_chunk_pos.x + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                                    face_chunk_pos.y,
                                    (face_chunk_pos.z + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                                );
                                let target_chunk = all_chunks.iter().find(|chunk| {
                                    chunk.x == target_chunk_x && chunk.y == target_chunk_y
                                });
                                match target_chunk {
                                    Some(target_chunk) => {
                                        if target_chunk.exists_block_at(&target_chunk_block) {
                                            face.is_visible = false
                                        } else {
                                            face.is_visible = true
                                        }
                                    }
                                    None => face.is_visible = true,
                                }
                            } else {
                                face.is_visible = true;
                            }
                        }
                    }

                    if face.is_visible {
                        let ci = vertex_map
                            .get(&(cube_pos.x as u32, cube_pos.y as u32, cube_pos.z as u32))
                            .expect("Every cube position should be defined");
                        let ind = ci
                            .get(&face.face_direction)
                            .expect("Every cube direction should be defined");
                        indices.append(&mut ind.to_vec());
                    }
                }
            }
        }

        queue.write_buffer(&self.chunk_index_buffer, 0, bytemuck::cast_slice(&indices));

        indices.len() as u32
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

        // let chunk_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //     // This is probably more than needed
        //     size: (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * CHUNK_HEIGHT as u32) as u64 * 3,
        //     label: Some(&format!("chunk-vertex-{x}-{y}")),
        //     mapped_at_creation: false,
        //     usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        // });
        let chunk_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            // This is more than needed but its the number of blocks * number of indices per face * number of faces
            size: (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * CHUNK_HEIGHT as u32) as u64 * 6 * 6,
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
            // chunk_vertex_buffer,
            blocks,
            blocks_map,
            indices: 0,
        }
    }
}
