use std::fs::File;
use std::io::Read;
use std::{cell::RefCell, rc::Rc};

use bytemuck::{Pod, Zeroable};
use obj::Vertex;
use wgpu::{include_wgsl, util::DeviceExt, BindGroup, Buffer, Face, RenderPipeline};

use crate::{
    blocks::block::Block,
    material::{Material, Texture},
    player::Camera,
    state::State,
};

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
