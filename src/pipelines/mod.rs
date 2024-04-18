use std::{
    error::Error,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use self::pipeline_manager::PipelineManager;
use crate::{chunk::Chunk, player::Player, state::State};

pub trait Pipeline {
    fn init(state: &State, pipeline_manager: &PipelineManager) -> Self;
    fn update(
        &mut self,
        pipeline_manager: &PipelineManager,
        player: Arc<RwLock<Player>>,
        queue: Arc<wgpu::Queue>,
        device: Arc<wgpu::Device>,
    ) -> Result<(), Box<dyn Error>>;
    fn render(
        &self,
        state: &State,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        player: &RwLockReadGuard<'_, Player>,
        chunks: &Vec<RwLockReadGuard<'_, Chunk>>,
    ) -> ();
}
mod highlight_selected;
mod main;
pub mod pipeline_manager;
mod translucent;
mod ui;
