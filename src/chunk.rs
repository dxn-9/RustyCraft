use crate::persistence::{Loadable, Saveable};
use crate::player::Player;
use crate::utils::math_utils::Plane;
use crate::world::{ChunkMap, RNG_SEED, WATER_HEIGHT_LEVEL};
use crate::{
    blocks::{
        block::{Block, BlockVertexData, FaceDirections},
        block_type::BlockType,
    },
    structures::Structure,
    world::{NoiseData, CHUNK_SIZE, MAX_TREES_PER_CHUNK, NOISE_CHUNK_PER_ROW, NOISE_SIZE},
};

use glam::Vec3;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::any::Any;
use std::error::Error;
use std::sync::{Arc, RwLock};
use wgpu::util::DeviceExt;

pub type BlockVec = Arc<RwLock<Vec<Vec<Option<Arc<RwLock<Block>>>>>>>;

#[derive(Debug)]
pub struct Chunk {
    pub x: i32,
    pub y: i32,
    pub blocks: BlockVec,
    pub indices: u32,
    pub water_indices: u32,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub noise_data: Arc<NoiseData>,
    pub chunk_bind_group: wgpu::BindGroup,
    pub chunk_position_buffer: wgpu::Buffer,
    pub chunk_index_buffer: Option<wgpu::Buffer>,
    pub chunk_vertex_buffer: Option<wgpu::Buffer>,
    pub chunk_water_vertex_buffer: Option<wgpu::Buffer>,
    pub chunk_water_index_buffer: Option<wgpu::Buffer>,
    pub outside_blocks: Vec<Arc<RwLock<Block>>>,
    pub visible: bool,
    pub modified: bool, // if true, it will be saved
}

