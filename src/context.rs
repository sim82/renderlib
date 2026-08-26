//! Graphics context module - provides RenderContext for the new architecture.
//!
//! This module provides the RenderContext struct which combines immutable GPU
//! infrastructure with mutable application state for renderer access.

use crate::device::GraphicsDevice;
use crate::state::AppState;

/// A context passed to renderers that provides access to both
/// immutable GPU infrastructure and mutable application state.
///
/// This struct holds references to both `GraphicsDevice` (immutable infrastructure)
/// and `AppState` (mutable state), allowing renderers to access all necessary
/// resources for rendering while maintaining a clean separation of concerns.
///
/// # Example
///
/// ```ignore
/// use renderlib::context::RenderContext;
///
/// fn render(&mut self, context: RenderContext) {
///     // Access GPU infrastructure
///     let device = context.device();
///     let queue = context.queue();
///
///     // Access mutable state
///     let mesh_cache = context.state().mesh_cache;
///     let camera = &context.state().camera;
/// }
/// ```
pub struct RenderContext<'a> {
    /// Immutable GPU infrastructure
    device: &'a GraphicsDevice,
    /// Mutable application state
    state: &'a mut AppState,
    /// Current texture view (optional, for rendering)
    texture_view: Option<wgpu::TextureView>,
}

impl<'a> RenderContext<'a> {
    /// Create a new render context.
    ///
    /// # Arguments
    ///
    /// * `device` - Reference to the GPU infrastructure
    /// * `state` - Mutable reference to the application state
    /// * `texture_view` - Optional texture view for the current frame
    pub fn new(
        device: &'a GraphicsDevice,
        state: &'a mut AppState,
        texture_view: Option<wgpu::TextureView>,
    ) -> Self {
        Self {
            device,
            state,
            texture_view,
        }
    }

    /// Get a reference to the GPU device.
    pub fn device(&self) -> &GraphicsDevice {
        self.device
    }

    /// Get a reference to the wgpu device.
    pub fn wgpu_device(&self) -> &wgpu::Device {
        &self.device.device
    }

    /// Get a reference to the wgpu queue.
    pub fn wgpu_queue(&self) -> &wgpu::Queue {
        &self.device.queue
    }

    /// Get a mutable reference to the application state.
    pub fn state(&mut self) -> &mut AppState {
        self.state
    }

    /// Take the current texture view, leaving None in its place.
    /// This is used when the texture view is no longer needed.
    pub fn take_texture_view(&mut self) -> Option<wgpu::TextureView> {
        self.texture_view.take()
    }

    /// Get the current texture view.
    pub fn texture_view(&self) -> Option<&wgpu::TextureView> {
        self.texture_view.as_ref()
    }

    /// Get the texture view.
    ///
    /// Returns the texture view that was passed to the context, or None if not available.
    pub fn get_texture_view(&self) -> Option<&wgpu::TextureView> {
        self.texture_view.as_ref()
    }

    /// Request a redraw of the window.
    pub fn request_redraw(&self) {
        self.device.request_redraw();
    }

    /// Notify the window before presenting.
    pub fn pre_present_notify(&self) {
        self.device.pre_present_notify();
    }

    /// Get the current window size.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.device.size()
    }

    /// Get the surface format.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.device.surface_format()
    }
}
