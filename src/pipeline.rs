use std::fs::File;
use std::io::Read;
use std::{cell::RefCell, rc::Rc};

use bytemuck::{Pod, Zeroable};
use obj::Vertex;
use wgpu::{include_wgsl, util::DeviceExt, BindGroup, Buffer, Face, RenderPipeline};

use crate::{
    blocks::block::Block,
    player::Camera,
    material::{Material, Texture},
    state::State,
};

struct Matrices {
    view: glam::Mat4,
    projection: glam::Mat4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct Uniforms {
    pub view: [f32; 16],
    pub projection: [f32; 16],
}

impl From<&Camera> for Uniforms {
    fn from(camera: &Camera) -> Self {
        Self {
            view: *camera.build_view_matrix().as_ref(),
            projection: *camera.build_projection_matrix().as_ref(),
        }
    }
}

impl Pipeline {
    pub fn new(state: &State) -> Self {
        let swapchain_capabilities = state.surface.get_capabilities(&state.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let shader_source = std::fs::read_to_string("src/shaders/shader.wgsl").unwrap();

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
                        &bind_group_1_layout,
                        &state.world.chunk_data_layout,
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
                        polygon_mode: state.config.polygon_mode,
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
            view_buffer,
            projection_buffer,
            pipeline_type: PipelineType::WORLD,
            depth_texture,
            bind_group_0,
            bind_group_1,
            pipeline: render_pipeline,
        }
    }
}
impl PipelineTrait for Pipeline {
    fn projection_buffer(&self) -> &Buffer {
        &self.projection_buffer
    }

    fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    fn view_buffer(&self) -> &Buffer {
        &self.view_buffer
    }

    fn depth_texture(&self) -> &Texture {
        &self.depth_texture
    }
    fn set_depth_texture(&mut self, texture: Texture) {
        self.depth_texture = texture;
    }
    fn bind_group_0(&self) -> &BindGroup {
        &self.bind_group_0
    }

    fn bind_group_1(&self) -> &BindGroup {
        &self.bind_group_1
    }
    fn get_type(&self) -> PipelineType {
        self.pipeline_type
    }
}

pub trait PipelineTrait {
    fn projection_buffer(&self) -> &wgpu::Buffer;
    fn pipeline(&self) -> &wgpu::RenderPipeline;
    fn view_buffer(&self) -> &wgpu::Buffer;
    fn bind_group_0(&self) -> &wgpu::BindGroup;
    fn bind_group_1(&self) -> &wgpu::BindGroup;
    fn depth_texture(&self) -> &Texture;
    fn set_depth_texture(&mut self, texture: Texture);

    fn get_type(&self) -> PipelineType;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PipelineType {
    WORLD,
    UI,
}
pub struct Pipeline {
    pub projection_buffer: wgpu::Buffer,
    pub view_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_0: wgpu::BindGroup,
    pub bind_group_1: wgpu::BindGroup,
    pub depth_texture: Texture,
    pub pipeline_type: PipelineType,
}
