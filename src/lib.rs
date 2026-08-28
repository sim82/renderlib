//! Renderlib - A wgpu/winit framework for graphics applications.
//!
//! Renderlib provides a foundation for building graphics applications with minimal boilerplate
//! while maintaining full access to the underlying WebGPU API.
//!
//! # Architecture
//!
//! The framework is organized into several layers:
//!
//! - **Application Layer**: [`app`] module with [`Application`] and [`AppRenderer`] trait
//! - **Framework Layer**: [`context`] module with [`RenderContext`] for resource access
//! - **Infrastructure Layer**: [`device`] and [`state`] modules for GPU resources and application state
//! - **Core Systems Layer**: [`camera`], [`geometry`], [`mesh`] for rendering components
//! - **Utilities Layer**: [`device_helpers`], [`input`], [`player`] for common functionality
//!
//! # Quick Start
//!
//! Create a renderer that implements [`AppRenderer`]:
//!
//! ```no_run
//! use renderlib::app::{AppRenderer, Application};
//! use renderlib::context::RenderContext;
//!
//! struct MyRenderer {
//!     render_pipeline: wgpu::RenderPipeline,
//!     vertex_buffer: wgpu::Buffer,
//! }
//!
//! impl AppRenderer for MyRenderer {
//!     async fn init(mut context: RenderContext<'_>) -> Self {
//!         let device = context.wgpu_device();
//!         // Create your resources here
//!         # panic!("example not complete");
//!     }
//!
//!     fn render(&mut self, mut context: RenderContext<'_>) {
//!         let texture_view = context.get_texture_view().unwrap();
//!         // Render here
//!     }
//!
//!     fn resize(&mut self, _context: RenderContext<'_>, _size: winit::dpi::PhysicalSize<u32>) {}
//!     fn input(&mut self, _context: RenderContext<'_>, _event: &winit::event::WindowEvent) {}
//! }
//!
//! fn main() {
//!     let event_loop = winit::event_loop::EventLoop::new().unwrap();
//!     let mut app = Application::<MyRenderer>::new();
//!     event_loop.run_app(&mut app).unwrap();
//! }
//! ```
//!
//! # Features
//!
//! - **Application Framework**: Event loop, window management, renderer trait
//! - **GPU Infrastructure**: Device, queue, surface management with thread safety
//! - **Resource Management**: Mesh loading, caching, and GPU buffer management
//! - **Rendering Techniques**: Forward and deferred rendering support
//! - **Camera System**: View, projection, transforms with orbit controls
//! - **Input Handling**: Keyboard, mouse, and camera control
//! - **Hot Reloading**: Live shader reloading during development
//!
//! # Examples
//!
//! Run examples with:
//!
//! ```bash
//! cargo run --bin triangle      # Simple rotating triangle
//! cargo run --bin forward       # Forward rendering with lighting
//! cargo run --bin deferred      # Deferred rendering with G-buffer
//! ```
//!
//! For more examples and documentation, see the `docs/` directory.

pub mod app;
pub mod camera;
pub mod context;
pub mod deferred;
pub mod device;
pub mod device_helpers;
pub mod geometry;
pub mod input;
pub mod mesh;
pub mod player;
pub mod state;
