//! Deferred rendering demo with GLTF mesh loading.
//!
//! Combines GLTF mesh loading with deferred shading pipeline:
//! - Loads a GLTF mesh from disk
//! - Geometry pass: renders mesh to G-buffer (position, normal, albedo)
//! - Lighting pass: full-screen quad that reads G-buffer and computes lighting
//! - Auto-scales and centers the mesh
//! - Press R to reload shaders

use std::time::Instant;

use cgmath::{Matrix4, Rad, SquareMatrix, Vector3};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{App, AppRenderer};
use renderlib::camera::Camera;
use renderlib::context::GraphicsContext;
use renderlib::deferred::GBuffer;
use renderlib::device_helpers::*;
use renderlib::geometry::PosColorNormalVertex;
use renderlib::mesh::{load_gltf, quad_vertices_2d, QuadVertex};

/// Paths to the shader files.
const GEOMETRY_SHADER_PATH: &str = "src/shaders/cube_deferred_geometry.wgsl";
const LIGHTING_SHADER_PATH: &str = "src/shaders/cube_deferred_lighting.wgsl";

/// Default GLTF file path.
const GLTF_PATH: &str = "assets/duck.glb";

/// Uniform data for the geometry pass (MVP and model matrices).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryUniforms {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
}

/// Uniform data for the lighting pass (camera and light positions).
/// Uses std140 layout: each vec3 occupies 16 bytes, f32 occupies 16 bytes with padding
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniforms {
    pub view_position: [f32; 4],  // vec3 padded to vec4 for 16-byte alignment
    pub light_position: [f32; 4], // vec3 padded to vec4 for 16-byte alignment
}

/// Renderer for deferred rendering demo with GLTF mesh.
pub struct GltfDeferredRenderer {
    // GLTF mesh resources
    mesh_vertex_buffer: wgpu::Buffer,
    mesh_index_buffer: wgpu::Buffer,
    num_indices: u32,

    // Geometry pass resources
    geometry_uniform_buffer: wgpu::Buffer,
    geometry_bind_group_layout: wgpu::BindGroupLayout,
    geometry_bind_group: wgpu::BindGroup,
    geometry_pipeline: wgpu::RenderPipeline,
    geometry_shader_path: String,

    // Lighting pass resources
    quad_vertex_buffer: wgpu::Buffer,
    lighting_uniform_buffer: wgpu::Buffer,
    lighting_uniform_bind_group_layout: wgpu::BindGroupLayout,
    lighting_uniform_bind_group: wgpu::BindGroup,
    lighting_pipeline: wgpu::RenderPipeline,
    lighting_shader_path: String,

    // Depth buffer for geometry pass
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,

    // G-buffer from framework
    gbuffer: GBuffer,

    // Pipeline state
    surface_format: wgpu::TextureFormat,

    // Mesh transforms
    model_scale: f32,
    mesh_center: Vector3<f32>,

    // Hot-reload state
    should_reload_geometry: bool,
    should_reload_lighting: bool,

    // Timing
    start_time: Instant,

    // Camera
    camera: Camera,
}

impl GltfDeferredRenderer {
    /// Create the geometry pass pipeline using the enhanced builder.
    fn create_geometry_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        _surface_format: wgpu::TextureFormat,
        shader_src: &str,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_module =
            create_shader_module(device, Some("Deferred Geometry Shader"), shader_src);

        let pipeline_layout = create_pipeline_layout(
            device,
            Some("Deferred Geometry Pipeline Layout"),
            &[Some(bind_group_layout)],
        );

        // Use the enhanced builder with multi-color attachments for G-buffer
        // Enable depth testing for geometry pass
        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let pipeline = RenderPipelineBuilder::new(device)
            .with_label(Some("Deferred Geometry Pipeline"))
            .with_layout(Some(&pipeline_layout))
            .with_shader_module(&shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(&[Some(PosColorNormalVertex::desc())])
            .with_color_formats(&GBuffer::color_formats())
            .with_blend_states(&[None, None, None])
            .with_depth_stencil(Some(depth_stencil))
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

    /// Create the lighting pass pipeline.
    fn create_lighting_pipeline(
        device: &wgpu::Device,
        gbuffer_bind_group_layout: &wgpu::BindGroupLayout,
        lighting_uniform_bind_group_layout: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
        shader_src: &str,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_module =
            create_shader_module(device, Some("Deferred Lighting Shader"), shader_src);

        let pipeline_layout = create_pipeline_layout(
            device,
            Some("Deferred Lighting Pipeline Layout"),
            &[
                Some(gbuffer_bind_group_layout),
                Some(lighting_uniform_bind_group_layout),
            ],
        );

        let pipeline = RenderPipelineBuilder::new(device)
            .with_label(Some("Deferred Lighting Pipeline"))
            .with_layout(Some(&pipeline_layout))
            .with_shader_module(&shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(&[Some(QuadVertex::desc())])
            .with_color_format(surface_format.add_srgb_suffix())
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            })
            .build();

        Ok(pipeline)
    }

