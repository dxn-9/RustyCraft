use std::{borrow::BorrowMut, cell::RefCell, collections::HashMap, rc::Rc};

// #[derive(Debug)]
struct Cube {
    faces: Option<Vec<CubeFace>>,
    position: glam::Vec3,
}

// #[derive(Debug)]
struct CubeFace {
    indices: [u32; 6],
    face_normal: glam::Vec3,
    cube: Rc<RefCell<Cube>>,
    tex_offset: glam::Vec2,
    is_visible: bool,
}
// impl CubeFace {}

pub static CHUNK_SIZE: u8 = 16;
pub static CHUNK_HEIGHT: u8 = u8::MAX;

pub struct Chunk {
    pub x: i32,
    pub y: i32,
    pub chunk_mesh_buffer: wgpu::Buffer,
    pub chunk_index_buffer: wgpu::Buffer,
    pub cubes: Vec<Cube>,
    pub visible_faces: Vec<CubeFace>,
}
impl Chunk {
    pub fn update_visible_faces(&mut self, player_pos: glam::Vec3) {
        self.visible_faces.clear();

        let base_pos = glam::vec3(
            (self.x * CHUNK_SIZE as i32) as f32,
            0.0,
            (self.y * CHUNK_SIZE as i32) as f32,
        );

        for y in (0..CHUNK_HEIGHT).rev() {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let cube = &self.cubes[(y as usize
                        * (CHUNK_SIZE as usize * CHUNK_SIZE as usize)
                        + (z as usize * CHUNK_SIZE as usize)
                        + x as usize) as usize];
                    let cube_world_pos = cube.position + base_pos;

                    let cube_to_player = player_pos - cube_world_pos;

                    for face in cube.faces.as_ref().unwrap().iter() {
                        if glam::Vec3::dot(
                            (face.face_normal + cube_world_pos).normalize(),
                            (cube_to_player).normalize(),
                        ) >= 0.0
                        {
                            // self.visible_faces.push(face)
                        }
                        // if glam::Vec3::dot()
                    }
                }
            }
            // for 0 in self.cubes {}
        }
    }
    // pub fn build_mesh(&mut self) {
    //     let indices: Vec<u32> = vec![];
    //     let mut cube = Cube {
    //         faces: None,
    //         position: glam::vec3(0.0, 0.0, 0.0),
    //     };

    //     let cube_rf = Rc::new(RefCell::new(cube));
    //     let face_directions = [
    //         CubeFaceDirections::Front,
    //         CubeFaceDirections::Left,
    //         CubeFaceDirections::Back,
    //     ]
    //     .iter()
    //     .map(|dir| CubeFace {
    //         cube: cube_rf.clone(),
    //         face_normal: dir.get_normal_vector(),
    //         indices: dir.get_indices(),
    //         tex_offset: glam::vec2(0.0, 0.0),
    //         is_visible: true,
    //     })
    //     .collect::<Vec<_>>();
    //     {
    //         cube_rf.borrow_mut().faces = Some(face_directions);
    //     }

    //     // let f
    //     // let vertex_data:

    //     // for face in self.visible_faces.iter() {
    //     // }
    // }
}

