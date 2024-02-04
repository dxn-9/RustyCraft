use glam::{vec2, vec3, Mat2, Vec2, Vec3};

use crate::{collision::CollisionBox, world::CHUNK_SIZE};

const SENSITIVITY: f32 = 0.001;
const CAMERA_SPEED: f32 = 10.0;

pub struct CameraController {
    pub movement_vector: Vec3,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            movement_vector: vec3(0.0, 0.0, 0.0),
        }
    }
}
pub struct Player {
    pub camera: Camera,
    pub current_chunk: (i32, i32),
}
impl Player {
    // Position relative to the chunk
    pub fn to_relative_position(&self) -> glam::Vec3 {
        todo!();
        // glam::vec3(
        //     f32::abs(self.camera.eye.x + (CHUNK_SIZE as f32 - 1.0) % CHUNK_SIZE as f32),
        //     self.camera.eye.y,
        //     f32::abs(self.camera.eye.z + (CHUNK_SIZE as f32 - 1.0) % CHUNK_SIZE as f32),
        // )
    }
    pub fn get_collision(&self) -> crate::collision::CollisionBox {
        crate::collision::CollisionBox::new(
            self.camera.eye.x - 0.4,
            self.camera.eye.y - 0.4,
            self.camera.eye.z - 0.4,
            0.8,
            2.0,
            0.8,
        )
    }
    pub fn calc_current_chunk(&self) -> (i32, i32) {
        (
            f32::floor(self.camera.eye.x / CHUNK_SIZE as f32) as i32,
            f32::floor(self.camera.eye.z / CHUNK_SIZE as f32) as i32,
        )
    }

    pub fn move_camera(
        &mut self,
        direction: &Vec3,
        delta_time: f32,
        collisions: &Vec<CollisionBox>,
    ) {
        let mut direction = direction.clone();
        let player_collision = self.get_collision();
        if let Some(collision) = collisions.first() {
            println!(
                "PLAYER POSITION: {:?} COLLISION POSITION {:?} ",
                player_collision, collision
            );

            if let Some(collision) = player_collision.intersects_dir(collision) {
                println!("COLLISION {:?}\n", collision);
                if collision.0 < 0.0 && direction.x < 0.0 {
                    direction.x = 0.0;
                }
                if collision.0 > 0.0 && direction.x > 0.0 {
                    direction.x = 0.0;
                }
                if collision.1 < 0.0 && direction.y < 0.0 {
                    direction.y = 0.0;
                }
                if collision.1 > 0.0 && direction.y > 0.0 {
                    direction.y = 0.0;
                }
                if collision.2 < 0.0 && direction.z < 0.0 {
                    direction.z = 0.0;
                }
                if collision.2 > 0.0 && direction.z > 0.0 {
                    direction.z = 0.0;
                }
            };
        };

        let forward = self.camera.calc_target();

        // z axis
        if direction.z > 0.0 {
            self.camera.eye += forward * CAMERA_SPEED * delta_time;
        } else if direction.z < 0.0 {
            self.camera.eye -= forward * CAMERA_SPEED * delta_time;
        }

        let right = Vec3::cross(forward, Vec3::Y);

        if direction.x > 0.0 {
            self.camera.eye -= right * CAMERA_SPEED * delta_time;
        } else if direction.x < 0.0 {
            self.camera.eye += right * CAMERA_SPEED * delta_time;
        }

        let up = Vec3::cross(right, forward);

        if direction.y > 0.0 {
            self.camera.eye += up * CAMERA_SPEED * delta_time;
        } else if direction.y < 0.0 {
            self.camera.eye -= up * CAMERA_SPEED * delta_time;
        }
        self.camera.needs_update = true;
    }
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
        glam::Mat4::look_at_lh(self.eye, self.eye + self.calc_target(), glam::Vec3::Y)
    }
    pub fn build_projection_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_lh(self.fovy, self.aspect_ratio, self.znear, self.zfar)
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
        self.pitch -= direction.y * SENSITIVITY;

        self.needs_update = true;
    }
}
