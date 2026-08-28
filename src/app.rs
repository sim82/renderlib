//! Application framework module.
//!
//! This module provides the main application framework for renderlib. It implements
//! the winit [`ApplicationHandler`] trait and manages the application lifecycle,
//! including window creation, GPU initialization, and renderer integration.
//!
//! # Main Types
//!
//! - [`Application<R>`] - The main application struct that implements [`ApplicationHandler`]
//! - [`AppRenderer`] - Trait that all renderers must implement
//!
//! # Usage
//!
//! Create a renderer struct and implement [`AppRenderer`]:
//!
//! ```no_run
//! use renderlib::app::{AppRenderer, Application};
//! use renderlib::context::RenderContext;
//!
//! struct MyRenderer {
//!     render_pipeline: wgpu::RenderPipeline,
//! }
//!
//! impl AppRenderer for MyRenderer {
//!     async fn init(mut context: RenderContext<'_>) -> Self {
//!         let device = context.wgpu_device();
//!         // Create your render pipeline and other resources
//!         # panic!("example not complete");
//!     }
//!
//!     fn render(&mut self, mut context: RenderContext<'_>) {
//!         let texture_view = context.get_texture_view().unwrap();
//!         // Render your scene
//!     }
//!
//!     fn resize(&mut self, _context: RenderContext<'_>, _size: winit::dpi::PhysicalSize<u32>) {
//!         // Recreate size-dependent resources
//!     }
//!
//!     fn input(&mut self, _context: RenderContext<'_>, _event: &winit::event::WindowEvent) {
//!         // Handle input events
//!     }
//! }
//!
//! fn main() {
//!     let event_loop = winit::event_loop::EventLoop::new().unwrap();
//!     let mut app = Application::<MyRenderer>::new();
//!     event_loop.run_app(&mut app).unwrap();
//! }
//! ```

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

/// Trait for application-specific rendering.
///
/// Implement this trait to create a custom renderer. The [`Application`] struct
/// will call these methods at appropriate times during the application lifecycle.
///
/// All methods receive a [`RenderContext`] which provides access to both
/// GPU infrastructure (via [`GraphicsDevice`]) and application state (via [`AppState`]).
pub trait AppRenderer: Sized {
    /// Initialize rendering resources asynchronously.
    ///
    /// Called once when the application starts, after the window and GPU resources
    /// have been initialized. Use this to create your render pipeline, buffers, textures,
    /// and other GPU resources.
    fn init(context: RenderContext<'_>) -> impl std::future::Future<Output = Self>;

    /// Called when the window needs to be redrawn.
    ///
    /// This is the main rendering method, called for each frame. Use the provided
    /// [`RenderContext`] to access the GPU device, queue, current texture view, and
    /// application state.
    fn render(&mut self, context: RenderContext<'_>);

    /// Called on window resize (after the surface has been reconfigured).
    ///
    /// Recreate any size-dependent resources here, such as depth textures,
    /// render targets, or camera projection matrices.
    fn resize(&mut self, context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>);

    /// Called when an input event occurs (e.g., key press, mouse movement).
    ///
    /// Override this to handle user input. The default implementation does nothing.
    fn input(&mut self, _context: RenderContext<'_>, _event: &WindowEvent) {}
}

/// Main application struct.
///
/// This struct implements winit's [`ApplicationHandler`] trait and manages the
/// application lifecycle. It holds the GPU infrastructure, application state,
/// and renderer instance.
pub struct Application<R: AppRenderer> {
    /// GPU infrastructure (device, queue, surface)
    device: Option<GraphicsDevice>,
    /// Application state (mesh cache, camera, input, time)
    state: Option<AppState>,
    /// Renderer instance
    renderer: Option<R>,
    /// Window reference
    window: Option<Arc<Window>>,
}

impl<R: AppRenderer + 'static> Application<R> {
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
    ///
    /// Creates a [`RenderContext`] with references to the GPU device, application state,
    /// and the current surface texture view.
    pub fn create_render_context(
        &mut self,
        surface_texture: Option<wgpu::SurfaceTexture>,
    ) -> RenderContext<'_> {
        let device = self.device.as_ref().expect("Device not initialized");
        let state = self.state.as_mut().expect("State not initialized");
        // Convert surface_texture to texture_view
        let texture_view =
            surface_texture.map(|texture| device.surface_config.create_texture_view(&texture));
        RenderContext::new(device, state, texture_view)
    }
}

impl<R: AppRenderer + 'static> Default for Application<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: AppRenderer + 'static> ApplicationHandler for Application<R> {
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

        match event.clone() {
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

                // Forward keyboard input to renderer for input handling
                let context = RenderContext::new(device, state, None);
                renderer.input(context, &event);
            }
            _ => {
                // Forward all events to renderer for input handling
                let context = RenderContext::new(device, state, None);
                renderer.input(context, &event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock renderer for testing
    struct MockRenderer;

    impl AppRenderer for MockRenderer {
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
    fn test_application_creation() {
        let app = Application::<MockRenderer>::new();
        assert!(app.device.is_none());
        assert!(app.state.is_none());
        assert!(app.renderer.is_none());
    }
}