impl Chunk {
    pub fn add_block(&mut self, block: Arc<RwLock<Block>>, modify_status: bool) {
        let block_borrow = block.read().unwrap();
        let block_position = block_borrow.position;
        std::mem::drop(block_borrow);
        let mut blocks_borrow = self.blocks.write().unwrap();

        let y_blocks = blocks_borrow
            .get_mut(((block_position.x * CHUNK_SIZE as f32) + block_position.z) as usize)
            .expect("Cannot add oob block");

        if block_position.y as usize >= y_blocks.len() {
            y_blocks.resize(block_position.y as usize + 1, None);
        }

        y_blocks[block_position.y as usize] = Some(block);
        if modify_status {
            self.modified = true;
        }
    }
    pub fn remove_block(&mut self, block_r_position: &Vec3) {
        let mut blocks_borrow = self.blocks.write().unwrap();
        let y_blocks = blocks_borrow
            .get_mut(((block_r_position.x * CHUNK_SIZE as f32) + block_r_position.z) as usize)
            .expect("Cannot delete oob block");
        y_blocks[block_r_position.y as usize] = None;
        self.modified = true;
    }
    pub fn block_type_at(&self, position: &glam::Vec3) -> Option<BlockType> {
        let block = self.get_block_at_relative(position)?;
        let block_type = block.read().unwrap().block_type;
        Some(block_type.clone())
    }
    pub fn exists_block_at(&self, position: &glam::Vec3) -> bool {
        if let Some(y_blocks) = self
            .blocks
            .read()
            .unwrap()
            .get(((position.x as u32 * CHUNK_SIZE) + position.z as u32) as usize)
        {
            if let Some(block_opt) = y_blocks.get(position.y as usize) {
                if let Some(_) = block_opt {
                    return true;
                }
            }
        }
        return false;
    }
    pub fn get_block_at_relative(&self, position: &glam::Vec3) -> Option<Arc<RwLock<Block>>> {
        if let Some(y_blocks) = self
            .blocks
            .read()
            .unwrap()
            .get(((position.x * CHUNK_SIZE as f32) + position.z) as usize)
        {
            if let Some(block) = y_blocks.get(position.y as usize)? {
                return Some(Arc::clone(block));
            }
        }
        return None;
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
    /*
    Return tuple:
    0: vertex indices     , 1: water vertex indices
    2: vertex buffer      , 3: index buffer
    4: water vertex buffer, 5: water index buffer */
    pub fn build_mesh(
        &self,
        other_chunks: ChunkMap,
    ) -> (
        u32,
        u32,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
    ) {
        let mut water_vertex: Vec<BlockVertexData> = vec![];
        let mut water_indices: Vec<u32> = vec![];
        let mut vertex: Vec<BlockVertexData> = vec![];
        let mut indices: Vec<u32> = vec![];
        let mut adjacent_chunks: Vec<((i32, i32), BlockVec)> = vec![];

        for x in self.x - 1..=self.x + 1 {
            for y in self.y - 1..=self.y + 1 {
                if let Some(chunk) = other_chunks.read().unwrap().get(&(x, y)) {
                    let chunk_read = chunk.read().unwrap();
                    adjacent_chunks.push(((x, y), chunk_read.blocks.clone()));
                }
            }
        }

        for region in self.blocks.read().unwrap().iter() {
            for y in 0..region.len() {
                if let Some(block_ptr) = &region[y] {
                    let block = block_ptr.read().unwrap();
                    let position = block.position;
                    let faces = FaceDirections::all();

                    for face in faces.iter() {
                        // For water block types, we only care about the top face
                        if block.block_type == BlockType::Water && *face != FaceDirections::Top {
                            continue;
                        }
                        let mut is_visible = true;
                        let face_position = face.get_normal_vector() + position;

                        if Chunk::is_outside_bounds(&face_position) {
                            is_visible = false;
                        } else if Chunk::is_outside_chunk(&face_position) {
                            let target_chunk_x =
                                self.x + (f32::floor(face_position.x / CHUNK_SIZE as f32) as i32);
                            let target_chunk_y =
                                self.y + (f32::floor(face_position.z / CHUNK_SIZE as f32) as i32);

                            let target_block = glam::vec3(
                                (face_position.x + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                                face_position.y,
                                (face_position.z + CHUNK_SIZE as f32) % CHUNK_SIZE as f32,
                            );

                            let other_chunks_brw = other_chunks.read().unwrap();
                            let target_chunk =
                                other_chunks_brw.get(&(target_chunk_x, target_chunk_y));
                            // let target_chunk = other_chunks.iter().find(|c| {
                            //     let c = c.read().unwrap();
                            //     c.x == target_chunk_x && c.y == target_chunk_y
                            // });
                            // If there's a chunk loaded in memory then check that, else it means we're on a edge and we can
                            // Calculate the block's height when the chunk gets generated
                            // TODO: Check for saved file chunk
                            match target_chunk {
                                Some(chunk) => {
                                    let chunk = chunk.read().unwrap();
                                    if chunk.exists_block_at(&target_block) {
                                        is_visible = false;

                                        if chunk.block_type_at(&target_block)
                                            == Some(BlockType::Water)
                                            && block.block_type != BlockType::Water
                                        {
                                            is_visible = true;
                                        }
                                    }
                                }
                                None => {
                                    let h = Chunk::get_height_value(
                                        target_chunk_x,
                                        target_chunk_y,
                                        target_block.x as u32,
                                        target_block.z as u32,
                                        self.noise_data.clone(),
                                    );

                                    if face_position.y as u32 <= h {
                                        is_visible = false
                                    };
                                }
                            }
                        } else if self.exists_block_at(&face_position) {
                            is_visible = false;
                            // This can be a oneline if, but it gets very hard to read
                            if self.block_type_at(&face_position) == Some(BlockType::Water)
                                && block.block_type != BlockType::Water
                            {
                                is_visible = true;
                            }
                        }

                        if is_visible {
                            let (mut vertex_data, index_data) =
                                face.create_face_data(block_ptr.clone(), &adjacent_chunks);
                            match block.block_type {
                                BlockType::Water => {
                                    water_vertex.append(&mut vertex_data);
                                    let indices_offset = water_vertex.len() as u32 - 4;
                                    water_indices.append(
                                        &mut index_data
                                            .iter()
                                            .map(|i| i + indices_offset)
                                            .collect(),
                                    )
                                }
                                _ => {
                                    vertex.append(&mut vertex_data);
                                    let indices_offset = vertex.len() as u32 - 4;
                                    indices.append(
                                        &mut index_data
                                            .iter()
                                            .map(|i| i + indices_offset)
                                            .collect(),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }

        let chunk_vertex_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&vertex),
                    label: Some(&format!("chunk-vertex-{}-{}", self.x, self.y)),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        let chunk_index_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&indices),
                    label: Some(&format!("chunk-vertex-{}-{}", self.x, self.y)),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });

        let chunk_water_vertex_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&water_vertex),
                    label: Some(&format!("water-chunk-vertex-{}-{}", self.x, self.y)),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        let chunk_water_index_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&water_indices),
                    label: Some(&format!("water-chunk-vertex-{}-{}", self.x, self.y)),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });

        (
            indices.len() as u32,
            water_indices.len() as u32,
            chunk_vertex_buffer,
            chunk_index_buffer,
            chunk_water_vertex_buffer,
            chunk_water_index_buffer,
        )
    }
    pub fn get_bind_group_layout() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("chunk_bind_group"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        }
    }

