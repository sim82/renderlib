//! Multi-mesh instanced deferred rendering demo with GPU-based frustum culling.
//!
//! This is a variant of multi_mesh_instanced.rs that implements frustum culling
//! on the GPU using compute shaders and indirect drawing.
//!
//! Key differences:
//! - CPU computes world-space bounding spheres for all instances
//! - GPU compute shader performs frustum culling in parallel
//! - Results are used with indirect drawing
//! - Mathematically identical culling logic to the CPU version
//!
//! Demonstrates:
//! - GPU instancing with frustum culling
//! - Compute shader for parallel culling
//! - Indirect drawing
//! - Storage buffers and atomic operations

use cgmath::{Matrix4, Rad, Transform, Vector3};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{AppRenderer, Application};

use renderlib::context::{Proftime, RenderContext};
use renderlib::deferred::GBuffer;
use renderlib::device_helpers::*;
use renderlib::geometry::PosColorNormalVertex;
use renderlib::gpu_culling::{GpuCullingSystem, GpuInstanceData};
use renderlib::indirect_drawing::IndirectArgsGenerator;
use renderlib::input::InputController;
use renderlib::lighting::LightingSystem;
use renderlib::mesh::{MeshHandle, MeshSource};
use renderlib::player::PlayerState;
use renderlib::uniforms::{CameraUniform, InstanceUniform};

/// Number of mesh instances to create
const NUM_MESH_INSTANCES: usize = 1024 * 50;
// const NUM_MESH_INSTANCES: usize = 100;

/// Base spacing between mesh instances (in world units)
const BASE_SPACING: f32 = 3.0;

/// Paths to the shader files.
const GEOMETRY_SHADER_PATH: &str = "src/shaders/deferred_geometry_instanced_gpucull.wgsl";
const LIGHTING_SHADER_PATH: &str = "src/shaders/deferred_lighting.wgsl";
const CULLING_SHADER_PATH: &str = "src/shaders/frustum_culling.wgsl";
const INDIRECT_ARGS_SHADER_PATH: &str = "src/shaders/indirect_args_generation.wgsl";

/// Default mesh file path.
const DEFAULT_MESH_PATH: &str = "assets/duck.glb";

/// Per-mesh instance data encapsulating CPU and GPU resources.
struct MeshInstance {
    /// Handle to the mesh data (shared between instances of the same mesh)
    mesh_handle: MeshHandle,

    /// CPU-side transform data
    scale: f32,
    center: Vector3<f32>,
    position_offset: Vector3<f32>,

    /// Bounding sphere data for frustum culling (in local space)
    bounding_sphere_center: Vector3<f32>,
    bounding_sphere_radius: f32,

    /// Cached world-space bounding sphere data (updated per frame)
    cached_world_center: Vector3<f32>,
    cached_world_radius: f32,
}

impl MeshInstance {
    fn new(
        mesh_handle: MeshHandle,
        scale: f32,
        center: Vector3<f32>,
        position_offset: Vector3<f32>,
        bounding_sphere_center: Vector3<f32>,
        bounding_sphere_radius: f32,
    ) -> Self {
        // Initialize cached world bounding sphere (will be updated per frame)
        let local_center = bounding_sphere_center - center;
        let scaled_center = local_center * scale;
        let world_center = position_offset + scaled_center; // No rotation initially
        let world_radius = bounding_sphere_radius * scale;

        Self {
            mesh_handle,
            scale,
            center,
            position_offset,
            bounding_sphere_center,
            bounding_sphere_radius,
            cached_world_center: world_center,
            cached_world_radius: world_radius,
        }
    }

    /// Update the cached world-space bounding sphere based on rotation
    fn update_world_bounding_sphere(&mut self, rotation: Matrix4<f32>) {
        // Calculate local center relative to mesh center
        let local_center = self.bounding_sphere_center - self.center;

        // Apply scale
        let scaled_center = local_center * self.scale;

        // Apply rotation
        let rotated_center = rotation.transform_vector(scaled_center);

        // Apply position offset
        self.cached_world_center = self.position_offset + rotated_center;

        // Radius is scale-invariant to rotation, only affected by scale
        self.cached_world_radius = self.bounding_sphere_radius * self.scale;
    }

    /// Get the precomputed world-space bounding sphere
    fn get_world_bounding_sphere(&self) -> (Vector3<f32>, f32) {
        (self.cached_world_center, self.cached_world_radius)
    }
}

/// Renderer for instanced multi-mesh deferred rendering demo with GPU culling.
pub struct DeferredRenderer {
    // Mesh instances - each has its own transform data
    mesh_instances: Vec<MeshInstance>,

