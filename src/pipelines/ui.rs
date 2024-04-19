use crate::blocks::block::FaceDirections;
use crate::material::Texture;
use crate::pipeline::Uniforms;
use crate::player::Player;
use crate::state::State;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, BufferUsages, RenderPipeline};

use super::pipeline_manager::PipelineManager;
use super::Pipeline;

pub struct UIPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub ui_bindgroup: wgpu::BindGroup,
    pub selected_blockid_buffer: wgpu::Buffer,
    pub resolution_buffer: wgpu::Buffer,
    pub screenspace_buffer: wgpu::Buffer,
}

impl Pipeline for UIPipeline {
    fn render(
        &self,
        state: &State,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        player: &std::sync::RwLockReadGuard<'_, Player>,
        chunks: &Vec<std::sync::RwLockReadGuard<'_, crate::chunk::Chunk>>,
    ) -> () {
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
        rpass.set_bind_group(1, &self.ui_bindgroup, &[]);
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

        let selected_blockid_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: std::mem::size_of::<u32>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let screen_quad: Vec<f32> = vec![
            -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0, -1.0,
        ];
        let screenspace_buffer =
            state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&screen_quad),
                    label: Some("Screenspace rectangle"),
                    usage: BufferUsages::VERTEX,
                });

        let resolution = [
            state.surface_config.width as f32,
            state.surface_config.height as f32,
        ];
        let resolution_buffer =
            state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&[resolution]),
                    label: None,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

        let ui_bindgroup_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let ui_bindgroup = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ui_bindgroup_layout,
            label: None,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: resolution_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: selected_blockid_buffer.as_entire_binding(),
                },
            ],
        });
        // Pipeline layouts
        let pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &pipeline_manager
                            .main_pipeline
                            .as_ref()
                            .unwrap()
                            .borrow()
                            .bind_group_0_layout,
                        &ui_bindgroup_layout,
                    ],
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
                        polygon_mode: state.config.polygon_mode,
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
            resolution_buffer,
            selected_blockid_buffer,
            ui_bindgroup,
        }
    }
    fn update(
        &mut self,
        pipeline_manager: &PipelineManager,
        player: Arc<RwLock<Player>>,
        queue: Arc<wgpu::Queue>,
        device: Arc<wgpu::Device>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        todo!();
        //     let player = state.player.read().unwrap();
        //     if let Some(block_ptr) = player.facing_block.as_ref() {
        //         let block = block_ptr.read().unwrap();

        //         let face_data = FaceDirections::all()
        //             .iter()
        //             .find(|f| **f == player.facing_face.unwrap())
        //             .unwrap()
        //             .create_face_data(block_ptr.clone(), &vec![]);

        //         let blocks_position = face_data
        //             .0
        //             .iter()
        //             .map(|v| {
        //                 [
        //                     // TODO: This is kinda ugly
        //                     v.position[0] + (block.absolute_position.x - block.position.x),
        //                     v.position[1] + (block.absolute_position.y - block.position.y),
        //                     v.position[2] + (block.absolute_position.z - block.position.z),
        //                 ]
        //             })
        //             .collect::<Vec<_>>();

        //         self.indices = face_data.1.len();
        //         state.queue.write_buffer(
        //             &self.vertex_buffer,
        //             0,
        //             bytemuck::cast_slice(&blocks_position),
        //         );
        //         state
        //             .queue
        //             .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&face_data.1));
        //     } else {
        //         self.indices = 0;
        //         state.queue.write_buffer(&self.vertex_buffer, 0, &[]);
        //         state.queue.write_buffer(&self.index_buffer, 0, &[]);
        //     }
        // }
    }
}
impl UIPipeline {
    fn get_vertex_data_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        }
    }
}
