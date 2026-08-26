//! Deferred rendering demo with mesh loading using the new architecture.
//!
//! Combines mesh loading with deferred shading pipeline:
//! - Loads a GLTF mesh from disk (defaults to assets/duck.glb)
//! - Falls back to a built-in cube if GLTF file is not found
//! - Geometry pass: renders mesh to G-buffer (position, normal, albedo)
//! - Lighting pass: full-screen quad that reads G-buffer and computes lighting
//! - Auto-scales and centers the mesh
//! - Press R to reload shaders
//!
//! Uses the new renderlib framework with clean separation between
//! GPU infrastructure and application state.

use std::time::Instant;

use cgmath::{Matrix4, Rad, SquareMatrix, Vector3};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{AppRenderer, Application};
use renderlib::camera::{Camera, GeometryUniform, Light, LightingUniform};
use renderlib::context::RenderContext;
use renderlib::deferred::GBuffer;
use renderlib::device_helpers::*;
use renderlib::geometry::PosColorNormalVertex;
use renderlib::mesh::{quad_vertices_2d, MeshHandle, MeshSource, QuadVertex};

/// Paths to the shader files.
const GEOMETRY_SHADER_PATH: &str = "src/shaders/deferred_geometry.wgsl";
const LIGHTING_SHADER_PATH: &str = "src/shaders/deferred_lighting.wgsl";

/// Default mesh file path.
/// The model should be a GLTF 2.0 (.gltf/.glb) file with at least POSITION attributes.
/// If the file doesn't exist, a built-in cube will be used.
const DEFAULT_MESH_PATH: &str = "assets/duck.glb";

/// Renderer for deferred rendering demo using new architecture.
pub struct DeferredRenderer {
    // GLTF mesh handle
    mesh_handle: MeshHandle,
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

    // Lighting
    lights: [Light; renderlib::camera::MAX_LIGHTS],
    num_lights: u32,
}

impl DeferredRenderer {
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
            .with_color_formats(&[surface_format.add_srgb_suffix()])
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

impl AppRenderer for DeferredRenderer {
    async fn init(mut context: RenderContext<'_>) -> Self {
        let size = context.size();
        let aspect = size.width as f32 / size.height as f32;

        // Use camera from app state as starting point
        let camera = context.state().camera.clone();

        // Load shaders
        let geometry_shader_src =
            load_shader_source(GEOMETRY_SHADER_PATH).expect("Failed to load geometry shader");
        let lighting_shader_src =
            load_shader_source(LIGHTING_SHADER_PATH).expect("Failed to load lighting shader");

        // Load mesh using the new mesh cache via context state
        let mesh_source = MeshSource::Path(DEFAULT_MESH_PATH.to_string());
        let mesh_handle = context
            .state()
            .mesh_cache
            .load_mut(&mesh_source)
            .expect("Failed to load mesh");

        // Store mesh handle in app state
        context.state().set_active_mesh(mesh_handle);

        // Get both mesh asset and resource using immutable access
        let (mesh_asset, mesh_resource) = context
            .state()
            .mesh_cache
            .get_both(mesh_handle)
            .expect("Failed to get mesh data");

        let model_scale = mesh_asset.scale;
        let mesh_center = mesh_asset.center;
        let num_indices = mesh_resource.num_indices;

        // Log which mesh was loaded
        if model_scale != 1.0 || mesh_center != Vector3::new(0.0, 0.0, 0.0) {
            eprintln!(
                "Loaded GLTF mesh: {} vertices, {} indices, scale: {:.2}, center: ({:.2}, {:.2}, {:.2})",
                mesh_asset.vertices.len(),
                mesh_asset.indices.len(),
                model_scale,
                mesh_center.x,
                mesh_center.y,
                mesh_center.z
            );
        }

        let device = context.wgpu_device();

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
        let geometry_uniform_init = GeometryUniform::new(&camera, Matrix4::identity(), aspect);
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
            context.surface_format(),
            &geometry_shader_src,
        )
        .expect("Failed to create geometry pipeline");

        // Create multiple lights for the scene
        let mut lights: [Light; renderlib::camera::MAX_LIGHTS] =
            [Light::default(); renderlib::camera::MAX_LIGHTS];
        lights[0] = Light::new([2.0, 3.0, 4.0], [1.0, 1.0, 1.0]); // White light above and to the right
        lights[1] = Light::new([-3.0, 2.0, 2.0], [1.0, 0.0, 0.0]); // Red light to the left
        lights[2] = Light::new([0.0, -2.0, 3.0], [0.0, 0.0, 1.0]); // Blue light below
        lights[3] = Light::new([0.0, 2.0, -3.0], [0.0, 1.0, 0.0]); // Green light behind
        let num_lights = 4u32;

        // Create lighting pass uniform buffer
        let lighting_uniform_init =
            LightingUniform::new_with_lights(&camera, &lights[..num_lights as usize]);
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
            context.surface_format(),
            &lighting_shader_src,
        )
        .expect("Failed to create lighting pipeline");

