use std::{
    cell::RefCell,
    sync::{Arc, RwLock},
};

use wgpu::{CommandEncoder, TextureView};

use crate::{player::Player, state::State};

use super::{
    highlight_selected::HighlightSelectedPipeline, main::MainPipeline,
    translucent::TranslucentPipeline, ui::UIPipeline, Pipeline,
};

pub struct PipelineManager {
    pub main_pipeline: Option<RefCell<MainPipeline>>,
    pub translucent_pipeline: Option<RefCell<TranslucentPipeline>>,
    pub highlight_selected_pipeline: Option<RefCell<HighlightSelectedPipeline>>,
    pub ui_pipeline: Option<RefCell<UIPipeline>>,
}

impl PipelineManager {
    pub fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        main_pipeline: &MainPipeline,
    ) {
    }
    pub fn init(state: &State) -> PipelineManager {
        let mut pipeline = PipelineManager {
            highlight_selected_pipeline: None,
            main_pipeline: None,
            translucent_pipeline: None,
            ui_pipeline: None,
        };
        pipeline.main_pipeline = Some(RefCell::new(MainPipeline::init(state, &pipeline)));
        pipeline.translucent_pipeline =
            Some(RefCell::new(TranslucentPipeline::init(state, &pipeline)));
        pipeline.highlight_selected_pipeline = Some(RefCell::new(HighlightSelectedPipeline::init(
            state, &pipeline,
        )));
        pipeline.ui_pipeline = Some(RefCell::new(UIPipeline::init(state, &pipeline)));
        return pipeline;
    }

    pub fn update(&self, state: &State) -> Result<(), Box<dyn std::error::Error>> {
        self.main_pipeline
            .as_ref()
            .unwrap()
            .borrow_mut()
            .update(&self, state)?;
        self.translucent_pipeline
            .as_ref()
            .unwrap()
            .borrow_mut()
            .update(&self, state)?;
        self.highlight_selected_pipeline
            .as_ref()
            .unwrap()
            .borrow_mut()
            .update(&self, state)?;
        self.ui_pipeline
            .as_ref()
            .unwrap()
            .borrow_mut()
            .update(&self, state)?;

        return Ok(());
    }
}
