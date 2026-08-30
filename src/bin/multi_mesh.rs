//! Multi-mesh deferred rendering demo with camera controls.
//!
//! Demonstrates mesh instancing with proper resource encapsulation:
//! - Loads a single GLTF mesh from disk (assets/duck.glb)
//! - Creates NUM_MESH_INSTANCES (27) MeshInstance objects arranged in a 3D cubic grid
//! - Falls back to a built-in cube if GLTF file is not found
//! - Geometry pass: renders all mesh instances to G-buffer (position, normal, albedo)
//! - Lighting pass: full-screen quad that reads G-buffer and computes lighting
//! - Auto-scales and centers the mesh, then positions instances on a grid with BASE_SPACING
//! - First-person camera controls with WASD + mouse look
//! - Press R to reload shaders
//!
//! Design note: Each MeshInstance encapsulates its own uniform buffer and bind group,
//! allowing multiple instances of the same mesh to be rendered with different transforms.
//! Positions are generated using an expanding cubic grid for even distribution.
//!
//! Uses the new renderlib framework with clean separation between
//! GPU infrastructure and application state, while maintaining the same
//! input handling with PlayerState and InputController as the original.

use cgmath::{Matrix4, Rad, SquareMatrix, Vector3};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{AppRenderer, Application};
use renderlib::camera::{GeometryUniform, Light, LightingUniform};
use renderlib::context::RenderContext;
use renderlib::deferred::GBuffer;
use renderlib::device_helpers::*;
use renderlib::geometry::PosColorNormalVertex;
use renderlib::input::InputController;
use renderlib::mesh::{quad_vertices_2d, MeshHandle, MeshSource, QuadVertex};
use renderlib::player::PlayerState;

/// Number of mesh instances to create
const NUM_MESH_INSTANCES: usize = 1024 * 10;

/// Base spacing between mesh instances (in world units)
/// Using 3.0 to ensure we can see past them
const BASE_SPACING: f32 = 3.0;

/// Generate positions in an expanding cubic grid
/// Grid expands in all three dimensions as needed
fn generate_expanding_grid_positions(count: usize, spacing: f32) -> Vec<cgmath::Vector3<f32>> {
    let mut positions = Vec::with_capacity(count);

    // Calculate grid dimensions (cube root of count, rounded up)
    let grid_size = ((count as f32).powf(1.0 / 3.0)).ceil() as i32;
    let half_grid = grid_size as f32 / 2.0;

    for i in 0..count {
        // Convert linear index to 3D grid coordinates
        let z = (i / (grid_size * grid_size) as usize) as i32;
        let remainder = i % (grid_size * grid_size) as usize;
        let y = (remainder / grid_size as usize) as i32;
        let x = (remainder % grid_size as usize) as i32;

        // Center the grid and apply spacing
        positions.push(Vector3::new(
            (x as f32 - half_grid) * spacing,
            (y as f32 - half_grid) * spacing,
            (z as f32 - half_grid) * spacing,
        ));
    }

    positions
}

/// Paths to the shader files.
const GEOMETRY_SHADER_PATH: &str = "src/shaders/deferred_geometry.wgsl";
const LIGHTING_SHADER_PATH: &str = "src/shaders/deferred_lighting.wgsl";

/// Default mesh file path.
/// The model should be a GLTF 2.0 (.gltf/.glb) file with at least POSITION attributes.
/// If the file doesn't exist, a built-in cube will be used.
const DEFAULT_MESH_PATH: &str = "assets/duck.glb";

/// Per-mesh instance data encapsulating CPU and GPU resources.
struct MeshInstance {
    /// Handle to the mesh data (shared between instances of the same mesh)
    mesh_handle: MeshHandle,

    /// CPU-side transform data
    scale: f32,
    center: Vector3<f32>,
    position_offset: Vector3<f32>,

    /// GPU-side per-instance resources
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl MeshInstance {
    fn new(
        mesh_handle: MeshHandle,
        scale: f32,
        center: Vector3<f32>,
        position_offset: Vector3<f32>,
        uniform_buffer: wgpu::Buffer,
        bind_group: wgpu::BindGroup,
    ) -> Self {
        Self {
            mesh_handle,
            scale,
            center,
            position_offset,
            uniform_buffer,
            bind_group,
        }
    }
}

/// Renderer for multi-mesh deferred rendering demo with camera controls.
pub struct DeferredRenderer {
    // Mesh instances - each has its own transform and GPU resources
    mesh_instances: Vec<MeshInstance>,

    // Geometry pass resources (shared)
    geometry_bind_group_layout: wgpu::BindGroupLayout,
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

    // Hot-reload state
    should_reload_geometry: bool,
    should_reload_lighting: bool,

    // Player state for movement
    player: PlayerState,

    // Input controller
    input_controller: InputController,

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

