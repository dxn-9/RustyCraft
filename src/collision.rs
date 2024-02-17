#[derive(Debug, Clone)]
pub struct CollisionBox {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

pub struct CollisionPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl CollisionPoint {
    pub fn new(x: f32, y: f32, z: f32) -> CollisionPoint {
        CollisionPoint { x, y, z }
    }
}

impl CollisionBox {
    pub fn from_block_position(x: f32, y: f32, z:f32) -> Self {
        CollisionBox {
            min_x: x,
            max_x: x + 1.0,
            min_y: y,
            max_y: y + 1.0,
            min_z: z,
            max_z: z + 1.0,
        }
    }
    pub fn to_block_position(&self) -> glam::Vec3 {
        glam::vec3(self.min_x, self.min_y, self.min_z)
    }
    pub fn new(x: f32, y: f32, z: f32, width: f32, height: f32, depth: f32) -> CollisionBox {
        CollisionBox {
            min_x: x,
            max_x: x + width,
            min_y: y,
            max_y: y + height,
            min_z: z,
            max_z: z + depth,
        }
    }
    pub fn intersects_point(&self, point: &CollisionPoint) -> bool {
        return point.x >= self.min_x
            && point.x <= self.max_x
            && point.y >= self.min_y
            && point.y <= self.max_y
            && point.z >= self.min_z
            && point.z <= self.max_z;
    }
    pub fn intersects_dir(&self, other: &CollisionBox) -> Option<(f32, f32, f32)> {
        if self.intersects(other) {
            let mut collision_dir = (0.0, 0.0, 0.0);

            // if self.max_x >= other.min_x && self.min_x < other.min_x {
            //     collision_dir.0 = self.max_x - other.min_x;
            // }
            // if self.min_x <= other.max_x && self.max_x > other.max_x {
            //     collision_dir.0 = self.min_x - other.max_x;
            // }
            // if self.max_y >= other.min_y && self.min_y < other.min_y {
            //     collision_dir.1 = 1.0;
            // }
            // if self.min_y <= other.max_y && self.max_y > other.max_y {
            //     collision_dir.1 = -1.0;
            // }
            // if self.max_z >= other.min_z && self.min_z > other.min_z {
            //     collision_dir.2 = self.max_z - other.min_z;
            // }
            // if self.min_z <= other.max_z && self.max_z > other.max_z {
            //     collision_dir.2 = self.min_z - other.max_z;
            // }

            return Some(collision_dir);
        } else {
            return None;
        }
    }
    pub fn intersects(&self, other: &CollisionBox) -> bool {
        return self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
            && self.min_z <= other.max_z
            && self.max_z >= other.min_z;
    }
    pub fn intersects_direction() {
        todo!()
    }
}

impl std::ops::Add<glam::Vec3> for CollisionBox {
    type Output = CollisionBox;

    fn add(self, rhs: glam::Vec3) -> Self::Output {
        CollisionBox::new(
            self.min_x + rhs.x,
            self.min_y + rhs.y,
            self.min_z + rhs.z,
            self.max_x - self.min_x,
            self.max_y - self.min_y,
            self.max_z - self.min_z,
        )
    }
}
