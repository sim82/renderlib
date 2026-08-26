//! Self-contained triangle demo binary using the new architecture.
//!
//! This example demonstrates a rotating triangle using the new renderlib framework
//! with clean separation between GPU infrastructure and application state.
//! Press R to reload shaders.

use std::time::Instant;

use cgmath::SquareMatrix;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{AppRenderer, Application};
use renderlib::context::RenderContext;
use renderlib::device_helpers::*;
use renderlib::geometry::{primitives, PosColorVertex};

/// Path to the shader file.
const SHADER_PATH: &str = "src/shaders/triangle.wgsl";

/// Uniform data containing the rotation matrix.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub rotation: [[f32; 4]; 4],
}

/// Renderer for the rotating triangle demo using new architecture.
pub struct TriangleRenderer {
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    should_reload: bool,
    start_time: Instant,
}

impl TriangleRenderer {
    /// Creates the render pipeline from the shader file.
    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, String> {
        use renderlib::device_helpers::{
            create_pipeline_layout, create_shader_module, RenderPipelineBuilder,
        };

        let shader_src = load_shader_source(SHADER_PATH)?;
        let shader_module = create_shader_module(device, Some("Triangle Shader"), &shader_src);
        let pipeline_layout = create_pipeline_layout(
            device,
            Some("Triangle Pipeline Layout"),
            &[Some(bind_group_layout)],
        );

        Ok(RenderPipelineBuilder::new(device)
            .with_label(Some("Triangle Pipeline"))
            .with_layout(Some(&pipeline_layout))
            .with_shader_module(&shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(&[Some(PosColorVertex::desc())])
            .with_color_formats(&[surface_format])
            .build())
    }

    /// Reloads the shader.
    fn reload_shader(&mut self, device: &wgpu::Device) -> Result<(), String> {
        self.render_pipeline =
            Self::create_pipeline(device, &self.bind_group_layout, self.surface_format)?;
        self.should_reload = false;
        Ok(())
    }
}

impl AppRenderer for TriangleRenderer {
    async fn init(mut context: RenderContext<'_>) -> Self {
        let device = context.wgpu_device();
        let surface_format = context.surface_format();

        // Create vertex buffer from framework primitive
        let vertex_buffer = create_buffer_from_slice(
            device,
            Some("Vertex Buffer"),
            primitives::triangle_vertices(),
            wgpu::BufferUsages::VERTEX,
        );

        // Create uniform buffer from single struct
        let uniform_init = Uniforms {
            rotation: cgmath::Matrix4::<f32>::identity().into(),
        };
        let uniform_buffer = create_buffer(
            device,
            Some("Uniform Buffer"),
            &uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create bind group layout and bind group using helpers
        let bind_group_layout = create_uniform_bind_group_layout(
            device,
            Some("Uniform Bind Group Layout"),
            wgpu::ShaderStages::VERTEX,
        );

        let uniform_bind_group = create_uniform_bind_group(
            device,
            Some("Uniform Bind Group"),
            &bind_group_layout,
            &uniform_buffer,
        );

        // Create initial pipeline
        let render_pipeline = Self::create_pipeline(device, &bind_group_layout, surface_format)
            .expect("Failed to create initial pipeline");

        TriangleRenderer {
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group,
            render_pipeline,
            bind_group_layout,
            surface_format,
            should_reload: false,
            start_time: Instant::now(),
        }
    }

    fn render(&mut self, mut context: RenderContext<'_>) {
        // Reload shader if requested
        if self.should_reload {
            if let Err(e) = self.reload_shader(context.wgpu_device()) {
                eprintln!("Shader reload failed: {}", e);
            }
        }

        // Calculate rotation based on elapsed time
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let rotation_matrix = cgmath::Matrix4::from_angle_z(cgmath::Rad(elapsed));

        // Update uniform buffer with new rotation matrix
        context.wgpu_queue().write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms {
                rotation: rotation_matrix.into(),
            }]),
        );

        // Get texture view from context
        let texture_view = match context.get_texture_view() {
            Some(view) => view,
            None => return,
        };

        // Clear the screen and draw the triangle
        let mut encoder = context
            .wgpu_device()
            .create_command_encoder(&Default::default());

        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // Draw the triangle
        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.draw(0..3, 0..1);

        // End the renderpass
        drop(renderpass);

        // Submit for rendering
        context.wgpu_queue().submit([encoder.finish()]);
    }

    fn resize(&mut self, context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>) {
        // Update surface format if needed
        self.surface_format = context.surface_format();
        // Recreate pipeline with new format
        if let Err(e) = self.reload_shader(context.wgpu_device()) {
            eprintln!("Pipeline recreation failed on resize: {}", e);
        }
    }

    fn input(&mut self, _context: RenderContext<'_>, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
        {
            // Check for R key (case-insensitive) - set flag for reload in render
            if let Key::Character(c) = &key_event.logical_key {
                if c.to_ascii_lowercase() == "r" && key_event.state.is_pressed() {
                    self.should_reload = true;
                }
            }
        }
    }
}

fn main() {
    // Initialize logger
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // Use Poll control flow for games that want to render as fast as possible
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // Use the Application struct with improved architecture
    let mut app = Application::<TriangleRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