        DeferredRenderer {
            mesh_handle,
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
            surface_format: context.surface_format(),
            model_scale,
            mesh_center,
            should_reload_geometry: false,
            should_reload_lighting: false,
            start_time: Instant::now(),
            camera,
            lights,
            num_lights,
        }
    }

    fn render(&mut self, mut context: RenderContext<'_>) {
        // Reload shaders if requested
        if self.should_reload_geometry {
            eprintln!("Reloading deferred geometry shader...");
            if let Err(e) = self.reload_geometry_shader(context.wgpu_device()) {
                eprintln!("Geometry shader reload failed: {}", e);
            } else {
                eprintln!("Deferred geometry shader reloaded successfully!");
            }
        }

        if self.should_reload_lighting {
            eprintln!("Reloading deferred lighting shader...");
            if let Err(e) = self.reload_lighting_shader(context.wgpu_device()) {
                eprintln!("Lighting shader reload failed: {}", e);
            } else {
                eprintln!("Deferred lighting shader reloaded successfully!");
            }
        }

        // Calculate MVP and model matrices
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Create model matrix with translation, scaling, and rotation
        let translation = Matrix4::from_translation(-self.mesh_center);
        let scale_matrix = Matrix4::from_scale(self.model_scale);
        let model = Matrix4::from_angle_y(Rad(elapsed * 0.5))
            * Matrix4::from_angle_x(Rad(elapsed * 0.3))
            * scale_matrix
            * translation;

        // Update geometry uniform buffer
        let size = context.size();
        let aspect = size.width as f32 / size.height as f32;
        let geometry_uniform = GeometryUniform::new(&self.camera, model, aspect);
        context.wgpu_queue().write_buffer(
            &self.geometry_uniform_buffer,
            0,
            bytemuck::cast_slice(&[geometry_uniform]),
        );

        // Update lighting uniform buffer with all lights
        let lighting_uniform = LightingUniform::new_with_lights(
            &self.camera,
            &self.lights[..self.num_lights as usize],
        );
        context.wgpu_queue().write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting_uniform]),
        );

        // Get the mesh resource from the cache first (to avoid borrow conflicts)
        let (_mesh_asset, mesh_resource) = context
            .state()
            .mesh_cache
            .get_both(self.mesh_handle)
            .expect("Failed to get mesh data");

        // Get current texture view from context
        let texture_view = match context.get_texture_view() {
            Some(view) => view,
            None => return,
        };

        // ===== GEOMETRY PASS =====
        // Render mesh to G-buffer
        let mut encoder = context
            .wgpu_device()
            .create_command_encoder(&Default::default());

        // Geometry pass render target
        let geometry_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Deferred Geometry Pass"),
            color_attachments: &[
                // Position target
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.gbuffer.position_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                // Normal target
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.gbuffer.normal_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                // Albedo target
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.gbuffer.albedo_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
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
        };

        let mut geometry_pass = encoder.begin_render_pass(&geometry_pass_desc);

        // Draw the mesh to G-buffer
        geometry_pass.set_pipeline(&self.geometry_pipeline);
        geometry_pass.set_bind_group(0, &self.geometry_bind_group, &[]);
        geometry_pass.set_vertex_buffer(0, mesh_resource.vertex_buffer.slice(..));
        geometry_pass.set_index_buffer(
            mesh_resource.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        geometry_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        drop(geometry_pass);

        // ===== LIGHTING PASS =====
        // Full-screen quad that reads G-buffer and computes lighting
        let lighting_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Deferred Lighting Pass"),
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
        };

        let mut lighting_pass = encoder.begin_render_pass(&lighting_pass_desc);

        // Create G-buffer bind group for lighting pass
        let gbuffer_bind_group =
            context
                .wgpu_device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("GBuffer Bind Group"),
                    layout: &self.gbuffer.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &self.gbuffer.position_view,
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&self.gbuffer.normal_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&self.gbuffer.albedo_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&self.gbuffer.sampler),
                        },
                    ],
                });

        // Draw full-screen quad
        lighting_pass.set_pipeline(&self.lighting_pipeline);
        lighting_pass.set_bind_group(0, &gbuffer_bind_group, &[]);
        lighting_pass.set_bind_group(1, &self.lighting_uniform_bind_group, &[]);
        lighting_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        lighting_pass.draw(0..6, 0..1);

        drop(lighting_pass);

        // Submit for rendering (presentation handled by framework)
        context.wgpu_queue().submit([encoder.finish()]);
    }

    fn resize(&mut self, context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>) {
        // Update surface format if needed
        self.surface_format = context.surface_format();

        // Recreate G-buffer with new size
        self.gbuffer = GBuffer::new(
            context.wgpu_device(),
            new_size.width,
            new_size.height,
            Some("Deferred"),
        );

        // Recreate depth texture with new size using framework helper
        let (depth_texture, depth_texture_view) = create_depth_texture(
            context.wgpu_device(),
            new_size.width,
            new_size.height,
            Some("GLTF Deferred Depth Texture"),
        );
        self.depth_texture = depth_texture;
        self.depth_texture_view = depth_texture_view;
    }

    fn input(&mut self, _context: RenderContext<'_>, event: &WindowEvent) {
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

    // Use the Application struct with improved architecture
    let mut app = Application::<DeferredRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