    pub fn get_height_value(
        chunk_x: i32,
        chunk_y: i32,
        x: u32,
        z: u32,
        noise_data: Arc<NoiseData>,
    ) -> u32 {
        let mut x = (chunk_x * CHUNK_SIZE as i32) + x as i32 % NOISE_SIZE as i32;
        let mut z = (chunk_y * CHUNK_SIZE as i32) + z as i32 % NOISE_SIZE as i32;

        if x < 0 {
            x = NOISE_SIZE as i32 + (x % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
        }
        if z < 0 {
            z = NOISE_SIZE as i32 + (z % (NOISE_CHUNK_PER_ROW * CHUNK_SIZE) as i32);
        }

        let y_top = (noise_data[((z * (NOISE_SIZE - 1) as i32) + x) as usize] + 1.0) * 0.5;
        return (f32::powf(100.0, y_top) - 1.0) as u32;
    }

    pub fn create_blocks_data(chunk_x: i32, chunk_y: i32, noise_data: Arc<NoiseData>) -> BlockVec {
        let size = (CHUNK_SIZE * CHUNK_SIZE) as usize;
        let blocks: BlockVec = Arc::new(RwLock::new(vec![
            Vec::with_capacity(
                WATER_HEIGHT_LEVEL as usize
            );
            size
        ]));

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let y_top = Chunk::get_height_value(chunk_x, chunk_y, x, z, noise_data.clone());

                let curr = &mut blocks.write().unwrap()[((x * CHUNK_SIZE) + z) as usize];

                for y in 0..=y_top {
                    let block_type = match BlockType::from_position(x, y, z) {
                        BlockType::Dirt if y == y_top => BlockType::Grass,
                        b => b,
                    };

                    let block = Arc::new(RwLock::new(Block::new(
                        glam::vec3(x as f32, y as f32, z as f32),
                        (chunk_x, chunk_y),
                        block_type,
                    )));

                    curr.push(Some(block.clone()));
                }
                // Fill with water empty blocks
                for y in curr.len()..=(WATER_HEIGHT_LEVEL as usize) {
                    if let None = curr.get(y) {
                        let block = Arc::new(RwLock::new(Block::new(
                            glam::vec3(x as f32, y as f32, z as f32),
                            (chunk_x, chunk_y),
                            BlockType::Water,
                        )));
                        curr.push(Some(block));
                    }
                }
            }
        }