type BlockMap = HashMap<i32, HashMap<i32, HashMap<i32, Rc<RefCell<Cube>>>>>;
fn exists_block_at(block_map: &BlockMap, position: &glam::Vec3) -> bool {
    match block_map.get(&(position.y as i32)) {
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

pub fn build_mesh(device: &wgpu::Device, player_pos: &glam::Vec3) -> (wgpu::Buffer, usize) {
    let mut cubes: Vec<_> = [
        glam::vec3(0.0, 0.0, 0.0),
        glam::vec3(1.0, 0.0, 0.0),
        glam::vec3(-1.0, 0.0, 0.0),
        glam::vec3(-1.0, 1.0, 0.0),
    ]
    .iter()
    .map(|p| {
        let mut cube = Rc::new(RefCell::new(Cube {
            faces: None,
            position: p.clone(),
        }));

        let face_directions = [
            CubeFaceDirections::Front,
            CubeFaceDirections::Back,
            CubeFaceDirections::Left,
            CubeFaceDirections::Right,
            CubeFaceDirections::Top,
            CubeFaceDirections::Bottom,
        ]
        .iter()
        .map(|dir| CubeFace {
            cube: cube.clone(),
            face_normal: dir.get_normal_vector(),
            indices: dir.get_indices(),
            tex_offset: glam::vec2(0.0, 0.0),
            is_visible: true,
        })
        .collect::<Vec<_>>();
        {
            cube.as_ref().borrow_mut().faces = Some(face_directions);
        }

        cube
    })
    .collect();
    // Build map
    let mut blocks_map: BlockMap = HashMap::new();
    for cube in cubes.iter() {
        let cubebrw = cube.borrow();
        let x_map = match blocks_map.get_mut(&(cubebrw.position.y as i32)) {
            Some(x_map) => x_map,
            None => {
                blocks_map.insert(cubebrw.position.y as i32, HashMap::new());
                blocks_map.get_mut(&(cubebrw.position.y as i32)).unwrap()
            }
        };
        let z_map = match x_map.get_mut(&(cubebrw.position.x as i32)) {
            Some(z_map) => z_map,
            None => {
                x_map.insert(cubebrw.position.x as i32, HashMap::new());
                x_map.get_mut(&(cubebrw.position.x as i32)).unwrap()
            }
        };
        match z_map.get_mut(&(cubebrw.position.z as i32)) {
            Some(_) => panic!("Cannot have more than 1 block in the same place"),
            None => {
                z_map.insert(cubebrw.position.z as i32, cube.clone());
            }
        }
    }

    let mut vertices: Vec<f32> = vec![];
    // Update visible faces
    for (i, cube) in cubes.iter().enumerate() {
        {
            let mut cube_brw = cube.as_ref().borrow_mut();
            let cube_pos = cube_brw.position.clone();
            let faces = cube_brw.faces.as_mut().unwrap();
            for face in faces.iter_mut() {
                let face_world_pos = face.face_normal + cube_pos;
                if exists_block_at(&blocks_map, &face_world_pos) {
                    face.is_visible = false
                } else {
                    let face_to_player_dir = (player_pos.clone() - face_world_pos).normalize();
                    if face.face_normal.dot(face_to_player_dir) < 0.0 {
                        face.is_visible = false
                    }
                }
            }
        }
        let cube_brw = cube.as_ref().borrow();
        let visible_faces: Vec<_> = cube_brw
            .faces
            .as_ref()
            .unwrap()
            .iter()
            .filter(|face| face.is_visible)
            .collect();

        for visible_face in visible_faces.iter() {
            let offset = visible_face.cube.as_ref().borrow().position;
            for i in visible_face.indices.iter() {
                vertices.push(CUBE_VERTEX[(*i * 3 + 0) as usize] + offset.x);
                vertices.push(CUBE_VERTEX[(*i * 3 + 1) as usize] + offset.y);
                vertices.push(CUBE_VERTEX[(*i * 3 + 2) as usize] + offset.z);
            }
        }
    }
    println!("{:?}", vertices);
    use wgpu::util::DeviceExt;
    (
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vertices),
            label: Some("a"),
            usage: wgpu::BufferUsages::VERTEX,
        }),
        vertices.len() / 3,
    )
    // todo!()
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
#[rustfmt::skip]
pub const CUBE_VERTEX_NON_INDEXED: [f32; 18] = [
    -0.5, -0.5, -0.5,
    -0.5, 0.5, -0.5,
    0.5, 0.5, -0.5,

    -0.5, -0.5, -0.5,
    0.5, 0.5, -0.5,
    0.5, -0.5, -0.5,

];

enum CubeFaceDirections {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}
impl CubeFaceDirections {
    fn get_normal_vector(&self) -> glam::Vec3 {
        match self {
            CubeFaceDirections::Back => glam::vec3(0.0, 0.0, 1.0),
            CubeFaceDirections::Bottom => glam::vec3(0.0, -1.0, 0.0),
            CubeFaceDirections::Top => glam::vec3(0.0, 1.0, 0.0),
            CubeFaceDirections::Front => glam::vec3(0.0, 0.0, -1.0),
            CubeFaceDirections::Left => glam::vec3(1.0, 0.0, 0.0),
            CubeFaceDirections::Right => glam::vec3(-1.0, 0.0, 0.0),
        }
    }
    fn get_indices(&self) -> [u32; 6] {
        match self {
            CubeFaceDirections::Back => [7, 4, 5, 7, 5, 6],
            CubeFaceDirections::Front => [0, 3, 2, 0, 2, 1],
            CubeFaceDirections::Left => [7, 3, 2, 7, 2, 6],
            CubeFaceDirections::Right => [4, 0, 1, 4, 1, 5],
            CubeFaceDirections::Top => [1, 2, 6, 1, 6, 5],
            CubeFaceDirections::Bottom => [0, 3, 7, 0, 7, 4],
        }
    }
}

#[rustfmt::skip]
pub const CUBE_INDICES: [u32; 36] = [
    // Front face
    0, 3, 2,
    0, 2, 1,
    // Left Face
    7, 3, 2,
    7, 2, 6,
    // Back Face
    7, 4, 5,
    7, 5, 6,
    // Right face
    4, 0, 1,
    4, 1, 5,
    // Top face
    1, 2, 6,
    1, 6, 5,
    // Bottom face
    0, 3, 7,
    0, 7, 4
];
