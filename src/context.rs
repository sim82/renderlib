//! Render context module.
//!
//! This module provides the [`RenderContext`] struct, which is the primary interface
//! for renderers to access both GPU infrastructure and application state.
//!
//! [`RenderContext`] is passed to all [`AppRenderer`] methods and provides:
//! - Access to GPU resources via [`GraphicsDevice`]
//! - Access to application state via [`AppState`]
//! - Access to the current frame's texture view

use std::time::Instant;

use crate::device::GraphicsDevice;
use crate::state::AppState;

/// Context passed to renderers providing access to resources.
///
/// `RenderContext` holds references to both the GPU infrastructure ([`GraphicsDevice`])
/// and the application state ([`AppState`]), allowing renderers to access all necessary
/// resources while maintaining a clean separation between immutable and mutable data.
///
/// # Example
///
/// ```no_run
/// use renderlib::context::RenderContext;
///
/// fn render(&mut self, mut context: RenderContext<'_>) {
///     // Access GPU infrastructure
///     let device = context.wgpu_device();
///     let queue = context.wgpu_queue();
///
///     // Access mutable state
///     let camera = &context.state().camera;
///     let texture_view = context.get_texture_view().unwrap();
///
///     // Use resources for rendering
/// }
/// ```
pub struct RenderContext<'a> {
    /// Reference to the GPU infrastructure
    device: &'a GraphicsDevice,
    /// Mutable reference to the application state
    state: &'a mut AppState,
    /// Current texture view for rendering (optional)
    texture_view: Option<wgpu::TextureView>,
}

impl<'a> RenderContext<'a> {
    /// Creates a new render context.
    ///
    /// # Arguments
    ///
    /// * `device` - Reference to the [`GraphicsDevice`]
    /// * `state` - Mutable reference to the [`AppState`]
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

    /// Returns a reference to the [`GraphicsDevice`].
    pub fn device(&self) -> &GraphicsDevice {
        self.device
    }

    /// Returns a reference to the underlying wgpu device.
    pub fn wgpu_device(&self) -> &wgpu::Device {
        &self.device.device
    }

    /// Returns a reference to the underlying wgpu queue.
    pub fn wgpu_queue(&self) -> &wgpu::Queue {
        &self.device.queue
    }

    /// Returns a mutable reference to the [`AppState`].
    pub fn state(&mut self) -> &mut AppState {
        self.state
    }

    /// Takes the current texture view, leaving `None` in its place.
    ///
    /// Use this when you need ownership of the texture view.
    pub fn take_texture_view(&mut self) -> Option<wgpu::TextureView> {
        self.texture_view.take()
    }

    /// Returns a reference to the current texture view, if available.
    pub fn texture_view(&self) -> Option<&wgpu::TextureView> {
        self.texture_view.as_ref()
    }

    /// Returns a reference to the texture view.
    ///
    /// This is an alias for [`texture_view()`](RenderContext::texture_view).
    pub fn get_texture_view(&self) -> Option<&wgpu::TextureView> {
        self.texture_view.as_ref()
    }

    /// Requests a redraw of the window.
    pub fn request_redraw(&self) {
        self.device.request_redraw();
    }

    /// Notifies the window before presenting.
    pub fn pre_present_notify(&self) {
        self.device.pre_present_notify();
    }

    /// Returns the current window size in physical pixels.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.device.size()
    }

    /// Returns the surface texture format.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.device.surface_format()
    }
}

/// Profiling timer for measuring execution intervals.
///
/// `Proftime` provides a simple way to measure and log execution times between
/// checkpoints. When dropped, it automatically prints the duration of each
/// interval and the total time.
///
/// # Example
///
/// ```no_run
/// use renderlib::context::Proftime;
///
/// fn my_function() {
///     let mut pt = Proftime::new();
///     // Do some work
///     pt.checkpoint("first operation");
///     // Do more work
///     pt.checkpoint("second operation");
///     // Proftime will automatically print timing info when it goes out of scope
/// }
/// ```
#[derive(Debug)]
pub struct Proftime {
    /// Start time of the profiling session
    pub start: Instant,
    /// List of named checkpoints with their timestamps
    pub interval: Vec<(String, Instant)>,
}

impl Proftime {
    /// Creates a new `Proftime` instance.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            interval: Vec::new(),
        }
    }

    /// Records a checkpoint with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - A string describing the checkpoint
    pub fn checkpoint(&mut self, name: &str) {
        self.interval.push((name.to_string(), Instant::now()));
    }
}

impl Default for Proftime {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Proftime {
    fn drop(&mut self) {
        let mut last = self.start;
        for (name, time) in &self.interval {
            println!("{}: {:?}", name, time.duration_since(last));
            last = *time;
        }
        println!("total: {:?}", last.duration_since(self.start));
    }
}