        blocks
    }
    // TODO: Use white noise + check that the tree is not being placed on water.
    pub fn place_trees(&mut self) {
        let mut rng = StdRng::seed_from_u64((self.x * 10 * self.y) as u64 + RNG_SEED);
        let number_of_trees = rng.gen::<f32>();
        let mut number_of_trees = f32::floor(number_of_trees * MAX_TREES_PER_CHUNK as f32) as u32;

        // Do a max 100 retries
        for _ in 0..100 {
            if number_of_trees == 0 {
                break;
            }
            let mut tree_blocks = vec![];
            {
                let x = f32::floor(rng.gen::<f32>() * CHUNK_SIZE as f32) as usize;
                let z = f32::floor(rng.gen::<f32>() * CHUNK_SIZE as f32) as usize;

                let blocks_read = self.blocks.read().unwrap();
                let block_column = blocks_read
                    .get((x * CHUNK_SIZE as usize) + z)
                    .expect("TODO: fix this case");
                let highest_block = block_column
                    .last()
                    .expect("TODO: Fix this case -h")
                    .as_ref()
                    .unwrap()
                    .read()
                    .unwrap();
                if highest_block.block_type == BlockType::Water
                    || highest_block.block_type == BlockType::Leaf
                {
                    continue;
                }
                let highest_block_position = highest_block.absolute_position.clone();

                tree_blocks.append(&mut crate::structures::Tree::get_blocks(
                    highest_block_position,
                ));
                number_of_trees -= 1;
            }
            for block in tree_blocks.iter() {
                let block_brw = block.read().unwrap();
                let block_chunk = block_brw.get_chunk_coords();
                if block_chunk == (self.x, self.y) {
                    self.add_block(block.clone(), false);
                } else {
                    self.outside_blocks.push(block.clone())
                }
            }
        }
    }
    // https://www.lighthouse3d.com/tutorials/view-frustum-culling/
    // Note: we don't compute the top and bottom planes, only far,near,right,left
    pub fn is_visible(&self, player: Arc<RwLock<Player>>) -> bool {
        let player = player.read().unwrap();
        let forward = player.camera.get_forward_dir();
        let right = player.camera.get_right_dir();
        let halfvside = player.camera.zfar / f32::tan(player.camera.fovy / 2.0);
        let halfhside = halfvside * player.camera.aspect_ratio;
        let front_mult_far = player.camera.zfar * forward;

        let chunk_points = [
            (
                (self.x as f32) * CHUNK_SIZE as f32,
                (self.y as f32) * CHUNK_SIZE as f32,
            ),
            (
                (self.x as f32 + 1.0) * CHUNK_SIZE as f32,
                (self.y as f32) * CHUNK_SIZE as f32,
            ),
            (
                (self.x as f32) * CHUNK_SIZE as f32,
                (self.y as f32 + 1.0) * CHUNK_SIZE as f32,
            ),
            (
                (self.x as f32 + 1.0) * CHUNK_SIZE as f32,
                (self.y as f32 + 1.0) * CHUNK_SIZE as f32,
            ),
        ];

        let near_plane = Plane {
            point: player.camera.eye + player.camera.znear * forward,
            normal: forward,
        };
        let far_plane = Plane {
            point: player.camera.eye + front_mult_far,
            normal: -forward,
        };
        let right_plane = Plane {
            point: player.camera.eye,
            normal: glam::vec3(0.0, 1.0, 0.0)
                .cross(player.camera.eye - (front_mult_far + right * halfhside))
                .normalize(),
        };
        let left_plane = Plane {
            point: player.camera.eye,
            normal: (player.camera.eye - (front_mult_far - right * halfhside))
                .cross(glam::vec3(0.0, 1.0, 0.0))
                .normalize(),
        };

        // returns true if at least one border of a chunk is visible is inside the frustum
        [far_plane, near_plane, left_plane, right_plane]
            .iter()
            .all(|p| {
                chunk_points.iter().any(|chunk_point| {
                    p.signed_plane_dist(glam::vec3(chunk_point.0, 0.0, chunk_point.1)) >= 0.0
                })
            })
    }

    pub fn new(
        x: i32,
        y: i32,
        noise_data: Arc<NoiseData>,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        chunk_data_layout: Arc<wgpu::BindGroupLayout>,
    ) -> Chunk {
        let mut was_loaded = false;

        let blocks = if let Ok(blocks) = Self::load(Box::new((x, y))) {
            was_loaded = true;
            blocks
        } else {
            Self::create_blocks_data(x, y, noise_data.clone())
        };

        let chunk_position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[x, y]),
            label: Some(&format!("chunk-position-{x}-{y}")),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let chunk_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: chunk_data_layout.as_ref(),
            label: Some(&format!("chunk-bg-{x}-{y}")),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: chunk_position_buffer.as_entire_binding(),
            }],
        });

        let mut chunk = Chunk {
            modified: false,
            chunk_water_index_buffer: None,
            chunk_water_vertex_buffer: None,
            blocks,
            x,
            y,
            device,
            queue,
            noise_data,
            chunk_vertex_buffer: None,
            chunk_index_buffer: None,
            chunk_bind_group,
            chunk_position_buffer,
            indices: 0,
            water_indices: 0,
            outside_blocks: vec![],
            visible: true,
        };

        if !was_loaded {
            chunk.place_trees();
        }
        return chunk;
    }
}