    // Geometry pass resources (shared)
    geometry_bind_group_layout: wgpu::BindGroupLayout,
    geometry_bind_group: wgpu::BindGroup,
    camera_uniform_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,

    geometry_pipeline: wgpu::RenderPipeline,
    geometry_shader_path: String,

    // Instancing resources
    instance_index_buffer: wgpu::Buffer,

    // Lighting system
    lighting_system: LightingSystem,

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

    // GPU Frustum Culling system
    gpu_culling: GpuCullingSystem,
    indirect_draw_buffer: wgpu::Buffer,

    // Indirect args generation system
    indirect_args_generator: IndirectArgsGenerator,

    first_iter: bool,
}

impl DeferredRenderer {
    /// Create the geometry pass pipeline for instanced rendering.
    fn create_geometry_pipeline(
        device: &wgpu::Device,
        geometry_bind_group_layout: &wgpu::BindGroupLayout,
        _gbuffer_bind_group_layout: &wgpu::BindGroupLayout,
        _surface_format: wgpu::TextureFormat,
        shader_src: &str,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_module = create_shader_module(
            device,
            Some("Instanced Deferred Geometry Shader"),
            shader_src,
        );

        let pipeline_layout = create_pipeline_layout(
            device,
            Some("Instanced Deferred Geometry Pipeline Layout"),
            &[Some(geometry_bind_group_layout)],
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
            .with_label(Some("Instanced Deferred Geometry Pipeline"))
            .with_layout(Some(&pipeline_layout))
            .with_shader_module(&shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(&[
                Some(PosColorNormalVertex::desc()),
                Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<u32>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 3, // Use location 3 (after position, color, normal)
                        format: wgpu::VertexFormat::Uint32,
                    }],
                }),
            ])
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

    /// Reload geometry shader.
    fn reload_geometry_shader(&mut self, device: &wgpu::Device) -> Result<(), String> {
        let shader_src = load_shader_source(&self.geometry_shader_path)?;
        self.geometry_pipeline = Self::create_geometry_pipeline(
            device,
            &self.geometry_bind_group_layout,
            &self.gbuffer.bind_group_layout,
            self.surface_format,
            &shader_src,
        )?;
        self.should_reload_geometry = false;
        Ok(())
    }
}

impl DeferredRenderer {
    // =========================================================================
    // Render Phase Helper Methods
    // =========================================================================

    /// Updates state for the frame: time, camera, and player input.
    fn update_frame_state(&mut self, context: &mut RenderContext<'_>) {
        // Update time state and get delta time
        context.state().time.update();
        let delta_time = context.state().time.delta_time as f32;

        // Update camera from player input - do this first to avoid borrow conflicts
        let player_input = self.input_controller.get_player_input();
        self.player.update(&player_input, delta_time);
        self.player.apply_to_camera(&mut context.state().camera);
    }

    /// Handles shader reloads if requested.
    fn handle_shader_reloads(&mut self, device: &wgpu::Device) {
        if self.should_reload_geometry {
            eprintln!("Reloading geometry shader...");
            if let Err(e) = self.reload_geometry_shader(device) {
                eprintln!("Geometry shader reload failed: {}", e);
            } else {
                eprintln!("Geometry shader reloaded successfully!");
                self.should_reload_geometry = false;
            }
        }

        if self.should_reload_lighting {
            eprintln!("Reloading lighting shader...");
            if let Err(e) = self
                .lighting_system
                .reload_shader(device, &self.gbuffer.bind_group_layout)
            {
                eprintln!("Lighting shader reload failed: {}", e);
            } else {
                eprintln!("Lighting shader reloaded successfully!");
                self.should_reload_lighting = false;
            }
        }
    }

    /// Handles resizing of G-buffer and depth texture.
    fn handle_resizing(&mut self, size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device) {
        // Resize G-buffer and depth texture if needed
        if self.gbuffer.width != size.width || self.gbuffer.height != size.height {
            self.gbuffer.resize(device, size.width, size.height);

            // Recreate depth texture with new size
            let (depth_texture, depth_texture_view) = create_depth_texture(
                device,
                size.width,
                size.height,
                Some("Instanced Deferred Depth Texture"),
            );
            self.depth_texture = depth_texture;
            self.depth_texture_view = depth_texture_view;
        }
    }

    // =========================================================================
    // Initialization Helper Methods
    // =========================================================================

    /// Creates the player and input systems.
    fn create_player_system() -> (PlayerState, InputController) {
        (PlayerState::new(), InputController::new())
    }

