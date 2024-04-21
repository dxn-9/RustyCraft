use crate::blocks::block::{FaceDirections, TexturedBlock};
use crate::material::Texture;
use crate::player::Player;
use crate::state::State;
use wgpu::util::DeviceExt;
use wgpu::BufferUsages;

use super::pipeline_manager::PipelineManager;
use super::Pipeline;

pub struct UIPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub screenspace_buffer: wgpu::Buffer,
}

impl Pipeline for UIPipeline {
    fn render(
        &self,
        state: &State,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        _player: &std::sync::RwLockReadGuard<'_, Player>,
        _chunks: &Vec<std::sync::RwLockReadGuard<'_, crate::chunk::Chunk>>,
    ) {
        let main_pipeline_ref = state
            .pipeline_manager
            .main_pipeline
            .as_ref()
            .unwrap()
            .borrow();
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &main_pipeline_ref.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &main_pipeline_ref.bind_group_0, &[]);
        rpass.set_vertex_buffer(0, self.screenspace_buffer.slice(..));
        rpass.draw(0..6, 0..1);
    }
    fn init(state: &State, pipeline_manager: &PipelineManager) -> Self {
        let swapchain_capabilities = state.surface.get_capabilities(&state.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let shader_source = include_str!("../shaders/ui_shader.wgsl");

        let shader = state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let aspect_ratio = state.surface_config.height as f32 / state.surface_config.width as f32;

        let player = state.player.read().unwrap();
        let block_type = player.placing_block;
        let tex_coords = block_type.get_texcoords(FaceDirections::Front);
        let screen_quad = Self::create_screen_quad(aspect_ratio, tex_coords);

        let screenspace_buffer =
            state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&screen_quad),
                    label: Some("Screenspace rectangle"),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        // Pipeline layouts
        let pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&pipeline_manager
                        .main_pipeline
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .bind_group_0_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            state
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[Self::get_vertex_data_layout()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: swapchain_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),

                    primitive: wgpu::PrimitiveState {
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: Texture::DEPTH_FORMAT,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::Always,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });

        Self {
            screenspace_buffer,
            pipeline: render_pipeline,
        }
    }
    fn update(
        &mut self,
        _pipeline_manager: &PipelineManager,
        state: &State,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let aspect_ratio = state.surface_config.height as f32 / state.surface_config.width as f32;
        let player = state.player.read().unwrap();
        let block_type = player.placing_block;
        let tex_coords = block_type.get_texcoords(FaceDirections::Front);
        let screen_quad = Self::create_screen_quad(aspect_ratio, tex_coords);
        state.queue.write_buffer(
            &self.screenspace_buffer,
            0,
            bytemuck::cast_slice(&screen_quad),
        );
        Ok(())
    }
}
impl UIPipeline {
    // Creates the rectangle coords for displaying the block that would be placed if something is placed.
    fn create_screen_quad(aspect_ratio: f32, tex_coords: [[f32; 2]; 4]) -> Vec<f32> {
        vec![
            -0.9 * aspect_ratio,
            -0.9,
            tex_coords[0][0],
            tex_coords[0][1],
            -0.9 * aspect_ratio,
            -0.6,
            tex_coords[1][0],
            tex_coords[1][1],
            -0.6 * aspect_ratio,
            -0.6,
            tex_coords[2][0],
            tex_coords[2][1],
            -0.9 * aspect_ratio,
            -0.9,
            tex_coords[0][0],
            tex_coords[0][1],
            -0.6 * aspect_ratio,
            -0.6,
            tex_coords[2][0],
            tex_coords[2][1],
            -0.6 * aspect_ratio,
            -0.9,
            tex_coords[3][0],
            tex_coords[3][1],
        ]
    }
    fn get_vertex_data_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // Uv
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 2]>() as u64,
                    shader_location: 1,
                },
            ],
        }
    }
}
