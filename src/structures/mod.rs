pub mod tree;

use std::sync::{Arc, RwLock};

pub trait Structure {
    // position: Initial absolute position
    fn get_blocks(position: glam::Vec3) -> Vec<Arc<RwLock<Block>>>;
}
pub use tree::Tree;

use crate::blocks::block::Block; // Reexport into structures module
