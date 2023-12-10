use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
}

#[rustfmt::skip]
pub fn create_triangle() -> Vec<Vertex> {
    vec![
        Vertex { pos: [ -0.5, 0.5, 0.0 ]},
        Vertex { pos: [ 0.0, -1.0, 0.0 ]},
        Vertex { pos: [ 0.5, 0.5, 0.0 ]}
    ]
}
