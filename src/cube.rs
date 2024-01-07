// use std::{borrow::BorrowMut, cell::RefCell, collections::HashMap, rc::Rc};

// enum BlockType {
//     Air,
//     Dirt,
//     Grass,
//     Stone,
// }

// struct Block {
//     faces: Option<Vec<BlockFace>>,
//     position: glam::Vec3,
//     block_type: BlockType,
// }

// struct BlockFace {
//     face_direction: FaceDirections,
//     block: Rc<RefCell<Block>>,
//     is_visible: bool,
// }
// // impl CubeFace {}

// pub static CHUNK_SIZE: u8 = 16;
// pub static CHUNK_HEIGHT: u8 = u8::MAX;

// pub struct Chunk {
//     pub x: i32,
//     pub y: i32,
//     pub chunk_vertex_buffer: wgpu::Buffer,
//     pub chunk_index_buffer: wgpu::Buffer,
//     pub blocks: Vec<Block>,
//     pub blocks_map: BlockMap,
//     pub visible_faces: Vec<BlockFace>,
//     pub is_dirty: bool,
// }
// impl Chunk {
//     pub fn update_visible_faces(&mut self, player_pos: glam::Vec3) {}
// }

// type BlockMap = HashMap<i32, HashMap<i32, HashMap<i32, Rc<RefCell<Block>>>>>;
// fn exists_block_at(block_map: &BlockMap, position: &glam::Vec3) -> bool {
//     match block_map.get(&(position.y as i32)) {
//         Some(x_map) => match x_map.get(&(position.x as i32)) {
//             Some(z_map) => match z_map.get(&(position.z as i32)) {
//                 Some(_) => true,
//                 None => false,
//             },
//             None => false,
//         },
//         None => false,
//     }
// }

// pub fn build_mesh(
//     device: &wgpu::Device,
//     player_pos: &glam::Vec3,
// ) -> (wgpu::Buffer, wgpu::Buffer, usize) {
//     let blocks: Vec<_> = [
//         glam::vec3(0.0, 0.0, 0.0),
//         glam::vec3(1.0, 0.0, 0.0),
//         glam::vec3(-1.0, 0.0, 0.0),
//         glam::vec3(-1.0, 1.0, 0.0),
//     ]
//     .iter()
//     .map(|p| {
//         let block = Rc::new(RefCell::new(Block {
//             faces: None,
//             position: p.clone(),
//             block_type: BlockType::Dirt,
//         }));

//         let face_directions = [
//             FaceDirections::Front,
//             FaceDirections::Back,
//             FaceDirections::Left,
//             FaceDirections::Right,
//             FaceDirections::Top,
//             FaceDirections::Bottom,
//         ]
//         .iter()
//         .map(|face_direction| BlockFace {
//             block: block.clone(),
//             face_direction: *face_direction,
//             is_visible: true,
//         })
//         .collect::<Vec<_>>();
//         {
//             block.as_ref().borrow_mut().faces = Some(face_directions);
//         }

//         block
//     })
//     .collect();
//     // Build map
//     let mut blocks_map: BlockMap = HashMap::new();
//     for block in blocks.iter() {
//         let blockbrw = block.borrow();
//         let x_map = match blocks_map.get_mut(&(blockbrw.position.y as i32)) {
//             Some(x_map) => x_map,
//             None => {
//                 blocks_map.insert(blockbrw.position.y as i32, HashMap::new());
//                 blocks_map.get_mut(&(blockbrw.position.y as i32)).unwrap()
//             }
//         };
//         let z_map = match x_map.get_mut(&(blockbrw.position.x as i32)) {
//             Some(z_map) => z_map,
//             None => {
//                 x_map.insert(blockbrw.position.x as i32, HashMap::new());
//                 x_map.get_mut(&(blockbrw.position.x as i32)).unwrap()
//             }
//         };
//         match z_map.get_mut(&(blockbrw.position.z as i32)) {
//             Some(_) => panic!("Cannot have more than 1 block in the same place"),
//             None => {
//                 z_map.insert(blockbrw.position.z as i32, block.clone());
//             }
//         }
//     }

//     let mut vertices: Vec<f32> = vec![];
//     let mut indices: Vec<u32> = vec![];
//     // Update visible faces
//     for block in blocks.iter() {
//         {
//             let mut blockbrw = block.as_ref().borrow_mut();
//             let cube_pos = blockbrw.position.clone();
//             let faces = blockbrw.faces.as_mut().unwrap();
//             for face in faces.iter_mut() {
//                 let face_world_pos = face.face_direction.get_normal_vector() + cube_pos;
//                 if exists_block_at(&blocks_map, &face_world_pos) {
//                     face.is_visible = false
//                 } else {
//                     let face_to_player_dir = (player_pos.clone() - face_world_pos).normalize();
//                     if face
//                         .face_direction
//                         .get_normal_vector()
//                         .dot(face_to_player_dir)
//                         < 0.0
//                     {
//                         face.is_visible = false
//                     }
//                 }
//             }
//         }
//         let blockbrw = block.as_ref().borrow();
//         let visible_faces: Vec<_> = blockbrw
//             .faces
//             .as_ref()
//             .unwrap()
//             .iter()
//             .filter(|face| face.is_visible)
//             .collect();

