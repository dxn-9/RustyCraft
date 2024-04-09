use std::sync::{Mutex, RwLock};
use std::time::Instant;
use std::{f32::consts, sync::Arc};

use crate::blocks::block::Block;
use crate::blocks::block_type::BlockType;
use crate::collision::CollisionBox;
use crate::perf;
use crate::persistence::Saveable;
use crate::pipeline::{Pipeline, PipelineTrait};
use crate::utils::{ChunkFromPosition, RelativeFromAbsolute};
use crate::water::WaterPipeline;
use crate::{
    material::Texture,
    pipeline::{self, Uniforms},
    player::{Camera, CameraController, Player},
    ui::{UIPipeline, UI},
    world::World,
};
use winit::event::MouseButton;
use winit::window::CursorGrabMode;
use winit::{
    dpi::PhysicalSize,
    event::KeyEvent,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

impl State {
    pub async fn new(window: Arc<Mutex<Window>>) -> Self {
        let windowbrw = window.lock().unwrap();
        let size = windowbrw.inner_size();
        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(&*windowbrw).unwrap() };
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

        let camera = Camera::new(
            surface_config.width as f32,
            surface_config.height as f32,
            device.clone(),
            queue.clone(),
        );
        let current_chunk = camera.eye.get_chunk_from_position_absolute();
        let player = Arc::new(RwLock::new(Player {
            camera,
            current_chunk,
            is_jumping: false,
            on_ground: false,
            facing_block: None,
            facing_face: None,
            jump_action_start: None,
            is_ghost: false,
        }));

        surface.configure(&device, &surface_config);
        let config = Config {
            polygon_mode: wgpu::PolygonMode::Fill,
        };

        let mut world = World::init_world(device.clone(), queue.clone());
        world.init_chunks(Arc::clone(&player));
        let ui = UI::new(device.clone(), queue.clone());

        let mut state = Self {
            config,
            player,
            ui,
            pipelines: vec![],
            surface_config,
            instance,
            window: window.clone(),
            device,
            world,
            queue,
            surface,
            adapter,
            camera_controller: CameraController::default(),
        };

        let world_pipeline = Box::new(Pipeline::new(&state));
        let water_pipeline = Box::new(WaterPipeline::new(&state));
        let ui_pipeline = Box::new(UIPipeline::new(&state));

        state.pipelines.push(world_pipeline);
        state.pipelines.push(water_pipeline);
        state.pipelines.push(ui_pipeline);

        state
    }
    pub fn save_state(&mut self) {
        self.player
            .read()
            .unwrap()
            .camera
            .save()
            .expect("Failed to save camera state");
        self.world.save_state();
    }
    pub fn dispose(&mut self) {
        self.world.dispose();
        self.device.destroy();
        std::mem::drop(self.queue.to_owned());
    }
    pub fn handle_keypress(&mut self, event: KeyEvent, delta_time: f32) {
        let is_pressed: f32 = if event.state.is_pressed() { 1. } else { 0. };
        let mut player = self.player.write().unwrap();

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
                physical_key: PhysicalKey::Code(KeyCode::KeyK),
                ..
            } => self
                .window
                .lock()
                .unwrap()
                .set_cursor_grab(CursorGrabMode::Confined)
                .unwrap(),
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::Space),
                state: winit::event::ElementState::Pressed,
                ..
            } => {
                if player.on_ground {
                    player.is_jumping = true;
                    player.jump_action_start = Some(std::time::Instant::now());
                }
            }
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyG),
                state: winit::event::ElementState::Pressed,
                ..
            } => {
                player.is_ghost = !player.is_ghost;
            }
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
            }
            _ => {}
        }
    }
    pub fn on_click(&mut self, button: MouseButton) {
        let player = self.player.read().unwrap();
        if let Some(facing_block) = player.facing_block.as_ref() {
            let facing_face = player
                .facing_face
                .expect("Cannot be not facing a face if it's facing a block");
            match button {
                MouseButton::Left => {
                    self.world.remove_block(facing_block.clone());
                }
                MouseButton::Right => {
                    let block_borrow = facing_block.read().unwrap();
                    let new_block_abs_position =
                        block_borrow.absolute_position + facing_face.get_normal_vector();

                    let chunk = new_block_abs_position.get_chunk_from_position_absolute();
                    let position = new_block_abs_position.relative_from_absolute();

                    let new_block =
                        Arc::new(RwLock::new(Block::new(position, chunk, BlockType::Dirt)));

                    self.world.place_block(new_block);
                }
                _ => {}
            }
        }
    }
    pub fn handle_mouse(&mut self, delta: &glam::Vec2) {
        self.player.write().unwrap().camera.move_target(delta)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width.max(1);
            self.surface_config.height = new_size.height.max(1);
            self.surface.configure(&self.device, &self.surface_config);
            let new_depth = Texture::create_depth_texture(&self);
            self.pipelines[0].set_depth_texture(new_depth);
        }
    }
    pub fn update(&mut self, delta_time: f32, total_time: f32) {
        let mut collisions = vec![];

        let start_update = Instant::now();
        if let Some(nearby_blocks) = self.world.get_blocks_nearby(Arc::clone(&self.player)) {
            for block in nearby_blocks.iter() {
                let block = block.read().unwrap();
                let collision = CollisionBox::from_block_position(
                    block.absolute_position.x,
                    block.position.y,
                    block.absolute_position.z,
                );
                collisions.push(collision);
            }
        };
        let mut player = self.player.write().unwrap();
        player.move_camera(
            &self.camera_controller.movement_vector,
            delta_time,
            &collisions,
        );
        player.update();
        if let Some((block, face_dir)) = player.get_facing_block(&collisions) {
            let block = self.world.get_blocks_absolute(&block.to_block_position());

            player.facing_block = block;
            player.facing_face = Some(face_dir);
        } else {
            player.facing_block = None;
            player.facing_face = None;
        }

        let uniforms = Uniforms::from(&player.camera);

        for pipeline in self.pipelines.iter() {
            self.queue.write_buffer(
                pipeline.view_buffer(),
                0,
                bytemuck::cast_slice(&[uniforms.view]),
            )
        }
        // Drop write lock
        std::mem::drop(player);

        self.world.update(
            Arc::clone(&self.player),
            Arc::clone(&self.queue),
            Arc::clone(&self.device),
        );
        self.ui.update(
            Arc::clone(&self.player),
            Arc::clone(&self.queue),
            Arc::clone(&self.device),
        );
    }
    pub fn draw(&mut self) {
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
        let chunk_map = self.world.chunks.read().unwrap();
        let chunks = chunk_map
            .values()
            .map(|f| f.read().unwrap())
            .collect::<Vec<_>>();

        let player = self.player.read().unwrap();

        {
            let mut main_rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                    view: &self.pipelines[0].depth_texture().view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let pipeline = &self.pipelines[0];

            main_rpass.set_pipeline(pipeline.pipeline());

            main_rpass.set_bind_group(0, pipeline.bind_group_0(), &[]);
            main_rpass.set_bind_group(1, pipeline.bind_group_1(), &[]);

            main_rpass.set_bind_group(3, &player.camera.position_bind_group, &[]);

            for chunk in chunks.iter() {
                if chunk.visible {
                    main_rpass.set_bind_group(2, &chunk.chunk_bind_group, &[]);
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
                };
            }
        }
        {
            let mut water_rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.pipelines[0].depth_texture().view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let pipeline = &self.pipelines[1];

            water_rpass.set_pipeline(pipeline.pipeline());

            water_rpass.set_bind_group(0, pipeline.bind_group_0(), &[]);
            water_rpass.set_bind_group(1, pipeline.bind_group_1(), &[]);
            water_rpass.set_bind_group(3, &player.camera.position_bind_group, &[]);

            for chunk in chunks.iter() {
                if chunk.visible {
                    water_rpass.set_bind_group(2, &chunk.chunk_bind_group, &[]);
                    water_rpass.set_vertex_buffer(
                        0,
                        chunk
                            .chunk_water_vertex_buffer
                            .as_ref()
                            .expect("Vertex buffer not initiated")
                            .slice(..),
                    );
                    water_rpass.set_index_buffer(
                        chunk
                            .chunk_water_index_buffer
                            .as_ref()
                            .expect("Index buffer not initiated")
                            .slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    water_rpass.draw_indexed(0..chunk.water_indices, 0, 0..1);
                };
            }
        }
        {
            let mut ui_renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.pipelines[0].depth_texture().view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let pipeline = &self.pipelines[2];
            ui_renderpass.set_pipeline(pipeline.pipeline());

            ui_renderpass.set_bind_group(0, pipeline.bind_group_0(), &[]);
            ui_renderpass.set_bind_group(1, pipeline.bind_group_1(), &[]);

            ui_renderpass.set_vertex_buffer(0, self.ui.vertex_buffer.slice(..));
            ui_renderpass
                .set_index_buffer(self.ui.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            ui_renderpass.draw_indexed(0..self.ui.indices as u32, 0, 0..1);
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
    pub window: Arc<Mutex<Window>>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub pipelines: Vec<Box<dyn PipelineTrait>>,
    pub player: Arc<RwLock<Player>>,
    pub world: World,
    pub ui: UI,
    pub config: Config,
    pub camera_controller: CameraController,
}
