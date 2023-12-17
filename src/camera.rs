use glam::{vec2, vec3, Mat2, Vec2, Vec3};

const SENSITIVITY: f32 = 0.001;
const CAMERA_SPEED: f32 = 100.0;

pub struct CameraController {
    speed: Vec3,
}

pub struct Camera {
    pub eye: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub aspect_ratio: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub needs_update: bool,
}

impl Camera {
    pub fn build_view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(self.eye, self.eye + self.calc_target(), glam::Vec3::Y)
    }
    pub fn build_projection_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_rh(self.fovy, self.aspect_ratio, self.znear, self.zfar)
    }

    pub fn calc_target(&self) -> glam::Vec3 {
        let mut direction = glam::Vec3::ZERO;

        direction.x = f32::cos(self.yaw) * f32::cos(self.pitch);
        direction.y = f32::sin(self.pitch);
        direction.z = f32::sin(self.yaw) * f32::cos(self.pitch);

        direction.normalize()
    }

    // target only moves in y and x direction
    pub fn move_target(&mut self, direction: &Vec2) {
        self.yaw -= direction.x * SENSITIVITY;
        self.pitch += direction.y * SENSITIVITY;

        self.needs_update = true;
    }
    pub fn move_camera(&mut self, direction: &Vec3, delta_time: f32) {
        println!("MOVING {:?} - {delta_time}", self.calc_target());
        let forward = self.calc_target();

        // z axis
        if direction.z > 0.0 {
            self.eye += forward * CAMERA_SPEED * delta_time;
        } else if direction.z < 0.0 {
            self.eye -= forward * CAMERA_SPEED * delta_time;
        }

        let right = Vec3::cross(forward, Vec3::Y);

        if direction.x > 0.0 {
            println!("RIGHT {right:?}");
            self.eye += right * CAMERA_SPEED * delta_time;
        } else if direction.x < 0.0 {
            self.eye -= right * CAMERA_SPEED * delta_time;
        }

        let up = Vec3::cross(right, forward);

        if direction.y > 0.0 {
            self.eye += up * CAMERA_SPEED * delta_time;
        } else if direction.y < 0.0 {
            self.eye -= up * CAMERA_SPEED * delta_time;
        }

        self.needs_update = true
    }
}
