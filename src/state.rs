use std::{cell::RefCell, f32::consts, rc::Rc};

use crate::{
    camera::{Camera, CameraController},
    material::Texture,
    model::{InstanceData, Model},
    pipeline::{self, Pipeline, Uniforms},
    world::World,
};
use glam::{vec2, Quat, Vec3};
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, KeyEvent},
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

        surface.configure(&device, &config);
        let model =
            Model::from_path("assets/cube.obj", "cube".to_string(), &device, &queue).unwrap();
        let model = Rc::new(RefCell::new(model));
        let world = World::init_world(model.clone(), &device);
        let mut state = Self {
            model,
            camera,
            pipelines: vec![],
            config,
            instance,
            device,
            world,
            queue,
            surface,
            adapter,
            camera_controller: CameraController::default(),
        };

        let pipeline = Pipeline::new(&state);

        state.pipelines.push(pipeline);

        state
    }
    pub fn handle_keypress(&mut self, event: KeyEvent, delta_time: f32) {
        let is_pressed: f32 = if event.state.is_pressed() { 1. } else { 0. };

        match event {
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                ..
            } => self.camera_controller.movement_vector.z = 1.0 * is_pressed,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyS),
                ..
            } => self.camera_controller.movement_vector.z = -1.0 * is_pressed,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyA),
                ..
            } => self.camera_controller.movement_vector.x = -1.0 * is_pressed,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyD),
                ..
            } => self.camera_controller.movement_vector.x = 1.0 * is_pressed,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyE),
                ..
            } => self.camera_controller.movement_vector.y = 1.0 * is_pressed,
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyQ),
                ..
            } => self.camera_controller.movement_vector.y = -1.0 * is_pressed,
            _ => {}
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
            self.pipelines[0].depth_texture = Texture::create_depth_texture(&self);
        }
    }
    pub fn update(&mut self, delta_time: f32, total_time: f32) {
        // let a = Basis3::from(self.pipeline.mesh.rotation);
        let rotation_increment = Quat::from_rotation_y(0.1);

        for (i, mesh) in self.model.as_ref().borrow().meshes.iter().enumerate() {
            // mesh.rotation * rotation_increment;
            self.queue.write_buffer(
                self.model.as_ref().borrow().meshes[0]
                    .world_mat_buffer
                    .as_ref()
                    .unwrap(),
                0,
                bytemuck::cast_slice(&[mesh._world_matrix]),
            );
        }
        // this is bad, it should in a uniform, but im just testing

        if self.camera_controller.movement_vector != Vec3::ZERO {
            self.camera
                .move_camera(&self.camera_controller.movement_vector, delta_time)
        }

        if self.camera.needs_update {
            let uniforms = Uniforms::from(&self.camera);
            self.queue.write_buffer(
                &self.pipelines[0].view_buffer,
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

        let mut instances_buffers: Vec<wgpu::BufferSlice<'_>> = vec![];
        let model_borrows: Vec<_> = self
            .pipelines
            .iter()
            .map(|pipeline| self.model.borrow())
            .collect();

        for model in model_borrows.iter() {
            instances_buffers.push(model.instances_buffer.slice(..));
        }
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
                    view: &self.pipelines[0].depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for (i, pipeline) in self.pipelines.iter().enumerate() {
                // let instances_buffer = pipeline.model.as_ref().borrow().instances_buffer.slice(..);
                rpass.set_pipeline(&pipeline.pipeline);
                // rpass.set_vertex_buffer(1, instances_buffers[i]);

                for mesh in model_borrows[i].meshes.iter() {
                    rpass.set_vertex_buffer(
                        0,
                        mesh.vertex_buffer
                            .as_ref()
                            .expect("vertex_buffer not set")
                            .slice(..),
                    );
                    rpass.set_index_buffer(
                        mesh.index_buffer
                            .as_ref()
                            .expect("index_buffer not set")
                            .slice(..),
                        wgpu::IndexFormat::Uint32,
                    );

                    rpass.set_bind_group(0, &pipeline.bind_group_0, &[]);
                    rpass.set_bind_group(1, &pipeline.bind_group_1, &[]);
                    for chunk in self.world.chunks.iter() {
                        rpass.set_bind_group(2, &chunk.chunk_bind_group, &[]);
                        rpass.draw_indexed(
                            0..mesh._indices.len() as u32,
                            0,
                            0..chunk.blocks.len() as u32,
                        );
                    }
                }
            }
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
    pub pipelines: Vec<Pipeline>,
    pub camera: Camera,
    pub world: World,
    pub camera_controller: CameraController,
    pub model: Rc<RefCell<Model>>,
}
