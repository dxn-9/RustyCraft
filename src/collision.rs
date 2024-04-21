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

pub struct Ray {
    pub origin: glam::Vec3,
    pub direction: glam::Vec3,
}

impl Ray {
    // https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-box-intersection.html
    pub fn intersects_box(&self, collision_box: &CollisionBox) -> Option<Vec<glam::Vec3>> {
        let mut tmin;
        let mut tmax;
        let tymin;
        let tymax;
        let tzmin;
        let tzmax;

        let invdirx = 1.0 / self.direction.x;
        let invdiry = 1.0 / self.direction.y;
        let invdirz = 1.0 / self.direction.z;

        if invdirx >= 0.0 {
            tmin = (collision_box.min_x - self.origin.x) * invdirx;
            tmax = (collision_box.max_x - self.origin.x) * invdirx;
        } else {
            tmin = (collision_box.max_x - self.origin.x) * invdirx;
            tmax = (collision_box.min_x - self.origin.x) * invdirx;
        }

        if invdiry >= 0.0 {
            tymin = (collision_box.min_y - self.origin.y) * invdiry;
            tymax = (collision_box.max_y - self.origin.y) * invdiry;
        } else {
            tymin = (collision_box.max_y - self.origin.y) * invdiry;
            tymax = (collision_box.min_y - self.origin.y) * invdiry;
        }

        if tmin > tymax || tymin > tmax {
            return None;
        }
        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }

        if invdirz >= 0.0 {
            tzmin = (collision_box.min_z - self.origin.z) * invdirz;
            tzmax = (collision_box.max_z - self.origin.z) * invdirz;
        } else {
            tzmin = (collision_box.max_z - self.origin.z) * invdirz;
            tzmax = (collision_box.min_z - self.origin.z) * invdirz;
        }

        if tmin > tzmax || tzmin > tmax || tmin < 0.0 || tmax < 0.0 {
            return None;
        }

        if tzmin > tmin {
            tmin = tzmin;
        }
        if tzmax < tmax {
            tmax = tzmax;
        }

        Some(vec![
            self.origin + self.direction * tmin,
            self.origin + self.direction * tmax,
        ])
    }
}

#[derive(Debug)]
pub struct RayResult {
    pub points: Vec<glam::Vec3>,
    pub collision: CollisionBox,
}

impl CollisionPoint {
    pub fn new(x: f32, y: f32, z: f32) -> CollisionPoint {
        CollisionPoint { x, y, z }
    }
}

impl CollisionBox {
    pub fn center(&self) -> glam::Vec3 {
        glam::vec3(
            self.min_x + (self.max_x - self.min_x) / 2.0,
            self.min_y + (self.max_y - self.min_y) / 2.0,
            self.min_z + (self.max_z - self.min_z) / 2.0,
        )
    }
    pub fn from_block_position(x: f32, y: f32, z: f32) -> Self {
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
        point.x >= self.min_x
            && point.x <= self.max_x
            && point.y >= self.min_y
            && point.y <= self.max_y
            && point.z >= self.min_z
            && point.z <= self.max_z
    }
    pub fn intersects(&self, other: &CollisionBox) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
            && self.min_z <= other.max_z
            && self.max_z >= other.min_z
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
