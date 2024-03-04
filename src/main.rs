use std::sync::{Arc, Mutex};
use std::{
    fs::File,
    io::BufReader,
    mem,
    ops::ControlFlow,
    process::exit,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use glam::vec2;
use material::Texture;
use player::CameraController;
use state::State;
use tobj::{load_obj, load_obj_buf, LoadOptions};
use winit::keyboard::KeyCode;
use winit::window::CursorGrabMode;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::*,
    event_loop::EventLoop,
    keyboard::{Key, NamedKey, PhysicalKey},
    window::Window,
};

const DEFAULT_WINDOW_WIDTH: u32 = 1200;
const DEFAULT_WINDOW_HEIGHT: u32 = 800;

#[macro_use]
extern crate lazy_static;

pub mod blocks;
pub mod chunk;
pub mod collision;
pub mod material;
pub mod pipeline;
pub mod player;
pub mod state;
pub mod structures;
pub mod ui;
pub mod utils;
pub mod world;

async fn run(event_loop: EventLoop<()>, window: Window) {
    // let model: Obj = load_obj(input).unwrap();

    let start = Instant::now();
    let mut total_time = start.elapsed();
    let mut delta_time = start.elapsed();

    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.width = size.height.max(1);

    window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
    window.set_cursor_visible(false);
    let window = Arc::new(Mutex::new(window));
    let mut state = State::new(window.clone()).await;

    let mut prev_mouse_pos = glam::vec2(0.0, 0.0);
    let mut cursor_in = false;

    event_loop
        .run(move |event, target| {
            // let _ = &(instance, &adapter, &shader, pipeline_layout);

            if let Event::WindowEvent {
                window_id: _,
                event,
            } = event
            {
                match event {
                    WindowEvent::Resized(new_size) => {
                        state.resize(new_size);
                        window.lock().unwrap().request_redraw();
                    }
                    // WindowEvent::RedrawRequested => {}
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key: Key::Named(NamedKey::Escape),
                                ..
                            },
                        ..
                    } => target.exit(),

                    WindowEvent::KeyboardInput { event, .. } => {
                        state.handle_keypress(event, delta_time.as_secs_f32())
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button,
                        ..
                    } => {
                        state.on_click(button);
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        if !cursor_in {
                            prev_mouse_pos.x = position.x as f32;
                            prev_mouse_pos.y = position.y as f32;
                            cursor_in = true;
                        }

                        let delta = glam::vec2(
                            prev_mouse_pos.x - position.x as f32,
                            prev_mouse_pos.y - position.y as f32,
                        );
                        prev_mouse_pos.x = position.x as f32;
                        prev_mouse_pos.y = position.y as f32;

                        // state.handle_mouse(&delta);
                    }
                    WindowEvent::CursorLeft { .. } => cursor_in = false,
                    WindowEvent::RedrawRequested => {
                        delta_time = start.elapsed() - total_time;
                        total_time = start.elapsed();

                        state.update(delta_time.as_secs_f32(), total_time.as_secs_f32());
                        state.draw();

                        window.lock().unwrap().request_redraw();
                    }

                    _ => {}
                };
            } else if let Event::DeviceEvent { event, .. } = event {
                match event {
                    DeviceEvent::MouseMotion { delta } => {
                        state.handle_mouse(&glam::vec2(delta.0 as f32, delta.1 as f32))
                    }
                    _ => {}
                }
            }
        })
        .unwrap()
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let builder = winit::window::WindowBuilder::new();
    let window = builder
        .with_inner_size(PhysicalSize::new(
            DEFAULT_WINDOW_WIDTH,
            DEFAULT_WINDOW_HEIGHT,
        ))
        .build(&event_loop)
        .unwrap();

    env_logger::init();
    pollster::block_on(run(event_loop, window))
}