    // =========================================================================
    // Render Pass Methods
    // =========================================================================

    /// Renders the geometry pass with instanced drawing.
    fn render_geometry_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        mesh_resource: &renderlib::mesh::MeshResource,
        camera: &renderlib::camera::Camera,
        aspect: f32,
        instance_rotations: Option<&Vec<Matrix4<f32>>>,
        queue: &wgpu::Queue,
        pt: &mut Proftime,
    ) {
        // Update camera uniform buffer
        let camera_view_proj = camera.get_view_projection_matrix(aspect);
        let camera_uniform = CameraUniform::new(camera_view_proj);
        queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        // Update instance data if rotations available (first iteration)
        if let Some(instance_rotations) = instance_rotations {
            // Update instance storage buffer with current model matrices for ALL instances
            // Reuse the precomputed rotation matrices
            let instance_data: Vec<InstanceUniform> = self
                .mesh_instances
                .iter()
                .enumerate()
                .map(|(instance_index, instance)| {
                    let center_translation = Matrix4::from_translation(-instance.center);
                    let scale_matrix = Matrix4::from_scale(instance.scale);
                    let rotation = instance_rotations[instance_index];
                    let position_translation = Matrix4::from_translation(instance.position_offset);
                    let model = position_translation * rotation * scale_matrix * center_translation;
                    InstanceUniform::new(model)
                })
                .collect();

            // Write instance data to storage buffer for all instances
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );

            self.first_iter = false;
        }
        pt.checkpoint("instance data");

        // =====================================================================
        // GEOMETRY PASS: Render all mesh instances to G-buffer with instancing
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

            // Set pipeline and bind groups
            geometry_pass.set_pipeline(&self.geometry_pipeline);
            geometry_pass.set_bind_group(0, &self.geometry_bind_group, &[]); // Camera + instance uniforms

            // Set vertex buffers
            geometry_pass.set_vertex_buffer(0, mesh_resource.vertex_buffer.slice(..));
            geometry_pass.set_vertex_buffer(1, self.instance_index_buffer.slice(..));

            // Set index buffer
            geometry_pass.set_index_buffer(
                mesh_resource.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            // Draw using GPU-generated indirect args
            // The indirect args were generated by the GPU in the previous compute passes
            geometry_pass.draw_indexed_indirect(&self.indirect_draw_buffer, 0);
        }

        pt.checkpoint("Geometry Pass");
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

        // Create new buffers for indirect args generation first

        // Create combined bind group layout for group 0 (camera + instance storage buffer + compacted indices)
        let geometry_bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label(Some("Geometry Bind Group Layout"))
            .with_uniform_buffer(
                wgpu::ShaderStages::VERTEX,
                Some(std::mem::size_of::<CameraUniform>() as u64),
            )
            .with_storage_buffer(wgpu::ShaderStages::VERTEX, true) // instance buffer
            .with_storage_buffer(wgpu::ShaderStages::VERTEX, true) // compacted indices buffer
            .build();

        // Create mesh instances with grid positions
        let mut mesh_instances = Vec::with_capacity(NUM_MESH_INSTANCES);
        for position_offset in position_offsets.iter().take(NUM_MESH_INSTANCES) {
            mesh_instances.push(MeshInstance::new(
                mesh_handle,
                mesh_asset.scale,
                mesh_asset.center,
                *position_offset,
                mesh_asset.bounding_sphere_center,
                mesh_asset.bounding_sphere_radius,
            ));
        }

        // Create camera uniform buffer
        let camera_view_proj = camera.get_view_projection_matrix(aspect);
        let camera_uniform = CameraUniform::new(camera_view_proj);
        let camera_uniform_buffer = create_buffer(
            device,
            Some("Camera Uniform Buffer"),
            &camera_uniform,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create storage buffer for instance data
        // First, create initial instance data
        let instance_data: Vec<InstanceUniform> = mesh_instances
            .iter()
            .map(|instance| {
                // Compute model matrix using the SAME logic as before
                let center_translation = Matrix4::from_translation(-instance.center);
                let scale_matrix = Matrix4::from_scale(instance.scale);
                let rotation = Matrix4::from_angle_y(Rad(0.0)) * Matrix4::from_angle_x(Rad(0.0));
                let position_translation = Matrix4::from_translation(instance.position_offset);
                let model = position_translation * rotation * scale_matrix * center_translation;
                InstanceUniform::new(model)
            })
            .collect();

        let instance_buffer = create_buffer_from_slice(
            device,
            Some("Instance Storage Buffer"),
            &instance_data,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        // Create instance index buffer
        let instance_indices: Vec<u32> = (0..NUM_MESH_INSTANCES as u32).collect();
        let instance_index_buffer = create_buffer_from_slice(
            device,
            Some("Instance Index Buffer"),
            &instance_indices,
            wgpu::BufferUsages::VERTEX,
        );

        // Load shaders
        let geometry_shader_src = load_shader_source(GEOMETRY_SHADER_PATH).unwrap_or_else(|_| {
            // Fall back to non-instanced shader for testing
            load_shader_source("src/shaders/deferred_geometry.wgsl")
                .expect("Failed to load geometry shader")
        });

        // Create G-buffer from framework
        let gbuffer = GBuffer::new(device, size.width, size.height, Some("Deferred"));

        // Create depth texture for geometry pass
        let (depth_texture, depth_texture_view) = create_depth_texture(
            device,
            size.width,
            size.height,
            Some("Instanced Deferred Depth Texture"),
        );

        // Create geometry pipeline for instanced rendering
        let geometry_pipeline = Self::create_geometry_pipeline(
            device,
            &geometry_bind_group_layout,
            &gbuffer.bind_group_layout,
            context.surface_format(),
            &geometry_shader_src,
        )
        .expect("Failed to create geometry pipeline");

        // Create lighting system
        let lighting_system = LightingSystem::new(
            device,
            &gbuffer.bind_group_layout,
            context.surface_format(),
            LIGHTING_SHADER_PATH,
        )
        .expect("Failed to create lighting system");

        // =====================================================================
        // GPU Frustum Culling Setup
        // =====================================================================

        // Create GPU culling system
        let gpu_culling = GpuCullingSystem::new(device, NUM_MESH_INSTANCES, CULLING_SHADER_PATH)
            .expect("Failed to create GPU culling system");

        // Indirect draw buffer - contains draw arguments
        // For draw_indexed_indirect, the structure is:
        // vertex_count: u32, instance_count: u32, first_index: u32, base_vertex: i32, first_instance: u32
        // Total: 20 bytes (5 u32s)
        let indirect_draw_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Indirect Draw Buffer"),
            size: 20, // 5 u32s for DrawIndexedIndirectArgs
            usage: wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // =====================================================================
        // Indirect Args Generation Setup
        // =====================================================================

        // Create indirect args generator
        let indirect_args_generator = IndirectArgsGenerator::new(
            device,
            NUM_MESH_INSTANCES,
            &gpu_culling.visible_indices_buffer,
            &gpu_culling.atomic_counter_buffer,
            &indirect_draw_buffer,
            INDIRECT_ARGS_SHADER_PATH,
        )
        .expect("Failed to create indirect args generator");

        // Create combined bind group for group 0 (camera + instance storage buffer + compacted indices)
        let geometry_bind_group = create_bind_group_auto(
            device,
            Some("Geometry Bind Group"),
            &geometry_bind_group_layout,
            &[
                camera_uniform_buffer.as_entire_binding(),
                instance_buffer.as_entire_binding(),
                indirect_args_generator
                    .compacted_indices_buffer
                    .as_entire_binding(),
            ],
        );

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
        eprintln!(
            "Created {} instanced mesh instances in a 3D grid with GPU frustum culling",
            NUM_MESH_INSTANCES
        );

        DeferredRenderer {
            mesh_instances,
            geometry_bind_group_layout,
            camera_uniform_buffer,
            instance_buffer,
            geometry_pipeline,
            geometry_shader_path: GEOMETRY_SHADER_PATH.to_string(),
            instance_index_buffer,
            depth_texture,
            depth_texture_view,
            gbuffer,
            surface_format: context.surface_format(),
            should_reload_geometry: false,
            should_reload_lighting: false,
            player: Self::create_player_system().0,
            input_controller: Self::create_player_system().1,
            lighting_system,
            // GPU Culling system
            gpu_culling,
            indirect_draw_buffer,
            // Indirect args generation system
            indirect_args_generator,
            geometry_bind_group,
            first_iter: true,
        }
    }

    fn render(&mut self, mut context: RenderContext<'_>) {
        let mut pt = Proftime::new();

        // Phase 1: Update frame state
        self.update_frame_state(&mut context);

        // Get camera and size after state updates
        let camera = context.state().camera.clone();
        let size = context.size();
        let elapsed = context.state().time.total_time as f32;
        let aspect = size.width as f32 / size.height as f32;

        // Pre-fetch mesh resources before borrowing context for device/queue
        let (mesh_asset, mesh_resource) = if !self.mesh_instances.is_empty() {
            context
                .state()
                .mesh_cache
                .get_both(self.mesh_instances[0].mesh_handle)
                .expect("Failed to get mesh data")
        } else {
            return;
        };
        let mesh_resource = mesh_resource.clone();

        pt.checkpoint("pre-fetch mesh resources");

        // Get device reference after state operations
        let device = context.wgpu_device();
        let queue = context.wgpu_queue();

        // Phase 2: Handle shader reloads
        self.handle_shader_reloads(device);

        // Phase 3: Handle resizing
        self.handle_resizing(size, device);

        pt.checkpoint("Setup");

        // =====================================================================
        // NEW: GPU Frustum Culling with Direct Usage Pipeline
        // =====================================================================
        // This implementation uses a fully GPU-driven pipeline:
        // 1. Dispatch frustum culling compute shader
        // 2. Dispatch indirect args generation compute shader
        // 3. Use GPU-generated indirect draw args directly in geometry pass

        // Step 1: Precompute rotation matrices for all instances and update cached world bounding spheres

        let instance_rotations = if self.first_iter {
            let base_rotation_y = elapsed * 0.5;
            let base_rotation_x = elapsed * 0.3;
            let instance_rotations: Vec<Matrix4<f32>> = (0..NUM_MESH_INSTANCES)
                .map(|i| {
                    Matrix4::from_angle_y(Rad(base_rotation_y + i as f32 * 0.7))
                        * Matrix4::from_angle_x(Rad(base_rotation_x + i as f32 * 0.4))
                })
                .collect();

            // Update cached world bounding spheres using precomputed rotations
            for (i, instance) in self.mesh_instances.iter_mut().enumerate() {
                instance.update_world_bounding_sphere(instance_rotations[i]);
            }
            // Step 2: Compute world-space bounding spheres for all instances on CPU (now using cached values)
            let instance_data: Vec<GpuInstanceData> = self
                .mesh_instances
                .iter()
                .map(|instance| {
                    let (world_center, world_radius) = instance.get_world_bounding_sphere();
                    GpuInstanceData::new(world_center, world_radius)
                })
                .collect();

            // Step 2: Upload instance data to GPU buffer
            self.gpu_culling.update_instance_data(queue, &instance_data);
            Some(instance_rotations)
        } else {
            None
        };

        pt.checkpoint("GPU Instance date");

        // Step 3: Upload view matrix and camera params to GPU
        self.gpu_culling
            .update_camera_params(queue, &camera, aspect);

        // Step 4: Update mesh info uniform (CPU → GPU)
        self.indirect_args_generator.update_mesh_info(
            queue,
            mesh_resource.num_indices as u32,
            mesh_asset.vertices.len() as u32,
        );

        // Step 5: Reset atomic counter
        self.gpu_culling.reset_atomic_counter(queue);

        // Step 6: Dispatch frustum culling compute shader
        let mut culling_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Culling Command Encoder"),
        });
        self.gpu_culling
            .dispatch(&mut culling_encoder, NUM_MESH_INSTANCES);
        queue.submit([culling_encoder.finish()]);

        // Step 7: Dispatch indirect args generation compute shader
        let mut indirect_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Indirect Args Command Encoder"),
        });
        self.indirect_args_generator.dispatch(&mut indirect_encoder);
        queue.submit([indirect_encoder.finish()]);
        pt.checkpoint("Frustum Culling");
        // Use the mesh_resource we fetched earlier
        if self.mesh_instances.is_empty() {
            return;
        }

        // Update lighting uniforms
        self.lighting_system.update_uniforms(queue, &camera);

        // Get current texture view from context
        let texture_view = match context.get_texture_view() {
            Some(view) => view,
            None => return,
        };

        // Phase 4: Render geometry pass
        let mut encoder = context
            .wgpu_device()
            .create_command_encoder(&Default::default());
        self.render_geometry_pass(
            &mut encoder,
            &mesh_resource,
            &camera,
            aspect,
            instance_rotations.as_ref(),
            queue,
            &mut pt,
        );

        // Phase 5: Render lighting pass
        self.lighting_system
            .render_pass(&mut encoder, device, &self.gbuffer, &texture_view);
        pt.checkpoint("Deferred Rendering");

        // Phase 6: Submit commands
        context.wgpu_queue().submit([encoder.finish()]);
        pt.checkpoint("Submit");
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
            Some("Instanced Deferred Depth Texture"),
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

    // Use the Application struct with improved architecture
    let mut app = Application::<DeferredRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
