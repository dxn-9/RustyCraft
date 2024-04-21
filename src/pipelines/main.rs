use wgpu::Face;

use crate::{
    blocks::block::Block, material::Texture, pipeline::Uniforms, player::Player, state::State,
};

use super::{pipeline_manager::PipelineManager, Pipeline};
use wgpu::util::DeviceExt;

pub struct MainPipeline {
    pub projection_buffer: wgpu::Buffer,
    pub view_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_0: wgpu::BindGroup,
    pub bind_group_0_layout: wgpu::BindGroupLayout,
    pub depth_texture: Texture,
}

impl Pipeline for MainPipeline {
    fn render(
        &self,
        _state: &State,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        player: &std::sync::RwLockReadGuard<'_, Player>,
        chunks: &Vec<std::sync::RwLockReadGuard<'_, crate::chunk::Chunk>>,
    ) {
        let mut main_rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.03,
                        g: 0.64,
                        b: 0.97,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        main_rpass.set_pipeline(&self.pipeline);
        main_rpass.set_bind_group(0, &self.bind_group_0, &[]);

        main_rpass.set_bind_group(2, &player.camera.position_bind_group, &[]);

        for chunk in chunks.iter() {
            if chunk.visible {
                main_rpass.set_bind_group(1, &chunk.chunk_bind_group, &[]);
                main_rpass.set_vertex_buffer(
                    0,
                    chunk
                        .chunk_vertex_buffer
                        .as_ref()
                        .expect("Vertex buffer not initiated")
                        .slice(..),
                );
                main_rpass.set_index_buffer(
                    chunk
                        .chunk_index_buffer
                        .as_ref()
                        .expect("Index buffer not initiated")
                        .slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                main_rpass.draw_indexed(0..chunk.indices, 0, 0..1);
            }
        }
    }

    fn update(
        &mut self,
        _pipeline_manager: &PipelineManager,
        _state: &State,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn init(state: &State, _pipeline_manager: &PipelineManager) -> Self {
        let swapchain_capabilities = state.surface.get_capabilities(&state.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let shader_source = include_str!("../shaders/shader.wgsl");

        let shader = state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });
        let camera = &state.player.read().unwrap().camera;
        let uniforms = Uniforms::from(camera);

        let projection_buffer =
            state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("projection_matrix"),
                    contents: bytemuck::cast_slice(&[uniforms.projection]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // View matrix
        let view_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("projection_matrix"),
                contents: bytemuck::cast_slice(&[uniforms.view]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Constant bindgroup for chunks per row
        let world_chunk_per_row_buffer =
            state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&[crate::world::CHUNKS_PER_ROW]),
                    label: Some("world_chunk_per_row"),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let image_bytes = include_bytes!("../../assets/tex_atlas.png");
        let texture_atlas = Texture::from_bytes(
            image_bytes,
            "tex_atlas".to_string(),
            &state.device,
            &state.queue,
        )
        .unwrap();
        // Bind 0: general purpouse group for 3d rendering
        let bind_group_0_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bind_group_0"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let bind_group_0 = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_0_layout,
            label: None,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: projection_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: view_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: world_chunk_per_row_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&texture_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&texture_atlas.sampler),
                },
            ],
        });

        // Textures
        let depth_texture = Texture::create_depth_texture(state);

        // Pipeline layouts
        let pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &bind_group_0_layout,
                        &state.world.chunk_data_layout,
                        &state
                            .player
                            .read()
                            .unwrap()
                            .camera
                            .position_bind_group_layout,
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
                        buffers: &[Block::get_vertex_data_layout()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(swapchain_format.into())],
                    }),

                    primitive: wgpu::PrimitiveState {
                        cull_mode: Some(Face::Front),
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: Texture::DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });

        Self {
            bind_group_0_layout,
            view_buffer,
            projection_buffer,
            depth_texture,
            bind_group_0,
            pipeline: render_pipeline,
        }
    }
}

impl MainPipeline {
    pub fn set_depth_texture(&mut self, texture: Texture) {
        self.depth_texture = texture;
    }
}
