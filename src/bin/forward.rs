//! Forward rendering demo with mesh loading.
//!
//! This demo demonstrates:
//! - Loading a GLTF mesh from disk (defaults to assets/duck.glb)
//! - Falls back to a built-in cube if GLTF file is not found
//! - Extracting vertex positions and normals
//! - Rendering with perspective projection and lighting
//! - Smooth rotation of the mesh
//! - Press R to reload shaders

use std::time::Instant;

use cgmath::{Matrix4, Rad, SquareMatrix, Vector3};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{App, AppRenderer};
use renderlib::camera::{Camera, GeometryUniform, LightingUniform};
use renderlib::context::GraphicsContext;
use renderlib::device_helpers::*;
use renderlib::geometry::PosColorNormalVertex;
use renderlib::mesh::load_gltf;

/// Path to the shader file.
const SHADER_PATH: &str = "src/shaders/forward.wgsl";

/// Default mesh file path.
/// The model should be a GLTF 2.0 (.gltf/.glb) file with at least POSITION attributes.
/// NORMAL attributes are optional (will use defaults if missing).
/// If the file doesn't exist, a built-in cube will be used.
const DEFAULT_MESH_PATH: &str = "assets/duck.glb";

/// Renderer for the forward rendering demo with lighting.
pub struct ForwardRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    geometry_uniform_buffer: wgpu::Buffer,
    lighting_uniform_buffer: wgpu::Buffer,
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

impl ForwardRenderer {
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

        let pipeline = RenderPipelineBuilder::new(device)
            .with_label(Some("GLTF Render Pipeline"))
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
            .with_depth_stencil(Some(depth_stencil))
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

impl AppRenderer for ForwardRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Load mesh using framework, or fall back to cube if file doesn't exist
        let (vertices, indices, model_scale, mesh_center) = match load_gltf(DEFAULT_MESH_PATH) {
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
                eprintln!("Failed to load mesh: {}", e);
                eprintln!("Falling back to hardcoded cube.");
                eprintln!(
                    "To use a custom GLTF/GLB file, place it at '{}'",
                    DEFAULT_MESH_PATH
                );
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

        let camera = Camera::new();
        let aspect = context.size.width as f32 / context.size.height as f32;

        // Create geometry uniform buffer for MVP and model matrices
        let geometry_uniform_buffer = create_buffer(
            device,
            Some("Geometry Uniform Buffer"),
            &GeometryUniform::new(&camera, Matrix4::identity(), aspect),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create lighting uniform buffer for view position and light position
        let lighting_uniform_buffer = create_buffer(
            device,
            Some("Lighting Uniform Buffer"),
            &LightingUniform::new(&camera, [2.0, 3.0, 4.0]),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create bind group layout with two entries: geometry (vertex) and lighting (fragment)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                // Geometry uniforms at binding 0 (vertex stage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Lighting uniforms at binding 1 (fragment stage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: geometry_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lighting_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create initial pipeline
        let render_pipeline =
            Self::create_pipeline(device, &bind_group_layout, context.surface_format)
                .expect("Failed to create initial pipeline");

        ForwardRenderer {
            vertex_buffer,
            index_buffer,
            num_indices,
            geometry_uniform_buffer,
            lighting_uniform_buffer,
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
            camera,
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Reload shader if requested
        if self.should_reload {
            eprintln!("Reloading forward shader...");
            if let Err(e) = self.reload_shader(&context.device) {
                eprintln!("Shader reload failed: {}", e);
            } else {
                eprintln!("Forward shader reloaded successfully!");
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

        // Update geometry uniform buffer
        let aspect = context.size.width as f32 / context.size.height as f32;
        let geometry_uniform = GeometryUniform::new(&self.camera, model, aspect);
        context.queue.write_buffer(
            &self.geometry_uniform_buffer,
            0,
            bytemuck::cast_slice(&[geometry_uniform]),
        );

        // Update lighting uniform buffer
        let lighting_uniform = LightingUniform::new(&self.camera, [2.0, 3.0, 4.0]);
        context.queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting_uniform]),
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

    let mut app = App::<ForwardRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