impl Saveable<Chunk> for Chunk {
    fn save(&self) -> Result<(), Box<dyn Error>> {
        if let Ok(_) = std::fs::create_dir("data") {
            println!("Created dir");
        }
        let mut data = String::new();

        for col in self.blocks.read().unwrap().iter() {
            for block in col.iter() {
                if let Some(block_ptr) = block {
                    let blockbrw = block_ptr.read().unwrap();
                    data += &format!(
                        "{},{},{},{}\n",
                        blockbrw.position.x,
                        blockbrw.position.y,
                        blockbrw.position.z,
                        blockbrw.block_type.to_id()
                    );
                }
            }
        }

        let chunk_file_name = format!("data/chunk{}_{}", self.x, self.y);
        std::fs::write(chunk_file_name.clone(), data.as_bytes())?;

        Ok(())
    }
}

impl Loadable<BlockVec> for Chunk {
    fn load(args: Box<dyn Any>) -> Result<BlockVec, Box<dyn Error>> {
        if let Ok(chunk_position) = args.downcast::<(i32, i32)>() {
            for entry in std::fs::read_dir("data")? {
                let file = entry?;
                let filename_chunk = file.file_name();
                let mut coords = filename_chunk
                    .to_str()
                    .unwrap()
                    .split("k")
                    .last()
                    .expect("Invalid filename")
                    .split("_");
                let x = coords.next().unwrap().parse::<i32>()?;
                let y = coords.next().unwrap().parse::<i32>()?;

                let size = (CHUNK_SIZE * CHUNK_SIZE) as usize;
                let blocks: BlockVec = Arc::new(RwLock::new(vec![vec![]; size]));
                if *chunk_position == (x, y) {
                    let file_contents = std::fs::read_to_string(format!("data/chunk{}_{}", x, y))?;
                    for line in file_contents.lines() {
                        let mut i = line.split(",");
                        let bx = i.next().unwrap().parse::<u32>()?;
                        let by = i.next().unwrap().parse::<u32>()?;
                        let bz = i.next().unwrap().parse::<u32>()?;
                        let block_type = i.next().unwrap().parse::<u32>()?;
                        let block_type = BlockType::from_id(block_type);

                        let block = Block::new(
                            glam::vec3(bx as f32, by as f32, bz as f32),
                            (x, y),
                            block_type,
                        );
                        let y_blocks =
                            &mut blocks.write().unwrap()[((bx * CHUNK_SIZE) + bz) as usize];
                        let start_len = y_blocks.len();

                        for i in start_len..=by as usize {
                            if i >= y_blocks.len() {
                                y_blocks.push(None);
                            }
                        }
                        y_blocks[by as usize] = Some(Arc::new(RwLock::new(block)));
                    }
                    return Ok(blocks);
                }
            }
        }
        return Err("Not valid args".into());
    }
}