//         for visible_face in visible_faces.iter() {
//             let offset = visible_face.block.as_ref().borrow().position;
//             for i in visible_face.face_direction.get_indices().iter() {
//                 let v_x = CUBE_VERTEX[(*i * 3 + 0) as usize] + offset.x;
//                 let v_y = CUBE_VERTEX[(*i * 3 + 1) as usize] + offset.y;
//                 let v_z = CUBE_VERTEX[(*i * 3 + 2) as usize] + offset.z;

//                 let index_len = indices.len();
//                 // This might be better as a hashmap instead of a linear search?
//                 for vi in 0..vertices.len() / 3 {
//                     if vertices[vi * 3 + 0] == v_x
//                         && vertices[vi * 3 + 1] == v_y
//                         && vertices[vi * 3 + 2] == v_z
//                     {
//                         println!("ALREADY PRESENT VERTEX");
//                         indices.push(vi as u32);
//                         break;
//                     }
//                 }
//                 if index_len == indices.len() {
//                     vertices.push(v_x);
//                     vertices.push(v_y);
//                     vertices.push(v_z);
//                     indices.push(((vertices.len() / 3) - 1) as u32)
//                 }
//             }
//         }
//     }
//     println!("VERTEX: {:?}, INDICES {:?}", vertices, indices);
//     use wgpu::util::DeviceExt;
//     (
//         device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             contents: bytemuck::cast_slice(&vertices),
//             label: Some("a"),
//             usage: wgpu::BufferUsages::VERTEX,
//         }),
//         device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             contents: bytemuck::cast_slice(&indices),
//             label: Some("b"),
//             usage: wgpu::BufferUsages::INDEX,
//         }),
//         indices.len(),
//     )
//     // todo!()
// }

// #[rustfmt::skip]
// pub const CUBE_VERTEX: [f32; 24] = [
//     -0.5, -0.5, -0.5,
//     -0.5, 0.5, -0.5,
//     0.5, 0.5, -0.5,
//     0.5, -0.5, -0.5,

//     -0.5, -0.5, 0.5,
//     -0.5, 0.5, 0.5,
//     0.5, 0.5, 0.5,
//     0.5, -0.5, 0.5,
// ];
// #[rustfmt::skip]
// pub const CUBE_VERTEX_NON_INDEXED: [f32; 18] = [
//     -0.5, -0.5, -0.5,
//     -0.5, 0.5, -0.5,
//     0.5, 0.5, -0.5,

//     -0.5, -0.5, -0.5,
//     0.5, 0.5, -0.5,
//     0.5, -0.5, -0.5,

// ];

// #[derive(Clone, Copy)]
// enum FaceDirections {
//     Front,
//     Back,
//     Left,
//     Right,
//     Top,
//     Bottom,
// }
// impl FaceDirections {
//     fn get_normal_vector(&self) -> glam::Vec3 {
//         match self {
//             FaceDirections::Back => glam::vec3(0.0, 0.0, 1.0),
//             FaceDirections::Bottom => glam::vec3(0.0, -1.0, 0.0),
//             FaceDirections::Top => glam::vec3(0.0, 1.0, 0.0),
//             FaceDirections::Front => glam::vec3(0.0, 0.0, -1.0),
//             FaceDirections::Left => glam::vec3(1.0, 0.0, 0.0),
//             FaceDirections::Right => glam::vec3(-1.0, 0.0, 0.0),
//         }
//     }
//     fn get_indices(&self) -> [u32; 6] {
//         match self {
//             FaceDirections::Back => [7, 4, 5, 7, 5, 6],
//             FaceDirections::Front => [0, 3, 2, 0, 2, 1],
//             FaceDirections::Left => [7, 3, 2, 7, 2, 6],
//             FaceDirections::Right => [4, 0, 1, 4, 1, 5],
//             FaceDirections::Top => [1, 2, 6, 1, 6, 5],
//             FaceDirections::Bottom => [0, 3, 7, 0, 7, 4],
//         }
//     }
// }

// #[rustfmt::skip]
// pub const CUBE_INDICES: [u32; 36] = [
//     // Front face
//     0, 3, 2,
//     0, 2, 1,
//     // Left Face
//     7, 3, 2,
//     7, 2, 6,
//     // Back Face
//     7, 4, 5,
//     7, 5, 6,
//     // Right face
//     4, 0, 1,
//     4, 1, 5,
//     // Top face
//     1, 2, 6,
//     1, 6, 5,
//     // Bottom face
//     0, 3, 7,
//     0, 7, 4
// ];
