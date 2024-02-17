use std::time::{Duration, Instant};

use glam::{vec2, vec3, Mat2, Vec2, Vec3};

use crate::{
    collision::CollisionBox,
    world::{World, CHUNK_SIZE},
};

const SENSITIVITY: f32 = 0.001;
const CAMERA_SPEED: f32 = 10.0;
const GRAVITY: f32 = 10.0;
lazy_static! {
    static ref JUMP_DURATION: Duration = Duration::from_secs_f32(0.1);
}
const JUMP_HEIGHT: f32 = 1.5;

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
    pub on_ground: bool,
    pub is_jumping: bool,
    pub jump_action_start: Option<Instant>,
    pub is_ghost: bool,
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
            self.camera.eye.y - 1.0,
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

    /* TODO: This probably can be optimized */
    pub fn move_camera(
        &mut self,
        direction: &Vec3,
        delta_time: f32,
        collisions: &Vec<CollisionBox>,
    ) {
        let input_direction = direction;
        let player_collision = self.get_collision();

        let forward = self.camera.calc_target();

        let mut velocity = vec3(0.0, 0.0, 0.0);

        // z axis
        if input_direction.z > 0.0 {
            velocity += forward * CAMERA_SPEED * delta_time;
        } else if input_direction.z < 0.0 {
            velocity -= forward * CAMERA_SPEED * delta_time;
        }

        let right = Vec3::cross(forward, Vec3::Y);

        if input_direction.x > 0.0 {
            velocity -= right * CAMERA_SPEED * delta_time;
        } else if input_direction.x < 0.0 {
            velocity += right * CAMERA_SPEED * delta_time;
        }

        /* Ignore collisions if ghost */
        if self.is_ghost {
            velocity *= 4.0;
            self.camera.eye += velocity;
            return;
        }

        let can_move_z = player_collision.clone() + glam::vec3(0.0, 0.0, velocity.z);
        for collision in collisions.iter() {
            if can_move_z.intersects(collision) {
                velocity.z = 0.0;
            }
        }
        let can_move_x = player_collision.clone() + glam::vec3(velocity.x, 0.0, 0.0);
        for collision in collisions.iter() {
            if can_move_x.intersects(collision) {
                velocity.x = 0.0;
            }
        }

        velocity.y -= GRAVITY * delta_time;
        self.on_ground = false;

        if self.is_jumping {
            let now = Instant::now();
            let delta_jump = now
                - self
                    .jump_action_start
                    .expect("If it's jumping this should be set");
            if delta_jump <= *JUMP_DURATION {
                velocity.y = JUMP_HEIGHT * delta_time * 10.0; /* Multiply by 10 bcs animation time is 0.1  */
            } else {
                self.is_jumping = false;
                self.jump_action_start = None;
            }
        }

        let can_move_y = player_collision.clone() + glam::vec3(0.0, velocity.y, 0.0);
        for collision in collisions.iter() {
            if can_move_y.intersects(collision) {
                velocity.y = 0.0;
                self.on_ground = true; // This can make it infinite to jump if there is a block above
            }
        }

        // fly up
        if input_direction.y > 0.0 {
            velocity.y = 2.0;
        }

        self.camera.eye += velocity;

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
