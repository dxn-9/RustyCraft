use std::any::Any;
use std::error::Error;
use std::f32::consts;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use glam::{vec2, vec3, Mat2, Vec2, Vec3};

use crate::blocks::block::{Block, FaceDirections};
use crate::collision::{CollisionPoint, RayResult};
use crate::persistence::{Loadable, Saveable};
use crate::{
    collision::CollisionBox,
    world::{World, CHUNK_SIZE},
};

const SENSITIVITY: f32 = 0.001;
const CAMERA_SPEED: f32 = 10.0;
const GRAVITY: f32 = 10.0;
pub static PLAYER_VIEW_OFFSET: Vec3 = vec3(0.4, 1.0, 0.4); /* this is kind of a hack, we should fix the camera's eye */

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
    pub facing_block: Option<Arc<RwLock<Block>>>,
    pub facing_face: Option<FaceDirections>,
}
impl Player {
    // Position relative to the chunk
    pub fn to_relative_position(&self) -> glam::Vec3 {
        todo!();
    }
    pub fn get_collision(&self) -> crate::collision::CollisionBox {
        crate::collision::CollisionBox::new(
            self.camera.eye.x - 0.4,
            self.camera.eye.y - 1.8,
            self.camera.eye.z - 0.4,
            0.8,
            2.0,
            0.8,
        )
    }
    // Gets the block that the player is facing
    pub fn get_facing_block<'a>(
        &mut self,
        collisions: &'a Vec<CollisionBox>,
    ) -> Option<(&'a CollisionBox, FaceDirections)> {
        let forward = self.camera.get_forward_dir();
        let mut ray_results: Vec<RayResult> = vec![];

        let ray = crate::collision::Ray {
            direction: forward,
            origin: self.camera.eye + PLAYER_VIEW_OFFSET,
        };

        for collision in collisions.iter() {
            if let Some(intersection_points) = ray.intersects_box(collision) {
                ray_results.push(RayResult {
                    points: intersection_points,
                    collision,
                })
            }
        }

        let mut block_collision: Option<&CollisionBox> = None;
        let mut max_distance = f32::MAX;
        let mut point: Option<Vec3> = None;

        for result in ray_results.iter() {
            let mut closest_point = result.points[0];
            if result.points[1].distance(self.camera.eye) < closest_point.distance(self.camera.eye)
            {
                closest_point = result.points[1];
            }

            if closest_point.distance(self.camera.eye) < max_distance {
                max_distance = closest_point.distance(self.camera.eye);
                block_collision = Some(result.collision);
                point = Some(closest_point.clone());
            }
        }
        let mut face_direction = None;

        return match (block_collision, point) {
            (Some(block_collision), Some(point)) => {
                // TODO: This can be precomputed
                let point_dir = ((block_collision.center() - point).normalize()) * -1.0;

                let face_directions = FaceDirections::all();
                let mut best_dot = -1.0;
                for face in face_directions.iter() {
                    let dot = point_dir.dot(face.get_normal_vector());
                    if dot > best_dot {
                        best_dot = dot;
                        face_direction = Some(face);
                    }
                }
                Some((block_collision, *face_direction.unwrap()))
            }
            _ => None,
        };
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

        let forward = self.camera.get_forward_dir();

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
    pub fn new(surface_width: f32, surface_height: f32) -> Camera {
        let (eye, yaw, pitch) = if let Ok((eye, yaw, pitch)) = Camera::load(Box::new(())) {
            (eye, yaw, pitch)
        } else {
            (glam::vec3(-4.0, 50.0, 4.0), consts::FRAC_PI_2, 0.0)
        };
        Self {
            aspect_ratio: surface_width / surface_height,
            eye,
            yaw,
            pitch,

            fovy: consts::FRAC_PI_4,
            znear: 0.1,
            zfar: 1000.,
            needs_update: false,
        }
    }
    pub fn build_view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_lh(self.eye, self.eye + self.get_forward_dir(), glam::Vec3::Y)
    }
    pub fn build_projection_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_lh(self.fovy, self.aspect_ratio, self.znear, self.zfar)
    }
    pub fn get_right_dir(&self) -> glam::Vec3 {
        glam::vec3(0.0, 1.0, 0.0).cross(self.get_forward_dir())
    }

    pub fn get_forward_dir(&self) -> glam::Vec3 {
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

impl Saveable<glam::Vec3> for Camera {
    fn save(&self) -> Result<(), Box<dyn Error>> {
        if let Ok(_) = std::fs::create_dir("data") {
            println!("Created dir");
        }
        let data = format!(
            "{},{},{},{},{}",
            self.eye.x, self.eye.y, self.eye.z, self.yaw, self.pitch
        );

        let player_file_name = "data/player";
        std::fs::write(player_file_name, data.as_bytes())?;

        Ok(())
    }
}

impl Loadable<(glam::Vec3, f32, f32)> for Camera {
    fn load(_: Box<dyn Any>) -> Result<((Vec3, f32, f32)), Box<dyn Error>> {
        let data = String::from_utf8(std::fs::read("data/player")?)?;
        let mut data = data.split(",");
        let x = data.next().unwrap().parse::<f32>().unwrap();
        let y = data.next().unwrap().parse::<f32>().unwrap();
        let z = data.next().unwrap().parse::<f32>().unwrap();
        let yaw = data.next().unwrap().parse::<f32>().unwrap();
        let pitch = data.next().unwrap().parse::<f32>().unwrap();

        return Ok((glam::vec3(x, y, z), yaw, pitch));
    }
}
