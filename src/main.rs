use std::{
    fs::File,
    io::BufReader,
    mem,
    ops::ControlFlow,
    process::exit,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use camera::CameraController;
use glam::vec2;
use material::Texture;
use model::VertexData;
use state::State;
use tobj::{load_obj, load_obj_buf, LoadOptions};
use winit::{
    dpi::PhysicalPosition,
    event::*,
    event_loop::{DeviceEvents, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::Window,
};

#[macro_use]
extern crate lazy_static;

mod camera;
mod material;
mod model;
mod pipeline;
mod state;
mod utils;
mod world;

async fn run(event_loop: EventLoop<()>, window: Window) {
    // let model: Obj = load_obj(input).unwrap();

    let start = Instant::now();
    let mut total_time = start.elapsed();
    let mut delta_time = start.elapsed();

    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.width = size.height.max(1);

    let mut state = State::new(&window).await;
    let window = &window;

    window
        .set_cursor_grab(winit::window::CursorGrabMode::Confined)
        .unwrap();

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
                        window.request_redraw();
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

                        window.request_redraw();
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
    let window = builder.build(&event_loop).unwrap();

    env_logger::init();
    pollster::block_on(run(event_loop, window))
}
