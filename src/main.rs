use std::{mem, ops::ControlFlow};

use bytemuck::{Pod, Zeroable};
use model::Vertex;
use state::State;
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{Key, NamedKey, PhysicalKey},
    window::Window,
};

mod camera;
mod model;
mod state;
mod texture;

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.width = size.height.max(1);

    let mut state = State::new(&window).await;
    let window = &window;

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
                    WindowEvent::RedrawRequested => {
                        state.update();
                        state.draw();

                        window.request_redraw();
                    }

                    _ => {}
                };
            };
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
