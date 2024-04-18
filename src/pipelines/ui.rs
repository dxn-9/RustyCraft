use crate::blocks::block::FaceDirections;
use crate::material::Texture;
use crate::pipeline::Uniforms;
use crate::player::Player;
use crate::state::State;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, RenderPipeline};

use super::pipeline_manager::PipelineManager;
use super::Pipeline;

pub struct UIPipeline {
    pub projection_buffer: wgpu::Buffer,
    pub view_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub indices: usize,
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
        todo!();
    }
    fn init(state: &State, pipeline_manager: &PipelineManager) -> Self {
        todo!();
        // {
        //     let swapchain_capabilities = state.surface.get_capabilities(&state.adapter);
        //     let swapchain_format = swapchain_capabilities.formats[0];

        //     let shader_source = include_str!("../shaders/ui_shader.wgsl");

        //     let shader = state
        //         .device
        //         .create_shader_module(wgpu::ShaderModuleDescriptor {
        //             label: None,
        //             source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        //         });

        //     let uniforms = Uniforms::from(&state.player.read().unwrap().camera);

        //     let projection_buffer =
        //         state
        //             .device
        //             .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //                 label: Some("projection_matrix"),
        //                 contents: bytemuck::cast_slice(&[uniforms.projection]),
        //                 usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        //             });

        //     // View matrix
        //     let view_buffer = state
        //         .device
        //         .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //             label: Some("projection_matrix"),
        //             contents: bytemuck::cast_slice(&[uniforms.view]),
        //             usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        //         });

        //     // Bind groups
        //     let bind_group_0_layout =
        //         state
        //             .device
        //             .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //                 label: Some("bind_group_0"),
        //                 entries: &[
        //                     wgpu::BindGroupLayoutEntry {
        //                         binding: 0,
        //                         visibility: wgpu::ShaderStages::VERTEX,
        //                         ty: wgpu::BindingType::Buffer {
        //                             ty: wgpu::BufferBindingType::Uniform,
        //                             has_dynamic_offset: false,
        //                             min_binding_size: None,
        //                         },
        //                         count: None,
        //                     },
        //                     wgpu::BindGroupLayoutEntry {
        //                         binding: 1,
        //                         visibility: wgpu::ShaderStages::VERTEX,
        //                         ty: wgpu::BindingType::Buffer {
        //                             ty: wgpu::BufferBindingType::Uniform,
        //                             has_dynamic_offset: false,
        //                             min_binding_size: None,
        //                         },
        //                         count: None,
        //                     },
        //                 ],
        //             });
        //     let bind_group_0 = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        //         layout: &bind_group_0_layout,
        //         label: None,
        //         entries: &[
        //             wgpu::BindGroupEntry {
        //                 binding: 0,
        //                 resource: projection_buffer.as_entire_binding(),
        //             },
        //             wgpu::BindGroupEntry {
        //                 binding: 1,
        //                 resource: view_buffer.as_entire_binding(),
        //             },
        //         ],
        //     });

        //     let texture_atlas = Texture::from_path(
        //         "assets/tex_atlas.png",
        //         "tex_atlas".to_string(),
        //         &state.device,
        //         &state.queue,
        //     )
        //     .unwrap();

        //     let bind_group_1_layout =
        //         state
        //             .device
        //             .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //                 label: Some("bind_group_1"),
        //                 entries: &[
        //                     wgpu::BindGroupLayoutEntry {
        //                         binding: 0,
        //                         visibility: wgpu::ShaderStages::FRAGMENT,
        //                         ty: wgpu::BindingType::Texture {
        //                             sample_type: wgpu::TextureSampleType::Float {
        //                                 filterable: true,
        //                             },
        //                             view_dimension: wgpu::TextureViewDimension::D2,
        //                             multisampled: false,
        //                         },
        //                         count: None,
        //                     },
        //                     wgpu::BindGroupLayoutEntry {
        //                         binding: 1,
        //                         visibility: wgpu::ShaderStages::FRAGMENT,
        //                         ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //                         count: None,
        //                     },
        //                 ],
        //             });

        //     let bind_group_1 = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        //         label: Some("bind_group_1"),
        //         layout: &bind_group_1_layout,
        //         entries: &[
        //             wgpu::BindGroupEntry {
        //                 binding: 0,
        //                 resource: wgpu::BindingResource::TextureView(&texture_atlas.view),
        //             },
        //             wgpu::BindGroupEntry {
        //                 binding: 1,
        //                 resource: wgpu::BindingResource::Sampler(&texture_atlas.sampler),
        //             },
        //         ],
        //     });

        //     let main_pipeline = state.main_pipeline.as_ref().expect("Should be initialized");
        //     // Pipeline layouts
        //     let pipeline_layout =
        //         state
        //             .device
        //             .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //                 label: None,
        //                 bind_group_layouts: &[
        //                     &main_pipeline.bind_group_0_layout,
        //                     &main_pipeline.bind_group_1_layout,
        //                 ],
        //                 push_constant_ranges: &[],
        //             });

        //     let render_pipeline =
        //         state
        //             .device
        //             .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        //                 label: None,
        //                 layout: Some(&pipeline_layout),
        //                 vertex: wgpu::VertexState {
        //                     module: &shader,
        //                     entry_point: "vs_main",
        //                     buffers: &[UI::get_vertex_data_layout()],
        //                 },
        //                 fragment: Some(wgpu::FragmentState {
        //                     module: &shader,
        //                     entry_point: "fs_main",
        //                     targets: &[Some(wgpu::ColorTargetState {
        //                         format: swapchain_format,
        //                         blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        //                         write_mask: wgpu::ColorWrites::ALL,
        //                     })],
        //                 }),

        //                 primitive: wgpu::PrimitiveState {
        //                     polygon_mode: state.config.polygon_mode,
        //                     cull_mode: None,

        //                     ..Default::default()
        //                 },
        //                 depth_stencil: Some(wgpu::DepthStencilState {
        //                     format: Texture::DEPTH_FORMAT,
        //                     depth_write_enabled: true,
        //                     depth_compare: wgpu::CompareFunction::Less,
        //                     stencil: wgpu::StencilState::default(),
        //                     bias: wgpu::DepthBiasState::default(),
        //                 }),
        //                 multisample: wgpu::MultisampleState::default(),

        //                 multiview: None,
        //             });

        //     Self {
        //         view_buffer,
        //         projection_buffer,
        //         pipeline_type: PipelineType::UI,
        //         bind_group_0,
        //         bind_group_1,
        //         pipeline: render_pipeline,
        //     }
        // }
        // let vertices: [[f32; 3]; 4] = [
        //     [0.0, 0.0, 0.0],
        //     [0.0, 1.0, 0.0],
        //     [1.0, 0.0, 0.0],
        //     [1.0, 1.0, 0.0],
        // ];
        // let indices: [u32; 6] = [0, 1, 2, 2, 1, 3];

        // let vertex_buffer = state
        //     .device
        //     .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //         label: Some("UI Vertex Buffer"),
        //         usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        //         contents: bytemuck::cast_slice(&vertices),
        //     });
        // let index_buffer = state
        //     .device
        //     .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //         label: Some("UI Vertex Buffer"),
        //         usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        //         contents: bytemuck::cast_slice(&indices),
        //     });

        // Self {
        //     indices: 0,
        //     vertex_buffer,
        //     index_buffer,
        // }
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
