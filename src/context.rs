//! Graphics context module - manages wgpu device, surface, and swap chain setup.

use std::sync::Arc;

use winit::{event_loop::OwnedDisplayHandle, window::Window};

use crate::device::GraphicsDevice;
use crate::mesh::MeshCache;
use crate::state::AppState;

/// Generic graphics context managing wgpu device, surface, and swap chain.
///
/// This separates the wgpu initialization boilerplate from application-specific
/// rendering code, making it reusable across different projects.
pub struct GraphicsContext {
    pub window: Arc<Window>,
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub size: winit::dpi::PhysicalSize<u32>,
    /// Central cache for managing mesh assets and GPU resources.
    pub mesh_cache: MeshCache,
}

impl GraphicsContext {
    /// Create a new graphics context from a window and display handle.
    pub async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> GraphicsContext {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let context = GraphicsContext {
            window: window.clone(),
            instance,
            device: device.clone(),
            queue,
            surface,
            surface_format,
            size,
            mesh_cache: MeshCache::new(&device),
        };

        // Configure surface for the first time
        context.configure_surface();

        context
    }

    /// Reconfigure the surface with current size and format.
    pub fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            // Request compatibility with the sRGB-format texture view
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    /// Handle window resize.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.configure_surface();
    }

    /// Try to acquire the current surface texture for rendering.
    /// Returns Some(texture) on success, None if surface is unavailable.
    /// Handles Suboptimal, Outdated, and Lost cases by reconfiguring or recreating.
    pub fn get_current_texture(&mut self) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => Some(texture),
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => None,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                None
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                None
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                let window = self.window.clone();
                self.surface = self.instance.create_surface(window).unwrap();
                self.configure_surface();
                None
            }
        }
    }

    /// Create a texture view from a surface texture using the context's surface format.
    pub fn create_texture_view(&self, surface_texture: &wgpu::SurfaceTexture) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            })
    }

    /// Request a redraw of the window.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Notify the window before presenting.
    pub fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }
}

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
    /// Current surface texture (optional, for rendering)
    surface_texture: Option<wgpu::SurfaceTexture>,
}

impl<'a> RenderContext<'a> {
    /// Create a new render context.
    ///
    /// # Arguments
    ///
    /// * `device` - Reference to the GPU infrastructure
    /// * `state` - Mutable reference to the application state
    /// * `surface_texture` - Optional surface texture for the current frame
    pub fn new(
        device: &'a GraphicsDevice,
        state: &'a mut AppState,
        surface_texture: Option<wgpu::SurfaceTexture>,
    ) -> Self {
        Self {
            device,
            state,
            surface_texture,
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

    /// Take the current surface texture, leaving None in its place.
    /// This is used when the texture needs to be presented after rendering.
    pub fn take_surface_texture(&mut self) -> Option<wgpu::SurfaceTexture> {
        self.surface_texture.take()
    }

    /// Get the current surface texture.
    pub fn surface_texture(&self) -> Option<&wgpu::SurfaceTexture> {
        self.surface_texture.as_ref()
    }

    /// Get a texture view from the current surface texture.
    ///
    /// Returns None if no surface texture is available.
    pub fn get_texture_view(&self) -> Option<wgpu::TextureView> {
        self.surface_texture.as_ref().map(|texture| {
            texture.texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.device.surface_config.format.add_srgb_suffix()),
                ..Default::default()
            })
        })
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
