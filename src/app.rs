//! Application framework module.
//!
//! Provides a generic application handler and renderer trait for wgpu/winit applications.

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

use crate::context::{GraphicsContext, RenderContext};
use crate::device::GraphicsDevice;
use crate::state::AppState;

/// Trait for application-specific rendering.
///
/// Implement this trait to create custom renderers that work with the application framework.
///
/// # New Architecture (Recommended)
///
/// For new code, implement the `init_new`, `render_new`, and `resize_new` methods
/// which use the new `RenderContext` for better separation of concerns:
///
/// ```ignore
/// impl AppRenderer for MyRenderer {
///     async fn init_new(context: RenderContext) -> Self {
///         // Use context.device() for GPU infrastructure
///         // Use context.state() for mutable state
///         Self::new(context.device(), context.state())
///     }
///
///     fn render_new(&mut self, context: RenderContext) {
///         // Render using the new context
///     }
/// }
/// ```
///
/// # Old Architecture (Deprecated)
///
/// For backward compatibility, the old methods using `GraphicsContext` are still available
/// but will be removed in a future version.
pub trait AppRenderer: Sized {
    /// Initialize rendering resources asynchronously (NEW).
    ///
    /// This is the preferred method for new code. Uses the new `RenderContext`
    /// which provides clean separation between GPU infrastructure and application state.
    fn init_new(context: RenderContext) -> impl std::future::Future<Output = Self> {
        // Default implementation bridges to old method for backward compatibility
        async {
            // This is a temporary bridge - in the future, renderers should implement init_new directly
            // For now, we need to extract GraphicsContext from the new types
            // This won't work perfectly until we have a proper bridge, so we'll panic for now
            panic!("init_new() not implemented. Either implement init_new() or use the old init() method.");
        }
    }

    /// Called when the window needs to be redrawn (NEW).
    fn render_new(&mut self, _context: RenderContext) {
        panic!("render_new() not implemented. Either implement render_new() or use the old render() method.");
    }

    /// Called on window resize (NEW).
    fn resize_new(&mut self, _context: RenderContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        panic!("resize_new() not implemented. Either implement resize_new() or use the old resize() method.");
    }

    /// Called when an input event occurs (NEW).
    fn input_new(&mut self, _context: RenderContext, _event: &WindowEvent) {}

    /// Initialize rendering resources asynchronously (OLD - Deprecated).
    ///
    /// This method is deprecated. Use `init_new()` instead for new code.
    #[deprecated(note = "Use init_new() with RenderContext instead")]
    fn init(context: &GraphicsContext) -> impl std::future::Future<Output = Self>;

    /// Called when the window needs to be redrawn (OLD - Deprecated).
    ///
    /// This method is deprecated. Use `render_new()` instead for new code.
    #[deprecated(note = "Use render_new() with RenderContext instead")]
    fn render(&mut self, context: &mut GraphicsContext);

    /// Called on window resize (after the surface has been reconfigured) (OLD - Deprecated).
    ///
    /// This method is deprecated. Use `resize_new()` instead for new code.
    #[deprecated(note = "Use resize_new() with RenderContext instead")]
    fn resize(&mut self, context: &mut GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>);

    /// Called when an input event occurs (e.g., key press) (OLD - Deprecated).
    /// Default implementation does nothing.
    #[deprecated(note = "Use input_new() with RenderContext instead")]
    fn input(&mut self, _event: &WindowEvent) {}
}

/// Main application struct that handles the event loop and manages the graphics context.
///
/// This is the old application struct for backward compatibility.
/// Use `Application` for new code with the improved architecture.
#[deprecated(note = "Use Application<R> instead for new architecture")]
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

/// New application struct that uses the improved architecture with
/// separate GPU infrastructure and application state.
///
/// This provides better separation of concerns and removes the need for
/// interior mutability in the mesh cache.
pub struct Application<R: AppRenderer> {
    /// GPU infrastructure (immutable)
    device: Option<GraphicsDevice>,
    /// Application state (mutable)
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

        // Initialize renderer using old method
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
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Exit on Escape key
                if let Key::Named(NamedKey::Escape) = key_event.logical_key {
                    event_loop.exit();
                }
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

        // Initialize renderer using new method
        let renderer = pollster::block_on(R::init_new(context));

        self.device = Some(device);
        self.state = Some(state);
        self.renderer = Some(renderer);
        self.window = Some(window);

        // Request first redraw
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let device = self.device.as_ref().expect("Device not initialized");
        let state = self.state.as_mut().expect("State not initialized");
        let renderer = self.renderer.as_mut().expect("Renderer not initialized");

        // Forward input events to renderer first
        let context = RenderContext::new(device, state, None);
        renderer.input_new(context, &event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Exit on Escape key
                if let Key::Named(NamedKey::Escape) = key_event.logical_key {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                // Get current surface texture
                let surface_texture = device
                    .surface_config
                    .get_current_texture(device.wgpu_device());

                if let Some(texture) = surface_texture {
                    // Create render context without texture (renderer will get texture view from device)
                    let context = RenderContext::new(device, state, None);

                    // Render using new method
                    renderer.render_new(context);

                    // Present the texture
                    device.pre_present_notify();
                    device.queue.present(texture);
                }

                // Request next frame
                device.request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Resize the surface
                device.resize(size);

                let context = RenderContext::new(device, state, None);
                renderer.resize_new(context, size);
            }
            _ => (),
        }
    }
}
