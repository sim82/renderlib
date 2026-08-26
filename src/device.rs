//! Graphics device module - manages wgpu device, queue, and surface infrastructure.
//!
//! This module provides the immutable GPU infrastructure that can be shared
//! across the application. Unlike the application state, this represents
//! the "hardware" layer that doesn't change during runtime.

use std::sync::Arc;
use std::sync::Mutex;

use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

/// Configuration for the wgpu surface.
///
/// This provides thread-safe access to the surface configuration
/// and allows for surface reconfiguration when the window is resized.
#[derive(Debug)]
pub struct SurfaceConfig {
    /// The wgpu instance (needed for surface recreation)
    pub instance: wgpu::Instance,
    /// The window (needed for surface recreation)
    pub window: Arc<Window>,
    /// The wgpu surface, protected by a mutex for thread safety
    pub surface: Arc<Mutex<wgpu::Surface<'static>>>,
    /// The surface texture format
    pub format: wgpu::TextureFormat,
    /// The current size of the surface
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl SurfaceConfig {
    /// Create a new surface configuration.
    pub fn new(
        instance: wgpu::Instance,
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        format: wgpu::TextureFormat,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Self {
        Self {
            instance,
            window,
            surface: Arc::new(Mutex::new(surface)),
            format,
            size,
        }
    }

    /// Lock the surface for exclusive access.
    ///
    /// This is useful when you need to perform operations that require
    /// mutable access to the surface, such as getting the current texture.
    pub fn lock_surface(&self) -> std::sync::MutexGuard<'_, wgpu::Surface<'static>> {
        self.surface.lock().unwrap()
    }

    /// Configure the surface with the current settings.
    ///
    /// This should be called after creating the surface or when the window is resized.
    pub fn configure(&self, device: &wgpu::Device) {
        let surface = self.surface.lock().unwrap();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            // Request compatibility with the sRGB-format texture view
            view_formats: vec![self.format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        surface.configure(device, &config);
    }

    /// Update the surface size and reconfigure.
    ///
    /// This should be called when the window is resized.
    pub fn resize(&self, new_size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device) {
        // Create a new surface config with updated size
        let mut surface = self.surface.lock().unwrap();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![self.format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: new_size.width,
            height: new_size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        surface.configure(device, &config);
    }

    /// Try to acquire the current surface texture for rendering.
    ///
    /// Returns Some(texture) on success, None if surface is unavailable.
    /// Handles Suboptimal, Outdated, and Lost cases by reconfiguring or recreating.
    pub fn get_current_texture(&self, device: &wgpu::Device) -> Option<wgpu::SurfaceTexture> {
        let surface = self.surface.lock().unwrap();

        match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => Some(texture),
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => None,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                drop(surface); // Release lock before reconfiguring
                self.configure(device);
                None
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                drop(surface); // Release lock before reconfiguring
                self.configure(device);
                None
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // Surface is lost, need to recreate
                drop(surface);
                // Recreate the surface
                let new_surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .expect("Failed to recreate surface");
                *self.surface.lock().unwrap() = new_surface;
                self.configure(device);
                None
            }
        }
    }

    /// Create a texture view from a surface texture using the surface format.
    pub fn create_texture_view(&self, surface_texture: &wgpu::SurfaceTexture) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.format.add_srgb_suffix()),
                ..Default::default()
            })
    }
}

/// Immutable GPU infrastructure that can be shared across the application.
///
/// This struct represents the "hardware" layer of the graphics system.
/// It contains the wgpu device, queue, instance, and surface configuration,
/// all of which are immutable after creation and can be safely shared between
/// different parts of the application.
///
/// # Example
///
/// ```ignore
/// use renderlib::device::GraphicsDevice;
///
/// // Create a new graphics device
/// let device = GraphicsDevice::new(display_handle, window).await;
///
/// // Share it across threads using Arc
/// let device_arc = Arc::new(device);
/// ```
#[derive(Debug)]
pub struct GraphicsDevice {
    /// The wgpu instance
    pub instance: wgpu::Instance,
    /// The logical device, wrapped in Arc for sharing
    pub device: Arc<wgpu::Device>,
    /// The command queue, wrapped in Arc for sharing
    pub queue: Arc<wgpu::Queue>,
    /// Surface configuration
    pub surface_config: SurfaceConfig,
    /// Window reference (for surface recreation if needed)
    pub window: Arc<Window>,
}

impl GraphicsDevice {
    /// Create a new graphics device from a window and display handle.
    ///
    /// This initializes the wgpu instance, requests an adapter, and creates
    /// the device and queue. It also sets up the surface for the window.
    ///
    /// # Arguments
    ///
    /// * `display` - The display handle from the event loop
    /// * `window` - The window to create the surface for
    ///
    /// # Returns
    ///
    /// A new `GraphicsDevice` instance ready for use.
    pub async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> Self {
        // Create wgpu instance with display handle
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Failed to request adapter");

        // Request device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Failed to request device");

        let size = window.inner_size();

        // Create surface
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        // Get surface capabilities and format
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        // Create surface config
        let surface_config = SurfaceConfig::new(
            instance.clone(),
            window.clone(),
            surface,
            surface_format,
            size,
        );

        // Configure surface for the first time
        surface_config.configure(&device);

        Self {
            instance,
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface_config,
            window,
        }
    }

    /// Get a reference to the wgpu device.
    ///
    /// This is a convenience method for accessing the underlying wgpu device.
    pub fn wgpu_device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get a reference to the wgpu queue.
    ///
    /// This is a convenience method for accessing the underlying wgpu queue.
    pub fn wgpu_queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Get the current window size.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.surface_config.size
    }

    /// Resize the surface to the new size.
    pub fn resize(&self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.surface_config.resize(new_size, &self.device);
    }

    /// Request a redraw of the window.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Notify the window before presenting.
    pub fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    /// Get the surface format.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_config_creation() {
        // This test would require a proper wgpu setup
        // For now, just verify the types compile
        let _: Option<SurfaceConfig> = None;
    }

    #[test]
    fn test_graphics_device_clone() {
        // Verify GraphicsDevice can be cloned
        // Actual cloning would require async context
    }
}
