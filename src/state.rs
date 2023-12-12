use std::time::SystemTime;

use crate::{
    model::{create_cube_mesh, Mesh, ModelVertex, Vertex},
    texture::Texture,
};
use glam::Quat;
use wgpu::{util::DeviceExt, Device};
use winit::{dpi::PhysicalSize, window::Window};

pub struct Pipeline {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub world_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub transform_bind_group: wgpu::BindGroup,
    pub depth_texture: Texture,
    pub mesh: Mesh,
}
impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });
        let cube_mesh = create_cube_mesh();
        let vertex_size = std::mem::size_of::<ModelVertex>();

        // Vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&cube_mesh._vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
            ],
        }];
        // Index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(&cube_mesh._indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Bind group @0
        // TODO: Refactor this

        let world_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("transform_buffer"),
            contents: bytemuck::cast_slice(&[cube_mesh._world_matrix]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind_group_0"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            label: None,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: world_buffer.as_entire_binding(),
            }],
        });

        // Textures
        let depth_texture = Texture::create_depth_texture(device, config, "depth_texture");

        // Pipeline layouts
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(swapchain_format.into())],
            }),

            primitive: wgpu::PrimitiveState::default(),
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
            world_buffer,
            depth_texture,
            transform_bind_group: bind_group,
            index_buffer,
            vertex_buffer,
            pipeline: render_pipeline,
            mesh: cube_mesh,
        }
    }
}

pub struct State {
    pub surface: wgpu::Surface,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub pipeline: Pipeline,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(&window).unwrap() };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        let pipeline = Pipeline::new(&device, &surface, &adapter, &config);

        surface.configure(&device, &config);
        Self {
            pipeline,
            config,
            instance,
            device,
            queue,
            surface,
            adapter,
        }
    }
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width.max(1);
            self.config.height = new_size.height.max(1);
            self.surface.configure(&self.device, &self.config);
            self.pipeline.depth_texture =
                Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }
    pub fn update(&mut self) {
        // let a = Basis3::from(self.pipeline.mesh.rotation);
        // let euler = self.pipeline.mesh.rotation.to_euler(glam::EulerRot::XYZ);
        let rotation_increment = Quat::from_rotation_y(0.1);

        self.pipeline.mesh.rotation = self.pipeline.mesh.rotation * rotation_increment;
        self.pipeline.mesh.recalculate_world_matrix();

        // self.pipeline.mesh.rotation = angle;
        self.queue.write_buffer(
            &self.pipeline.world_buffer,
            0,
            bytemuck::cast_slice(&[self.pipeline.mesh._world_matrix]),
        );
    }
    pub fn draw(&self) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swapchain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder"),
            });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.pipeline.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_vertex_buffer(0, self.pipeline.vertex_buffer.slice(..));
            rpass.set_bind_group(0, &self.pipeline.transform_bind_group, &[]);
            rpass.set_index_buffer(
                self.pipeline.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rpass.set_pipeline(&self.pipeline.pipeline);
            rpass.draw_indexed(
                0..self.pipeline.mesh.elements_count,
                0,
                0..self.pipeline.mesh.instances,
            );
            // rpass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
