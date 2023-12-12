use glam::Vec3;

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub aspect: f32,
    pub fovy: f32,
}
