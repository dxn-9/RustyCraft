use std::{error::Error, sync::RwLockReadGuard};

use self::pipeline_manager::PipelineManager;
use crate::{chunk::Chunk, player::Player, state::State};

pub trait Pipeline {
    fn init(state: &State, pipeline_manager: &PipelineManager) -> Self;
    fn update(
        &mut self,
        pipeline_manager: &PipelineManager,
        state: &State,
        // player: Arc<RwLock<Player>>,
        // queue: Arc<wgpu::Queue>,
        // device: Arc<wgpu::Device>,
        // surface_config: &wgpu::SurfaceConfiguration,
    ) -> Result<(), Box<dyn Error>>;
    fn render(
        &self,
        state: &State,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        player: &RwLockReadGuard<'_, Player>,
        chunks: &Vec<RwLockReadGuard<'_, Chunk>>,
    );
}
mod highlight_selected;
mod main;
pub mod pipeline_manager;
mod translucent;
mod ui;
