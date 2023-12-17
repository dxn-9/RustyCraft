use std::f32::consts;

use crate::{
    camera::Camera,
    pipeline::{Pipeline, Uniforms},
    texture::Texture,
};
use glam::{vec2, Quat};
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, KeyEvent},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey, SmolStr},
    window::Window,
};

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

        let camera = Camera {
            aspect_ratio: config.width as f32 / config.height as f32,
            eye: glam::vec3(0.0, 0.0, -5.0),
            yaw: consts::FRAC_PI_2,
            pitch: 0.0,

            fovy: consts::FRAC_PI_4,
            znear: 0.1,
            zfar: 1000.,
            needs_update: false,
        };

        let pipeline = Pipeline::new(&device, &surface, &adapter, &config, &camera);

        surface.configure(&device, &config);
        Self {
            camera,
            pipeline,
            config,
            instance,
            device,
            queue,
            surface,
            adapter,
        }
    }
    pub fn handle_keypress(&mut self, event: KeyEvent, delta_time: f32) {
        let mut movement_vector = glam::Vec3::ZERO;

        match event {
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                ..
            } => movement_vector.z = 1.0,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyS),
                ..
            } => movement_vector.z = -1.0,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyA),
                ..
            } => movement_vector.x = -1.0,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyD),
                ..
            } => movement_vector.x = 1.0,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyE),
                ..
            } => movement_vector.y = 1.0,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyQ),
                ..
            } => movement_vector.y = -1.0,
            _ => {}
        }

        if movement_vector != glam::Vec3::ZERO {
            self.camera.move_camera(&movement_vector, delta_time);
        }
    }
    pub fn handle_mouse(&mut self, delta: &glam::Vec2) {
        self.camera.move_target(delta)
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
        let rotation_increment = Quat::from_rotation_y(0.1);

        self.pipeline.mesh.rotation = self.pipeline.mesh.rotation * rotation_increment;
        // self.pipeline.mesh.recalculate_world_matrix();

        // self.pipeline.mesh.rotation = angle;
        self.queue.write_buffer(
            &self.pipeline.world_buffer,
            0,
            bytemuck::cast_slice(&[self.pipeline.mesh._world_matrix]),
        );

        if self.camera.needs_update {
            let uniforms = Uniforms::from(&self.camera);
            self.queue.write_buffer(
                &self.pipeline.view_buffer,
                0,
                bytemuck::cast_slice(&[uniforms.view]),
            );

            self.camera.needs_update = false;
        }
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
            rpass.set_bind_group(0, &self.pipeline.bind_group_0, &[]);
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

pub struct State {
    pub surface: wgpu::Surface,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub pipeline: Pipeline,
    pub camera: Camera,
}
