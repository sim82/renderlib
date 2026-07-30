//! Renderlib - A wgpu/winit application with a rotating triangle demo.

mod app;
mod context;
mod demo;
mod device_helpers;

use demo::DemoRenderer;

fn main() {
    // Initialize logger
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    // Use Poll control flow for games that want to render as fast as possible
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = app::App::<DemoRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
