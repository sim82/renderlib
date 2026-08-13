//! Advanced demo: 3D spinning cube with colored sides, perspective projection, and fake lighting.
//!
//! This example demonstrates a 3D cube rendering with:
//! - Perspective projection
//! - Model-view-projection matrix
//! - Colored faces
//! - Simple fake lighting (pseudo light source at camera position)
//! - Smooth rotation on multiple axes
//!
//! Press R to reload shaders.

use std::time::Instant;

use cgmath::{Matrix4, Rad, SquareMatrix};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{App, AppRenderer};
use renderlib::camera::Camera;
use renderlib::context::GraphicsContext;
use renderlib::device_helpers::*;
use renderlib::geometry::{primitives, PosColorNormalVertex};

/// Path to the shader file.
const SHADER_PATH: &str = "src/shaders/cube.wgsl";

/// Uniform data containing the model-view-projection matrix, model matrix, and light position.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub light_pos: [f32; 3],
    pub _padding: f32,
}

/// Renderer for the 3D spinning cube demo with lighting.
pub struct CubeRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    should_reload: bool,
    start_time: Instant,
    camera: Camera,
}

impl CubeRenderer {
    /// Creates the render pipeline from the shader file.
    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_src = std::fs::read_to_string(SHADER_PATH)
            .map_err(|e| format!("Failed to read shader file: {}", e))?;

        let shader_module = create_shader_module(device, Some("Cube Shader"), &shader_src);

        let render_pipeline_layout = create_pipeline_layout(
            device,
            Some("Cube Render Pipeline Layout"),
            &[Some(bind_group_layout)],
        );

        let pipeline = RenderPipelineBuilder::new(device)
            .with_label(Some("Cube Render Pipeline"))
            .with_layout(Some(&render_pipeline_layout))
            .with_shader_module(&shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(&[Some(PosColorNormalVertex::desc())])
            .with_color_format(surface_format.add_srgb_suffix())
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            })
            .build();

        Ok(pipeline)
    }

    /// Reloads the shader from disk and recreates the pipeline.
    pub fn reload_shader(&mut self, device: &wgpu::Device) -> Result<(), String> {
        self.render_pipeline =
            Self::create_pipeline(device, &self.bind_group_layout, self.surface_format)?;
        self.should_reload = false;
        Ok(())
    }
}

impl AppRenderer for CubeRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Get cube vertices and indices from framework primitives
        let (vertices, indices) = primitives::cube_vertices();

        // Create vertex buffer
        let vertex_buffer = create_buffer_from_slice(
            device,
            Some("Cube Vertex Buffer"),
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );

        // Create index buffer
        let index_buffer = create_buffer_from_slice(
            device,
            Some("Cube Index Buffer"),
            &indices,
            wgpu::BufferUsages::INDEX,
        );

        // Create uniform buffer for MVP, model matrix, and light position
        let uniform_init = Uniforms {
            mvp: Matrix4::<f32>::identity().into(),
            model: Matrix4::<f32>::identity().into(),
            light_pos: [0.0, 0.0, 5.0], // Light at camera position
            _padding: 0.0,
        };
        let uniform_buffer = create_buffer(
            device,
            Some("MVP Uniform Buffer"),
            &uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create bind group layout - needs to be accessible from both vertex and fragment shaders
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create initial pipeline
        let render_pipeline =
            Self::create_pipeline(device, &bind_group_layout, context.surface_format)
                .expect("Failed to create initial pipeline");

        CubeRenderer {
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            render_pipeline,
            bind_group_layout,
            surface_format: context.surface_format,
            should_reload: false,
            start_time: Instant::now(),
            camera: Camera::new(),
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Reload shader if requested
        if self.should_reload {
            eprintln!("Reloading cube shader...");
            if let Err(e) = self.reload_shader(&context.device) {
                eprintln!("Shader reload failed: {}", e);
            } else {
                eprintln!("Cube shader reloaded successfully!");
            }
        }

        // Calculate MVP and model matrices
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Create model matrix with rotation
        let model =
            Matrix4::from_angle_y(Rad(elapsed * 0.5)) * Matrix4::from_angle_x(Rad(elapsed * 0.3));

        // Get view and projection matrices from camera
        let aspect = context.size.width as f32 / context.size.height as f32;
        let view = self.camera.get_view_matrix();
        let proj = self.camera.get_projection_matrix(aspect);

        // MVP = projection * view * model
        let mvp = proj * view * model;

        // Light position (at camera)
        let camera_pos = self.camera.get_position();
        let light_pos = [camera_pos.x, camera_pos.y, camera_pos.z];

        // Update uniform buffer
        context.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms {
                mvp: mvp.into(),
                model: model.into(),
                light_pos,
                _padding: 0.0,
            }]),
        );

        // Get current surface texture
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let texture_view = context.create_texture_view(&surface_texture);

        // Clear the screen and draw the cube
        let mut encoder = context.device.create_command_encoder(&Default::default());

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

        // Draw the cube
        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        renderpass.draw_indexed(0..36, 0, 0..1);

        // End the renderpass
        drop(renderpass);

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
    }

    fn resize(&mut self, _context: &mut GraphicsContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        // Resize is handled by the aspect ratio in the projection matrix
    }

    fn input(&mut self, event: &WindowEvent) {
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

    // Use Poll control flow for smooth animation
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::<CubeRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
