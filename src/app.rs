//! Application framework module.
//!
//! Provides a generic application handler and renderer trait for wgpu/winit applications.

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::context::GraphicsContext;

/// Trait for application-specific rendering.
///
/// Implement this trait to create custom renderers that work with the application framework.
pub trait AppRenderer: Sized {
    /// Initialize rendering resources asynchronously.
    fn init(context: &GraphicsContext) -> impl std::future::Future<Output = Self>;

    /// Called when the window needs to be redrawn.
    fn render(&mut self, context: &mut GraphicsContext);

    /// Called on window resize (after the surface has been reconfigured).
    fn resize(&mut self, context: &mut GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>);

    /// Called when an input event occurs (e.g., key press).
    /// Default implementation does nothing.
    fn input(&mut self, _event: &WindowEvent) {}
}

/// Main application struct that handles the event loop and manages the graphics context.
pub struct App<R: AppRenderer> {
    context: Option<GraphicsContext>,
    renderer: Option<R>,
}

impl<R: AppRenderer> App<R> {
    /// Create a new application instance.
    pub fn new() -> Self {
        Self {
            context: None,
            renderer: None,
        }
    }
}

impl<R: AppRenderer> Default for App<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: AppRenderer + 'static> ApplicationHandler for App<R> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        // Initialize graphics context
        let context = pollster::block_on(GraphicsContext::new(
            event_loop.owned_display_handle(),
            window.clone(),
        ));

        // Initialize renderer
        let renderer = pollster::block_on(R::init(&context));

        self.context = Some(context);
        self.renderer = Some(renderer);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let context = self.context.as_mut().expect("Context not initialized");
        let renderer = self.renderer.as_mut().expect("Renderer not initialized");

        // Forward input events to renderer
        renderer.input(&event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                renderer.render(context);
                context.request_redraw();
            }
            WindowEvent::Resized(size) => {
                context.resize(size);
                renderer.resize(context, size);
            }
            _ => (),
        }
    }
}
