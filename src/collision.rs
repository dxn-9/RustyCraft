#[derive(Debug)]
pub struct CollisionBox {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    min_z: f32,
    max_z: f32,
}

impl CollisionBox {
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
    pub fn intersects_dir(&self, other: &CollisionBox) -> Option<(f32, f32, f32)> {
        if self.intersects(other) {
            let mut collision_dir = (0.0, 0.0, 0.0);

            if self.max_x >= other.min_x && self.min_x < other.min_x {
                collision_dir.0 = self.max_x - other.min_x;
            }
            if self.min_x <= other.max_x && self.max_x > other.max_x {
                collision_dir.0 = self.min_x - other.max_x;
            }
            if self.max_y >= other.min_y && self.min_y < other.min_y {
                collision_dir.1 = self.max_y - other.min_y;
            }
            if self.min_y <= other.max_y && self.max_y > other.max_y {
                collision_dir.1 = self.min_y - other.max_y;
            }
            if self.max_z >= other.min_z && self.min_z > other.min_z {
                collision_dir.2 = self.max_z - other.min_z;
            }
            if self.min_z <= other.max_z && self.max_z > other.max_z {
                collision_dir.2 = self.min_z - other.max_z;
            }

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
