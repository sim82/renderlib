//! GLTF mesh loading demo: Loads and renders a GLTF mesh instead of a cube.
//!
//! This demo demonstrates:
//! - Loading a GLTF mesh from disk using the framework mesh module
//! - Extracting vertex positions and normals
//! - Rendering with perspective projection and lighting
//! - Smooth rotation of the loaded mesh
//! - Press R to reload shaders

use std::time::Instant;

use cgmath::{Matrix4, Rad, SquareMatrix, Vector3};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{App, AppRenderer};
use renderlib::camera::Camera;
use renderlib::context::GraphicsContext;
use renderlib::device_helpers::*;
use renderlib::geometry::PosColorNormalVertex;
use renderlib::mesh::load_gltf;

/// Path to the shader file (reuse cube shader).
const SHADER_PATH: &str = "src/shaders/cube.wgsl";

/// Default GLTF file path.
/// The model should be a GLTF 2.0 (.gltf/.glb) file with at least POSITION attributes.
/// NORMAL attributes are optional (will use defaults if missing).
const GLTF_PATH: &str = "assets/duck.glb";

/// Uniform data containing the model-view-projection matrix, model matrix, and light position.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub light_pos: [f32; 3],
    pub _padding: f32,
}

/// Renderer for the GLTF mesh demo with lighting.
pub struct GltfRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    should_reload: bool,
    start_time: Instant,
    /// Scale factor to normalize model size (computed from bounding box).
    model_scale: f32,
    /// Translation to center the mesh at origin (computed from bounding box center).
    mesh_center: Vector3<f32>,
    camera: Camera,
}

impl GltfRenderer {
    /// Creates the render pipeline from the shader file with depth testing enabled.
    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_src = load_shader_source(SHADER_PATH)?;

        let shader_module = create_shader_module(device, Some("GLTF Shader"), &shader_src);

        let render_pipeline_layout = create_pipeline_layout(
            device,
            Some("GLTF Render Pipeline Layout"),
            &[Some(bind_group_layout)],
        );

        // Enable depth testing
        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GLTF Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[Some(PosColorNormalVertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format.add_srgb_suffix(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

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

impl AppRenderer for GltfRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Load GLTF mesh using framework, or fall back to cube if file doesn't exist
        let (vertices, indices, model_scale, mesh_center) = match load_gltf(GLTF_PATH) {
            Ok(mesh) => {
                eprintln!(
                    "Loaded GLTF mesh: {} vertices, {} indices, scale: {:.2}, center: ({:.2}, {:.2}, {:.2})",
                    mesh.vertices.len(),
                    mesh.indices.len(),
                    mesh.scale,
                    mesh.center.x,
                    mesh.center.y,
                    mesh.center.z
                );
                (mesh.vertices, mesh.indices, mesh.scale, mesh.center)
            }
            Err(e) => {
                eprintln!("Failed to load GLTF mesh: {}", e);
                eprintln!("Falling back to hardcoded cube.");
                eprintln!("To use a custom GLTF/GLB file, place it at '{}'", GLTF_PATH);
                // Fall back to the hardcoded cube from primitives
                use renderlib::geometry::primitives;
                let (v, i) = primitives::cube_vertices();
                (v, i, 1.0, Vector3::new(0.0, 0.0, 0.0)) // Cube is already centered
            }
        };
        let num_indices = indices.len() as u32;

        // Create vertex buffer
        let vertex_buffer = create_buffer_from_slice(
            device,
            Some("GLTF Vertex Buffer"),
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );

        // Create index buffer
        let index_buffer = create_buffer_from_slice(
            device,
            Some("GLTF Index Buffer"),
            &indices,
            wgpu::BufferUsages::INDEX,
        );

        // Create depth texture for depth testing using framework helper
        let (depth_texture, depth_texture_view) = create_depth_texture(
            device,
            context.size.width,
            context.size.height,
            Some("GLTF Depth Texture"),
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

        // Create bind group layout
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

        GltfRenderer {
            vertex_buffer,
            index_buffer,
            num_indices,
            uniform_buffer,
            uniform_bind_group,
            render_pipeline,
            bind_group_layout,
            surface_format: context.surface_format,
            depth_texture,
            depth_texture_view,
            should_reload: false,
            start_time: Instant::now(),
            model_scale,
            mesh_center,
            camera: Camera::new(),
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Reload shader if requested
        if self.should_reload {
            eprintln!("Reloading GLTF shader...");
            if let Err(e) = self.reload_shader(&context.device) {
                eprintln!("Shader reload failed: {}", e);
            } else {
                eprintln!("GLTF shader reloaded successfully!");
            }
        }

        // Calculate MVP and model matrices
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Create model matrix with translation, scaling, and rotation
        // In column-major matrices (cgmath), transformations are applied right-to-left:
        // M = R * S * T means vertex is transformed as M * v = R * S * T * v
        // So T (translation) is applied first, then S (scale), then R (rotation)
        let translation = Matrix4::from_translation(-self.mesh_center);
        let scale_matrix = Matrix4::from_scale(self.model_scale);
        let model = Matrix4::from_angle_y(Rad(elapsed * 0.5))
            * Matrix4::from_angle_x(Rad(elapsed * 0.3))
            * scale_matrix
            * translation;

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

        // Clear the screen and draw the mesh
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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // Draw the mesh
        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        renderpass.draw_indexed(0..self.num_indices, 0, 0..1);

        // End the renderpass
        drop(renderpass);

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
    }

    fn resize(&mut self, context: &mut GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>) {
        // Recreate depth texture with new size using framework helper
        let (depth_texture, depth_texture_view) = create_depth_texture(
            &context.device,
            new_size.width,
            new_size.height,
            Some("GLTF Depth Texture"),
        );
        self.depth_texture = depth_texture;
        self.depth_texture_view = depth_texture_view;
    }

    fn input(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
        {
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

    let mut app = App::<GltfRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
