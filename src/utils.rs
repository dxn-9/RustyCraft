fn fade(t: f32) -> f32 {
    ((6.0 * t - 15.) * t + 10.) * t * t * t
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + ((b - a) * t)
}
pub(crate) mod perlin_noise {
    use std::fmt::Debug;

    use super::*;
    use glam::Vec2;

    const WRAP: u32 = 256;
    lazy_static! {
        pub static ref PERM_TABLE: Vec<u32> = {
            let mut table: Vec<u32> = (0..WRAP).map(|i| i as u32).collect();
            shuffle(&mut table);
            for i in 0..WRAP {
                table.push(table[i as usize]);
            }
            table
        };
    }

    pub fn shuffle<T: Copy + Debug>(vec: &mut Vec<T>) -> &mut Vec<T> {
        use rand::prelude::*;

        let mut rng = rand::thread_rng();

        for i in (0..vec.len()).rev() {
            let a: usize = if i > 0 {
                f32::max(f32::floor(rng.gen::<f32>() * (i - 1) as f32), 0.0) as usize
            } else {
                0
            };
            let temp = vec[i].clone();

            vec[i] = vec[a];
            vec[a] = temp;
        }
        println!("SHUFFLED {vec:?}");
        vec
    }

    fn get_corner_consts(v: u32) -> Vec2 {
        // wrap the value in range 0..4
        let h = v & 3;

        match h {
            0 => glam::vec2(1.0, 1.0),
            1 => glam::vec2(-1.0, 1.0),
            2 => glam::vec2(-1.0, -1.0),
            _ => glam::vec2(1.0, -1.0),
        }
    }

    // https://rtouti.github.io/graphics/perlin-noise-algorithm
    pub fn perlin_noise(x: f32, y: f32) -> f32 {
        let qX = f32::floor(x) as u32 & (WRAP - 1);
        let qY = f32::floor(y) as u32 & (WRAP - 1);

        let dx = x - f32::floor(x);
        let dy = y - f32::floor(y);

        let top_left_vec = glam::vec2(dx, dy - 1.0);
        let top_right_vec = glam::vec2(dx - 1.0, dy - 1.0);
        let bottom_left_vec = glam::vec2(dx, dy);
        let bottom_right_vec = glam::vec2(dx - 1.0, dy);

        let top_left_const = PERM_TABLE[(PERM_TABLE[qX as usize] + qY + 1) as usize];
        let top_right_const = PERM_TABLE[(PERM_TABLE[(qX + 1) as usize] + qY + 1) as usize];
        let bottom_left_const = PERM_TABLE[(PERM_TABLE[(qX) as usize] + qY) as usize];
        let bottom_right_const = PERM_TABLE[(PERM_TABLE[(qX + 1) as usize] + qY) as usize];

        let top_left_dot = top_left_vec.dot(get_corner_consts(top_left_const));
        let top_right_dot = top_right_vec.dot(get_corner_consts(top_right_const));
        let bottom_left_dot = bottom_left_vec.dot(get_corner_consts(bottom_left_const));
        let bottom_right_dot = bottom_right_vec.dot(get_corner_consts(bottom_right_const));

        let u = fade(dx);
        let v = fade(dy);

        lerp(
            lerp(bottom_left_dot, top_left_dot, v),
            lerp(bottom_right_dot, top_right_dot, v),
            u,
        )
    }
    pub fn create_perlin_noise_data(width: u32, height: u32, frequency: f32) -> Vec<f32> {
        let mut data: Vec<f32> = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push(perlin_noise((x as f32) * frequency, (y as f32) * frequency));
            }
        }
        data
    }
}
