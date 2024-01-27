use std::{cell::RefCell, rc::Rc};

use bytemuck::{Pod, Zeroable};
use obj::Vertex;
use wgpu::{util::DeviceExt, Face};

use crate::{
    blocks::block::Block,
    camera::Camera,
    material::{Material, Texture},
    model::{InstanceData, Mesh, Model, PerVertex, VertexData},
    state::State,
    world::{self, Chunk, World},
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

        let shader = state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
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
            depth_texture,
            bind_group_0,
            bind_group_1,
            pipeline: render_pipeline,
        }
    }
}

pub struct Pipeline {
    pub projection_buffer: wgpu::Buffer,
    pub view_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_0: wgpu::BindGroup,
    pub bind_group_1: wgpu::BindGroup,
    pub depth_texture: Texture,
}