        // Clone camera from app state first to avoid borrow conflicts
        let camera = context.state().camera.clone();

        // Load the duck mesh once
        let mesh_source = MeshSource::Path(DEFAULT_MESH_PATH.to_string());
        let mesh_handle = context
            .state()
            .mesh_cache
            .load_mut(&mesh_source)
            .expect("Failed to load duck mesh");

        // Get mesh asset for scale/center
        let (mesh_asset, _mesh_resource) = context
            .state()
            .mesh_cache
            .get_both(mesh_handle)
            .expect("Failed to get mesh data");

        // Store first mesh handle in app state for compatibility
        context.state().set_active_mesh(mesh_handle);

        let device = context.wgpu_device();

        // Generate position offsets in an expanding cubic grid
        let position_offsets = generate_expanding_grid_positions(NUM_MESH_INSTANCES, BASE_SPACING);

        // Create geometry bind group layout (shared for all instances)
        let geometry_bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label(Some("Geometry Uniform Bind Group Layout"))
            .with_uniform_buffer(wgpu::ShaderStages::VERTEX, None)
            .build();

        // Create mesh instances with grid positions
        let mut mesh_instances = Vec::with_capacity(NUM_MESH_INSTANCES);
        for position_offset in position_offsets.iter().take(NUM_MESH_INSTANCES) {
            // Create per-instance uniform buffer
            let geometry_uniform_init = GeometryUniform::new(&camera, Matrix4::identity(), aspect);
            let uniform_buffer = create_buffer(
                device,
                Some("Mesh Instance Uniform Buffer"),
                &geometry_uniform_init,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            );

            // Create per-instance bind group
            let bind_group = create_bind_group_auto(
                device,
                Some("Mesh Instance Bind Group"),
                &geometry_bind_group_layout,
                &[uniform_buffer.as_entire_binding()],
            );

            mesh_instances.push(MeshInstance::new(
                mesh_handle,
                mesh_asset.scale,
                mesh_asset.center,
                *position_offset,
                uniform_buffer,
                bind_group,
            ));
        }

        // Log which mesh was loaded
        eprintln!(
            "Loaded mesh {}: {} vertices, {} indices, scale: {:.2}, center: ({:.2}, {:.2}, {:.2})",
            mesh_asset.name,
            mesh_asset.vertices.len(),
            mesh_asset.indices.len(),
            mesh_asset.scale,
            mesh_asset.center.x,
            mesh_asset.center.y,
            mesh_asset.center.z
        );

        // Load shaders
        let geometry_shader_src =
            load_shader_source(GEOMETRY_SHADER_PATH).expect("Failed to load geometry shader");
        let lighting_shader_src =
            load_shader_source(LIGHTING_SHADER_PATH).expect("Failed to load lighting shader");

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

        // Create geometry bind group layout (shared for all instances)
        let geometry_bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label(Some("Geometry Uniform Bind Group Layout"))
            .with_uniform_buffer(wgpu::ShaderStages::VERTEX, None)
            .build();

        // Create three instances of the same mesh with different positions
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
        let lighting_uniform_bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label(Some("Lighting Uniform Bind Group Layout"))
            .with_uniform_buffer(wgpu::ShaderStages::FRAGMENT, None)
            .build();

        let lighting_uniform_bind_group = create_bind_group_auto(
            device,
            Some("Lighting Uniform Bind Group"),
            &lighting_uniform_bind_group_layout,
            &[lighting_uniform_buffer.as_entire_binding()],
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
            mesh_instances,
            geometry_bind_group_layout,
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
            should_reload_geometry: false,
            should_reload_lighting: false,
            player: PlayerState::new(),
            input_controller: InputController::new(),
            lights,
            num_lights,
        }
    }

