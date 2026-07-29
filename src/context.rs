//! Graphics context module - manages wgpu device, surface, and swap chain setup.

use std::sync::Arc;

use winit::{event_loop::OwnedDisplayHandle, window::Window};

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
            device,
            queue,
            surface,
            surface_format,
            size,
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

    /// Get a reference to the window.
    pub fn window(&self) -> &Window {
        &self.window
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
}