    /// Reload geometry shader.
    fn reload_geometry_shader(&mut self, device: &wgpu::Device) -> Result<(), String> {
        let shader_src = load_shader_source(&self.geometry_shader_path)?;
        self.geometry_pipeline = Self::create_geometry_pipeline(
            device,
            &self.geometry_bind_group_layout,
            self.surface_format,
            &shader_src,
        )?;
        self.should_reload_geometry = false;
        Ok(())
    }

    /// Reload lighting shader.
    fn reload_lighting_shader(&mut self, device: &wgpu::Device) -> Result<(), String> {
        let shader_src = load_shader_source(&self.lighting_shader_path)?;
        self.lighting_pipeline = Self::create_lighting_pipeline(
            device,
            &self.gbuffer.bind_group_layout,
            &self.lighting_uniform_bind_group_layout,
            self.surface_format,
            &shader_src,
        )?;
        self.should_reload_lighting = false;
        Ok(())
    }
}

impl AppRenderer for GltfDeferredRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;
        let size = context.size;

        // Load shaders
        let geometry_shader_src =
            load_shader_source(GEOMETRY_SHADER_PATH).expect("Failed to load geometry shader");
        let lighting_shader_src =
            load_shader_source(LIGHTING_SHADER_PATH).expect("Failed to load lighting shader");

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

        // Create mesh buffers
        let mesh_vertex_buffer = create_buffer_from_slice(
            device,
            Some("GLTF Vertex Buffer"),
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );

        let mesh_index_buffer = create_buffer_from_slice(
            device,
            Some("GLTF Index Buffer"),
            &indices,
            wgpu::BufferUsages::INDEX,
        );

        // Create quad buffer for lighting pass
        let quad_vertex_buffer = create_buffer_from_slice(
            device,
            Some("Quad Vertex Buffer"),
            quad_vertices_2d(),
            wgpu::BufferUsages::VERTEX,
        );

        // Create G-buffer from framework
        let gbuffer = GBuffer::new(device, size.width, size.height, Some("Deferred"));

        // Create depth texture for geometry pass
        let (depth_texture, depth_texture_view) = create_depth_texture(
            device,
            size.width,
            size.height,
            Some("GLTF Deferred Depth Texture"),
        );

        // Create geometry pass uniform buffer
        let geometry_uniform_init = GeometryUniforms {
            mvp: Matrix4::<f32>::identity().into(),
            model: Matrix4::<f32>::identity().into(),
        };
        let geometry_uniform_buffer = create_buffer(
            device,
            Some("Geometry Uniform Buffer"),
            &geometry_uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create geometry bind group layout and bind group using framework helpers
        let geometry_bind_group_layout = create_uniform_bind_group_layout(
            device,
            Some("Geometry Uniform Bind Group Layout"),
            wgpu::ShaderStages::VERTEX,
        );

        let geometry_bind_group = create_uniform_bind_group(
            device,
            Some("Geometry Uniform Bind Group"),
            &geometry_bind_group_layout,
            &geometry_uniform_buffer,
        );

        // Create geometry pipeline
        let geometry_pipeline = Self::create_geometry_pipeline(
            device,
            &geometry_bind_group_layout,
            context.surface_format,
            &geometry_shader_src,
        )
        .expect("Failed to create geometry pipeline");

        // Create lighting pass uniform buffer
        let lighting_uniform_init = LightingUniforms {
            view_position: [0.0, 0.0, 5.0, 0.0],
            light_position: [2.0, 3.0, 4.0, 0.0],
        };
        let lighting_uniform_buffer = create_buffer(
            device,
            Some("Lighting Uniform Buffer"),
            &lighting_uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create lighting uniform bind group layout and bind group using framework helpers
        let lighting_uniform_bind_group_layout = create_uniform_bind_group_layout(
            device,
            Some("Lighting Uniform Bind Group Layout"),
            wgpu::ShaderStages::FRAGMENT,
        );

        let lighting_uniform_bind_group = create_uniform_bind_group(
            device,
            Some("Lighting Uniform Bind Group"),
            &lighting_uniform_bind_group_layout,
            &lighting_uniform_buffer,
        );

        // Create lighting pipeline
        let lighting_pipeline = Self::create_lighting_pipeline(
            device,
            &gbuffer.bind_group_layout,
            &lighting_uniform_bind_group_layout,
            context.surface_format,
            &lighting_shader_src,
        )
        .expect("Failed to create lighting pipeline");

        GltfDeferredRenderer {
            mesh_vertex_buffer,
            mesh_index_buffer,
            num_indices,
            geometry_uniform_buffer,
            geometry_bind_group_layout,
            geometry_bind_group,
            geometry_pipeline,
            geometry_shader_path: GEOMETRY_SHADER_PATH.to_string(),
            quad_vertex_buffer,
            lighting_uniform_buffer,
            lighting_uniform_bind_group_layout,
            lighting_uniform_bind_group,
            lighting_pipeline,
            lighting_shader_path: LIGHTING_SHADER_PATH.to_string(),
            depth_texture,
            depth_texture_view,
            gbuffer,
            surface_format: context.surface_format,
            model_scale,
            mesh_center,
            should_reload_geometry: false,
            should_reload_lighting: false,
            start_time: Instant::now(),
            camera: Camera::new(),
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Handle shader reload
        if self.should_reload_geometry {
            eprintln!("Reloading geometry shader...");
            if let Err(e) = self.reload_geometry_shader(&context.device) {
                eprintln!("Geometry shader reload failed: {}", e);
            } else {
                eprintln!("Geometry shader reloaded successfully!");
            }
        }

        if self.should_reload_lighting {
            eprintln!("Reloading lighting shader...");
            if let Err(e) = self.reload_lighting_shader(&context.device) {
                eprintln!("Lighting shader reload failed: {}", e);
            } else {
                eprintln!("Lighting shader reloaded successfully!");
            }
        }

        // Resize G-buffer and depth texture if needed
        if self.gbuffer.width != context.size.width || self.gbuffer.height != context.size.height {
            self.gbuffer
                .resize(&context.device, context.size.width, context.size.height);

            // Recreate depth texture with new size
            let (depth_texture, depth_texture_view) = create_depth_texture(
                &context.device,
                context.size.width,
                context.size.height,
                Some("GLTF Deferred Depth Texture"),
            );
            self.depth_texture = depth_texture;
            self.depth_texture_view = depth_texture_view;
        }

        // Calculate matrices
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

        // Update geometry uniform buffer
        context.queue.write_buffer(
            &self.geometry_uniform_buffer,
            0,
            bytemuck::cast_slice(&[GeometryUniforms {
                mvp: mvp.into(),
                model: model.into(),
            }]),
        );

        // Update lighting uniform buffer
        let camera_pos = self.camera.get_position();
        context.queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[LightingUniforms {
                view_position: [camera_pos.x, camera_pos.y, camera_pos.z, 0.0],
                light_position: [2.0, 3.0, 4.0, 0.0],
            }]),
        );

        // Get current surface texture
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let surface_view = context.create_texture_view(&surface_texture);

        // Create command encoder
        let mut encoder = context.device.create_command_encoder(&Default::default());

        // Create G-buffer bind group for lighting pass
        let gbuffer_bind_group = self.gbuffer.create_bind_group(&context.device);

        // =====================================================================
        // GEOMETRY PASS: Render mesh to G-buffer
        // =====================================================================
        {
            let mut geometry_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Geometry Pass"),
                color_attachments: &self.gbuffer.color_attachments(),
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

            // Draw mesh
            geometry_pass.set_pipeline(&self.geometry_pipeline);
            geometry_pass.set_bind_group(0, &self.geometry_bind_group, &[]);
            geometry_pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
            geometry_pass
                .set_index_buffer(self.mesh_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            geometry_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // =====================================================================
        // LIGHTING PASS: Full-screen quad that reads G-buffer and computes lighting
        // =====================================================================
        {
            let mut lighting_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Lighting Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
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

            // Draw full-screen quad
            lighting_pass.set_pipeline(&self.lighting_pipeline);
            lighting_pass.set_bind_group(0, &gbuffer_bind_group, &[]);
            lighting_pass.set_bind_group(1, &self.lighting_uniform_bind_group, &[]);
            lighting_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            lighting_pass.draw(0..6, 0..1);
        }

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
    }

    fn resize(&mut self, context: &mut GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>) {
        // Resize G-buffer
        self.gbuffer
            .resize(&context.device, new_size.width, new_size.height);

        // Recreate depth texture with new size
        let (depth_texture, depth_texture_view) = create_depth_texture(
            &context.device,
            new_size.width,
            new_size.height,
            Some("GLTF Deferred Depth Texture"),
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
                    // Reload both shaders
                    self.should_reload_geometry = true;
                    self.should_reload_lighting = true;
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

    let mut app = App::<GltfDeferredRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
