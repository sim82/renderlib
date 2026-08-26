//! New application framework module using the improved architecture.
//!
//! This module provides a clean separation between GPU infrastructure and application state,
//! replacing the old `App` and `AppRenderer` system.

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

use crate::context::RenderContext;
use crate::device::GraphicsDevice;
use crate::state::AppState;

/// Trait for application-specific rendering using the new architecture.
///
/// This trait provides a clean interface for renderers that use the new
/// `RenderContext` for accessing both GPU infrastructure and application state.
pub trait NewAppRenderer: Sized {
    /// Initialize rendering resources asynchronously.
    fn init(context: RenderContext<'_>) -> impl std::future::Future<Output = Self>;

    /// Called when the window needs to be redrawn.
    fn render(&mut self, context: RenderContext<'_>);

    /// Called on window resize (after the surface has been reconfigured).
    fn resize(&mut self, context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>);

    /// Called when an input event occurs (e.g., key press).
    /// Default implementation does nothing.
    fn input(&mut self, _context: RenderContext<'_>, _event: &WindowEvent) {}
}

/// New application struct that uses the improved architecture with
/// separate GPU infrastructure and application state.
pub struct NewApplication<R: NewAppRenderer> {
    /// GPU infrastructure (immutable)
    device: Option<GraphicsDevice>,
    /// Application state (mutable)
    state: Option<AppState>,
    /// Renderer instance
    renderer: Option<R>,
    /// Window reference
    window: Option<Arc<Window>>,
}

impl<R: NewAppRenderer + 'static> NewApplication<R> {
    /// Create a new application instance.
    pub fn new() -> Self {
        Self {
            device: None,
            state: None,
            renderer: None,
            window: None,
        }
    }

    /// Get a render context for the current frame.
    pub fn create_render_context(
        &mut self,
        surface_texture: Option<wgpu::SurfaceTexture>,
    ) -> RenderContext {
        let device = self.device.as_ref().expect("Device not initialized");
        let state = self.state.as_mut().expect("State not initialized");
        // Convert surface_texture to texture_view
        let texture_view =
            surface_texture.map(|texture| device.surface_config.create_texture_view(&texture));
        RenderContext::new(device, state, texture_view)
    }
}

impl<R: NewAppRenderer + 'static> ApplicationHandler for NewApplication<R> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        // Initialize GPU infrastructure
        let device = pollster::block_on(GraphicsDevice::new(
            event_loop.owned_display_handle(),
            window.clone(),
        ));

        // Initialize application state
        let mut state = AppState::new(device.wgpu_device());

        // Create render context for initialization
        let context = RenderContext::new(&device, &mut state, None);

        // Initialize renderer
        let renderer = pollster::block_on(R::init(context));

        self.device = Some(device);
        self.state = Some(state);
        self.renderer = Some(renderer);
        self.window = Some(window.clone());

        // Request first redraw using the window we just created
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let device = self.device.as_ref().expect("Device not initialized");
        let state = self.state.as_mut().expect("State not initialized");
        let renderer = self.renderer.as_mut().expect("Renderer not initialized");

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Get current surface texture
                let surface_texture = device
                    .surface_config
                    .get_current_texture(device.wgpu_device());

                if let Some(texture) = surface_texture {
                    // Create texture view
                    let texture_view = device.surface_config.create_texture_view(&texture);

                    // Create render context with texture view
                    let context = RenderContext::new(device, state, Some(texture_view));

                    // Render
                    renderer.render(context);

                    // Present the texture
                    device.pre_present_notify();
                    device.queue.present(texture);

                    // Request next frame for continuous rendering
                    device.request_redraw();
                }
            }
            WindowEvent::Resized(size) => {
                // Resize the surface
                device.resize(size);

                let context = RenderContext::new(device, state, None);
                renderer.resize(context, size);
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Handle escape key
                if let Key::Named(NamedKey::Escape) = key_event.logical_key {
                    event_loop.exit();
                    return;
                }
            }
            other_event => {
                // Forward other events to renderer for input handling
                let context = RenderContext::new(device, state, None);
                renderer.input(context, &other_event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock renderer for testing
    struct MockRenderer;

    impl NewAppRenderer for MockRenderer {
        async fn init(_context: RenderContext<'_>) -> Self {
            MockRenderer
        }

        fn render(&mut self, _context: RenderContext<'_>) {}

        fn resize(
            &mut self,
            _context: RenderContext<'_>,
            _new_size: winit::dpi::PhysicalSize<u32>,
        ) {
        }
    }

    #[test]
    fn test_new_application_creation() {
        let app = NewApplication::<MockRenderer>::new();
        assert!(app.device.is_none());
        assert!(app.state.is_none());
        assert!(app.renderer.is_none());
    }
}
