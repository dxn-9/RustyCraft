pub mod tree;

use std::sync::{Arc, Mutex};

pub trait Structure {
    // position: Initial absolute position
    fn get_blocks(position: glam::Vec3) -> Vec<Arc<Mutex<Block>>>;
}
pub use tree::Tree;

use crate::blocks::block::Block; // Reexport into structures module
