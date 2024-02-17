use crate::blocks::block::{Block, BlockVertexData, FaceDirections};
use crate::material::Texture;
use crate::pipeline::{Pipeline, PipelineTrait, PipelineType, Uniforms};
use crate::player::Player;
use crate::state::State;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, RenderPipeline};

pub struct UI {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

impl UI {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let vertices: [[f32; 3]; 4] = [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let indices: [u32; 6] = [0, 1, 2, 2, 1, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&vertices),
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Vertex Buffer"),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&indices),
        });

        Self {
            vertex_buffer,
            index_buffer,
        }
    }
    pub fn update(&mut self, player: &Player, queue: Arc<wgpu::Queue>, device: Arc<wgpu::Device>) {
        if let Some(facing_block) = player.facing_block.as_ref() {
            let block = facing_block.lock().unwrap();

            let face_data = block
                .faces
                .as_ref()
                .unwrap()
                .iter()
                .find(|f| f.face_direction == FaceDirections::Top)
                .unwrap()
                .create_face_data_abs(&block);

            let blocks_position = face_data.0.iter().map(|v| v.position).collect::<Vec<_>>();

            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&blocks_position),
            );
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&face_data.1));
        }
    }

    pub fn get_vertex_data_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
            ],
        }
    }
}
pub struct UIPipeline {
    pub projection_buffer: wgpu::Buffer,
    pub view_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_0: wgpu::BindGroup,
    pub bind_group_1: wgpu::BindGroup,
    pub pipeline_type: PipelineType,
}
impl UIPipeline {
    pub fn new(state: &State) -> Self {
        let swapchain_capabilities = state.surface.get_capabilities(&state.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let shader_source = std::fs::read_to_string("src/shaders/ui_shader.wgsl").unwrap();

        let shader = state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let uniforms = Uniforms::from(&state.player.camera);

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

        // Bind groups
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
            ],
        });

        let texture_atlas = Texture::from_path(
            "assets/tex_atlas.png",
            "tex_atlas".to_string(),
            &state.device,
            &state.queue,
        )
        .unwrap();

        let bind_group_1_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bind_group_1"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let bind_group_1 = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind_group_1"),
            layout: &bind_group_1_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_atlas.sampler),
                },
            ],
        });

        // Pipeline layouts
        let pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_0_layout, &bind_group_1_layout],
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
                        buffers: &[UI::get_vertex_data_layout()],
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
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),

                    multiview: None,
                });

        Self {
            view_buffer,
            projection_buffer,
            pipeline_type: PipelineType::UI,
            bind_group_0,
            bind_group_1,
            pipeline: render_pipeline,
        }
    }
}
impl PipelineTrait for UIPipeline {
    fn projection_buffer(&self) -> &Buffer {
        &self.projection_buffer
    }

    fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    fn view_buffer(&self) -> &Buffer {
        &self.view_buffer
    }

    fn bind_group_0(&self) -> &BindGroup {
        &self.bind_group_0
    }

    fn bind_group_1(&self) -> &BindGroup {
        &self.bind_group_1
    }

    fn depth_texture(&self) -> &Texture {
        todo!()
    }

    fn set_depth_texture(&mut self, texture: Texture) {
        todo!()
    }

    fn get_type(&self) -> PipelineType {
        self.pipeline_type
    }
}
