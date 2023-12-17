use std::{
    mem,
    ops::ControlFlow,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use camera::CameraController;
use glam::vec2;
use model::Vertex;
use state::State;
use winit::{
    dpi::PhysicalPosition,
    event::*,
    event_loop::{DeviceEvents, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::Window,
};

mod camera;
mod model;
mod pipeline;
mod state;
mod texture;

async fn run(event_loop: EventLoop<()>, window: Window) {
    let start = Instant::now();
    let mut previous_frame = start.elapsed();
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
                        state.update(delta_time.as_secs_f32());
                        state.draw();
                        delta_time = start.elapsed() - previous_frame;
                        previous_frame = start.elapsed();

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
