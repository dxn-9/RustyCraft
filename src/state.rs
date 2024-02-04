use std::{cell::RefCell, f32::consts, rc::Rc, sync::Arc};

use crate::{
    camera::{Camera, CameraController, Player},
    // cube::{build_mesh, CUBE_INDICES, CUBE_VERTEX, CUBE_VERTEX_NON_INDEXED},
    material::Texture,
    model::{InstanceData, Model},
    pipeline::{self, Pipeline, Uniforms},
    world::World,
};
use glam::{vec2, Quat, Vec3};
use wgpu::{util::DeviceExt, BufferUsages};
use winit::{
    dpi::{LogicalSize, PhysicalSize},
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
                    features: wgpu::Features::POLYGON_MODE_LINE,
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        let camera = Camera {
            aspect_ratio: surface_config.width as f32 / surface_config.height as f32,
            eye: glam::vec3(-4.0, 0.0, 4.0),
            yaw: consts::FRAC_PI_2,
            pitch: 0.0,

            fovy: consts::FRAC_PI_4,
            znear: 0.1,
            zfar: 1000.,
            needs_update: false,
        };
        let player = Player {
            camera,
            current_chunk: (0, 0),
        };

        surface.configure(&device, &surface_config);
        let config = Config {
            polygon_mode: wgpu::PolygonMode::Fill,
        };

        let world = World::init_world(device.clone(), queue.clone());

        let mut state = Self {
            config,
            player,
            pipelines: vec![],
            surface_config,
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
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyF),
                state: winit::event::ElementState::Pressed,
                ..
            } => {
                if self.config.polygon_mode == wgpu::PolygonMode::Line {
                    self.config.polygon_mode = wgpu::PolygonMode::Fill
                } else {
                    self.config.polygon_mode = wgpu::PolygonMode::Line
                }

                self.pipelines.pop();
                self.pipelines.push(Pipeline::new(&self))
            }
            _ => {}
        }
    }
    pub fn handle_mouse(&mut self, delta: &glam::Vec2) {
        self.player.camera.move_target(delta)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width.max(1);
            self.surface_config.height = new_size.height.max(1);
            self.surface.configure(&self.device, &self.surface_config);
            self.pipelines[0].depth_texture = Texture::create_depth_texture(&self);
        }
    }
    pub fn update(&mut self, delta_time: f32, total_time: f32) {
        let rotation_increment = Quat::from_rotation_y(0.1);

        if self.camera_controller.movement_vector != Vec3::ZERO {
            self.player
                .camera
                .move_camera(&self.camera_controller.movement_vector, delta_time)
        }

        if self.player.camera.needs_update {
            let uniforms = Uniforms::from(&self.player.camera);
            self.queue.write_buffer(
                &self.pipelines[0].view_buffer,
                0,
                bytemuck::cast_slice(&[uniforms.view]),
            );

            self.player.camera.needs_update = false;
        }
    }
    pub fn draw(&mut self) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swapchain texture");
        // ?
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
            self.world.update(
                &mut self.player,
                Arc::clone(&self.queue),
                Arc::clone(&self.device),
            );

            for pipeline in self.pipelines.iter() {
                // let instances_buffer = pipeline.model.as_ref().borrow().instances_buffer.slice(..);
                rpass.set_pipeline(&pipeline.pipeline);

                rpass.set_bind_group(0, &pipeline.bind_group_0, &[]);
                rpass.set_bind_group(1, &pipeline.bind_group_1, &[]);
                for chunk in self.world.chunks.iter() {
                    rpass.set_bind_group(2, &chunk.chunk_bind_group, &[]);
                    rpass.set_vertex_buffer(0, chunk.chunk_vertex_buffer.slice(..));
                    rpass.set_index_buffer(
                        chunk.chunk_index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    rpass.draw_indexed(0..chunk.indices, 0, 0..1);
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
pub struct Config {
    pub polygon_mode: wgpu::PolygonMode,
}

pub struct State {
    pub surface: wgpu::Surface,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub pipelines: Vec<Pipeline>,
    pub player: Player,
    pub world: World,
    pub config: Config,
    pub camera_controller: CameraController,
    // pub model: Rc<RefCell<Model>>,
}
