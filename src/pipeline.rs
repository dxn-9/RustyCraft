use crate::player::Camera;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct Uniforms {
    pub view: [f32; 16],
    pub projection: [f32; 16],
}

impl From<&Camera> for Uniforms {
    fn from(camera: &Camera) -> Self {
        Self {
            view: *camera.build_view_matrix().as_ref(),
            projection: *camera.build_projection_matrix().as_ref(),
        }
    }
}