    fn render(&mut self, mut context: RenderContext<'_>) {
        // Update time state and get delta time
        context.state().time.update();
        let delta_time = context.state().time.delta_time as f32;

        // Update camera from player input - do this first to avoid borrow conflicts
        let player_input = self.input_controller.get_player_input();
        self.player.update(&player_input, delta_time);
        self.player.apply_to_camera(&mut context.state().camera);

        // Get device reference after state operations
        let device = context.wgpu_device();

        // Handle shader reload
        if self.should_reload_geometry {
            eprintln!("Reloading geometry shader...");
            if let Err(e) = self.reload_geometry_shader(device) {
                eprintln!("Geometry shader reload failed: {}", e);
            } else {
                eprintln!("Geometry shader reloaded successfully!");
            }
        }

        if self.should_reload_lighting {
            eprintln!("Reloading lighting shader...");
            if let Err(e) = self.reload_lighting_shader(device) {
                eprintln!("Lighting shader reload failed: {}", e);
            } else {
                eprintln!("Lighting shader reloaded successfully!");
            }
        }

        // Resize G-buffer and depth texture if needed
        let size = context.size();
        if self.gbuffer.width != size.width || self.gbuffer.height != size.height {
            self.gbuffer.resize(device, size.width, size.height);

            // Recreate depth texture with new size
            let (depth_texture, depth_texture_view) = create_depth_texture(
                device,
                size.width,
                size.height,
                Some("GLTF Deferred Depth Texture"),
            );
            self.depth_texture = depth_texture;
            self.depth_texture_view = depth_texture_view;
        }

        // Calculate matrices
        let elapsed = context.state().time.total_time as f32;
        let aspect = size.width as f32 / size.height as f32;

        // Get all the data we need from context first to avoid borrow conflicts
        let camera = context.state().camera.clone();

        // Pre-fetch mesh resources for all instances to avoid borrow conflicts
        // Since all instances use the same mesh, we only need to fetch once
        let mesh_resource = if !self.mesh_instances.is_empty() {
            context
                .state()
                .mesh_cache
                .get_both(self.mesh_instances[0].mesh_handle)
                .map(|(_, resource)| resource.clone())
                .expect("Failed to get mesh data")
        } else {
            return;
        };

        // Get graphics device reference for queue
        let graphics_device = context.device();
        let queue = &graphics_device.queue;

        // Get current texture view from context
        let texture_view = match context.get_texture_view() {
            Some(view) => view,
            None => return,
        };

        // Create lighting uniform buffer (same for all meshes)
        let lighting_uniform =
            LightingUniform::new_with_lights(&camera, &self.lights[..self.num_lights as usize]);
        queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting_uniform]),
        );

        // Create command encoder
        let mut encoder = context
            .wgpu_device()
            .create_command_encoder(&Default::default());

        // Create G-buffer bind group for lighting pass
        let gbuffer_bind_group = create_bind_group_auto(
            context.wgpu_device(),
            Some("GBuffer Bind Group"),
            &self.gbuffer.bind_group_layout,
            &[
                wgpu::BindingResource::TextureView(&self.gbuffer.position_view),
                wgpu::BindingResource::TextureView(&self.gbuffer.normal_view),
                wgpu::BindingResource::TextureView(&self.gbuffer.albedo_view),
                wgpu::BindingResource::Sampler(&self.gbuffer.sampler),
            ],
        );

        // =====================================================================
        // GEOMETRY PASS: Render all meshes to G-buffer
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

            // Set pipeline once (same for all meshes)
            geometry_pass.set_pipeline(&self.geometry_pipeline);

            // Draw each mesh instance with its own transform
            for (i, instance) in self.mesh_instances.iter().enumerate() {
                // Create model matrix for local rotation
                // Order: center -> scale -> rotate -> position
                let center_translation = Matrix4::from_translation(-instance.center);
                let scale_matrix = Matrix4::from_scale(instance.scale);
                let rotation = Matrix4::from_angle_y(Rad(elapsed * 0.5 + i as f32 * 0.7))
                    * Matrix4::from_angle_x(Rad(elapsed * 0.3 + i as f32 * 0.4));
                let position_translation = Matrix4::from_translation(instance.position_offset);
                let model = position_translation * rotation * scale_matrix * center_translation;

                // Update this instance's uniform buffer
                let geometry_uniform = GeometryUniform::new(&camera, model, aspect);
                queue.write_buffer(
                    &instance.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[geometry_uniform]),
                );

                // Set this instance's bind group
                geometry_pass.set_bind_group(0, &instance.bind_group, &[]);

                // Draw this mesh instance
                geometry_pass.set_vertex_buffer(0, mesh_resource.vertex_buffer.slice(..));
                geometry_pass.set_index_buffer(
                    mesh_resource.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                geometry_pass.draw_indexed(0..mesh_resource.num_indices, 0, 0..1);
            }
        }

        // =====================================================================
        // LIGHTING PASS: Full-screen quad that reads G-buffer and computes lighting
        // =====================================================================
        {
            let mut lighting_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Lighting Pass"),
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

            // Draw full-screen quad
            lighting_pass.set_pipeline(&self.lighting_pipeline);
            lighting_pass.set_bind_group(0, &gbuffer_bind_group, &[]);
            lighting_pass.set_bind_group(1, &self.lighting_uniform_bind_group, &[]);
            lighting_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            lighting_pass.draw(0..6, 0..1);
        }

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
        // Forward event to input controller for key state tracking
        self.input_controller.handle_window_event(event);

        // Handle R key for shader reload
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
        {
            if let Key::Character(c) = &key_event.logical_key {
                let key_str = c.to_ascii_lowercase();
                if key_str == "r" && key_event.state.is_pressed() {
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

    // Use the Application struct with improved architecture for multi-mesh rendering
    let mut app = Application::<DeferredRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
