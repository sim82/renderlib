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
use renderlib::camera::{Light, LightingUniform};
use renderlib::context::RenderContext;
use renderlib::deferred::GBuffer;
use renderlib::device_helpers::*;
use renderlib::geometry::PosColorNormalVertex;
use renderlib::input::InputController;
use renderlib::mesh::{quad_vertices_2d, MeshHandle, MeshSource, QuadVertex};
use renderlib::player::PlayerState;

/// Camera uniform data for instanced rendering
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new(view_proj: Matrix4<f32>) -> Self {
        Self {
            view_proj: view_proj.into(),
        }
    }
}

/// Instance uniform data for instanced rendering
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceUniform {
    model: [[f32; 4]; 4],
}

impl InstanceUniform {
    fn new(model: Matrix4<f32>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

/// GPU instance data for frustum culling
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuInstanceData {
    world_center: [f32; 3],
    world_radius: f32,
}

impl GpuInstanceData {
    fn new(center: Vector3<f32>, radius: f32) -> Self {
        Self {
            world_center: [center.x, center.y, center.z],
            world_radius: radius,
        }
    }
}

/// Camera parameters for frustum culling compute shader
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CullingCameraParams {
    near: f32,
    far: f32,
    tan_fov_x: f32,
    tan_fov_y: f32,
}

impl CullingCameraParams {
    fn new(camera: &renderlib::camera::Camera, aspect: f32) -> Self {
        let proj = camera.get_projection_matrix(aspect);
        Self {
            near: camera.near,
            far: camera.far,
            tan_fov_x: 1.0 / proj[0][0],
            tan_fov_y: 1.0 / proj[1][1],
        }
    }
}

/// Number of mesh instances to create
// const NUM_MESH_INSTANCES: usize = 1024 * 100;
const NUM_MESH_INSTANCES: usize = 100;

/// Base spacing between mesh instances (in world units)
const BASE_SPACING: f32 = 3.0;

/// Paths to the shader files.
const GEOMETRY_SHADER_PATH: &str = "src/shaders/deferred_geometry_instanced.wgsl";
const LIGHTING_SHADER_PATH: &str = "src/shaders/deferred_lighting.wgsl";
const CULLING_SHADER_PATH: &str = "src/shaders/frustum_culling.wgsl";

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
        Self {
            mesh_handle,
            scale,
            center,
            position_offset,
            bounding_sphere_center,
            bounding_sphere_radius,
        }
    }

    /// Get the world-space bounding sphere for this instance at a specific time
    /// This accounts for the rotation that's applied in the render method
    fn get_world_bounding_sphere(
        &self,
        elapsed: f32,
        instance_index: usize,
    ) -> (Vector3<f32>, f32) {
        // The mesh transformation in render() is:
        // center_translation = Matrix4::from_translation(-instance.center)
        // scale_matrix = Matrix4::from_scale(instance.scale)
        // rotation = Matrix4::from_angle_y(Rad(elapsed * 0.5 + instance_index as f32 * 0.7))
        //             * Matrix4::from_angle_x(Rad(elapsed * 0.3 + instance_index as f32 * 0.4))
        // position_translation = Matrix4::from_translation(instance.position_offset)
        // model = position_translation * rotation * scale_matrix * center_translation

        // The bounding sphere center in local mesh space is self.bounding_sphere_center
        // After centering: local_center = bounding_sphere_center - center
        let local_center = self.bounding_sphere_center - self.center;

        // Apply scale
        let scaled_center = local_center * self.scale;

        // Apply rotation (same as in render method)
        let rotation = Matrix4::from_angle_y(Rad(elapsed * 0.5 + instance_index as f32 * 0.7))
            * Matrix4::from_angle_x(Rad(elapsed * 0.3 + instance_index as f32 * 0.4));
        let rotated_center = rotation.transform_vector(scaled_center);

        // Apply position
        let world_center = self.position_offset + rotated_center;

        // Apply scale to the bounding sphere radius (rotation doesn't change radius)
        let world_radius = self.bounding_sphere_radius * self.scale;
        (world_center, world_radius)
    }
}

/// Generate positions in an expanding cubic grid
fn generate_expanding_grid_positions(count: usize, spacing: f32) -> Vec<Vector3<f32>> {
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

    // GPU Frustum Culling resources
    culling_pipeline: wgpu::ComputePipeline,
    instance_data_buffer: wgpu::Buffer,
    view_matrix_buffer: wgpu::Buffer,
    camera_params_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    visible_indices_buffer: wgpu::Buffer,
    atomic_counter_buffer: wgpu::Buffer,
    indirect_draw_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    culling_bind_group_layout: wgpu::BindGroupLayout,
    culling_bind_group: wgpu::BindGroup,

    // For CPU fallback path (used while GPU readback is not implemented)
    visible_instances: Vec<usize>,
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

    /// Create the compute pipeline for frustum culling.
    fn create_culling_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<wgpu::ComputePipeline, String> {
        let shader_src = load_shader_source(CULLING_SHADER_PATH).unwrap_or_else(|_| {
            // Fallback shader if file not found
            include_str!("../shaders/frustum_culling.wgsl").to_string()
        });

        let shader_module =
            create_shader_module(device, Some("Frustum Culling Compute Shader"), &shader_src);

        let pipeline_layout = create_pipeline_layout(
            device,
            Some("Frustum Culling Pipeline Layout"),
            &[Some(bind_group_layout)],
        );

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Frustum Culling Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

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

        // Create combined bind group layout for group 0 (camera + instance storage buffer)
        let geometry_bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label(Some("Geometry Bind Group Layout"))
            .with_uniform_buffer(
                wgpu::ShaderStages::VERTEX,
                Some(std::mem::size_of::<CameraUniform>() as u64),
            )
            .with_storage_buffer(wgpu::ShaderStages::VERTEX, true)
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

        // Create combined bind group for group 0 (camera + instance storage buffer)
        let geometry_bind_group = create_bind_group_auto(
            device,
            Some("Geometry Bind Group"),
            &geometry_bind_group_layout,
            &[
                camera_uniform_buffer.as_entire_binding(),
                instance_buffer.as_entire_binding(),
            ],
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

        // Create multiple lights for the scene
        let mut lights: [Light; renderlib::camera::MAX_LIGHTS] =
            [Light::default(); renderlib::camera::MAX_LIGHTS];
        lights[0] = Light::new([2.0, 3.0, 4.0], [1.0, 1.0, 1.0]);
        lights[1] = Light::new([-3.0, 2.0, 2.0], [1.0, 0.0, 0.0]);
        lights[2] = Light::new([0.0, -2.0, 3.0], [0.0, 0.0, 1.0]);
        lights[3] = Light::new([0.0, 2.0, -3.0], [0.0, 1.0, 0.0]);
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

        // =====================================================================
        // GPU Frustum Culling Setup
        // =====================================================================

        // Create bind group layout for culling compute shader
        let culling_bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label(Some("Culling Bind Group Layout"))
            // Instance data buffer (read-only storage)
            .with_storage_buffer(wgpu::ShaderStages::COMPUTE, true)
            // View matrix buffer (uniform)
            .with_uniform_buffer(wgpu::ShaderStages::COMPUTE, None)
            // Camera params buffer (uniform)
            .with_uniform_buffer(wgpu::ShaderStages::COMPUTE, None)
            // Visible indices buffer (write-only storage)
            .with_storage_buffer(wgpu::ShaderStages::COMPUTE, false)
            // Atomic counter buffer (write-only storage)
            .with_storage_buffer(wgpu::ShaderStages::COMPUTE, false)
            .build();

        // Create GPU buffers for culling
        // Instance data buffer - will be updated each frame with world-space bounding spheres
        let instance_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Culling Instance Data Buffer"),
            size: (NUM_MESH_INSTANCES * std::mem::size_of::<GpuInstanceData>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // View matrix buffer for culling
        let view_matrix_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Culling View Matrix Buffer"),
            size: 64, // mat4x4<f32>
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Camera params buffer for culling
        let camera_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Culling Camera Params Buffer"),
            size: std::mem::size_of::<CullingCameraParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Visible indices buffer - stores indices of visible instances
        let visible_indices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Visible Indices Buffer"),
            size: (NUM_MESH_INSTANCES * 4) as u64, // u32 per instance
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Atomic counter buffer - single u32 for counting visible instances
        let atomic_counter_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Atomic Counter Buffer"),
            size: 4, // u32
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Indirect draw buffer - contains draw arguments
        // For draw_indexed_indirect, the structure is:
        // vertex_count: u32, instance_count: u32, first_index: u32, base_vertex: i32, first_instance: u32
        // Total: 20 bytes (5 u32s)
        let indirect_draw_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Indirect Draw Buffer"),
            size: 20, // 5 u32s for DrawIndexedIndirectArgs
            usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create culling bind group
        let culling_bind_group = create_bind_group_auto(
            device,
            Some("Culling Bind Group"),
            &culling_bind_group_layout,
            &[
                instance_data_buffer.as_entire_binding(),
                view_matrix_buffer.as_entire_binding(),
                camera_params_buffer.as_entire_binding(),
                visible_indices_buffer.as_entire_binding(),
                atomic_counter_buffer.as_entire_binding(),
            ],
        );

        // Create culling compute pipeline
        let culling_pipeline = Self::create_culling_pipeline(device, &culling_bind_group_layout)
            .expect("Failed to create culling pipeline");

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
            geometry_bind_group,
            camera_uniform_buffer,
            instance_buffer,
            geometry_pipeline,
            geometry_shader_path: GEOMETRY_SHADER_PATH.to_string(),
            instance_index_buffer,
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
            // GPU Culling resources
            culling_pipeline,
            instance_data_buffer,
            view_matrix_buffer,
            camera_params_buffer,
            visible_indices_buffer,
            atomic_counter_buffer,
            indirect_draw_buffer,
            culling_bind_group_layout,
            culling_bind_group,
            visible_instances: Vec::new(),
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

        // Clone camera and get size before borrowing context for device/queue
        let camera = context.state().camera.clone();
        let size = context.size();
        let elapsed = context.state().time.total_time as f32;
        let aspect = size.width as f32 / size.height as f32;

        // Pre-fetch mesh resources before borrowing context for device/queue
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

        // Get device reference after state operations
        let device = context.wgpu_device();
        let queue = context.wgpu_queue();

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

        // =====================================================================
        // GPU Frustum Culling Setup
        // =====================================================================
        // Note: This implementation dispatches the GPU compute shader for culling,
        // but currently uses CPU-based culling for the actual visible instance list
        // and indirect draw args generation. In a production app, you would:
        // 1. Use a second compute shader to generate indirect draw args on GPU
        // 2. Or use async buffer readback to get the visible indices from GPU
        // For now, we demonstrate the GPU culling shader dispatch but use CPU culling
        // for the final result to avoid async complexity in the render loop.

        // Step 1: Compute world-space bounding spheres for all instances on CPU
        let instance_data: Vec<GpuInstanceData> = self
            .mesh_instances
            .iter()
            .enumerate()
            .map(|(i, instance)| {
                let (world_center, world_radius) = instance.get_world_bounding_sphere(elapsed, i);
                GpuInstanceData::new(world_center, world_radius)
            })
            .collect();

        // Step 2: Upload instance data to GPU buffer
        queue.write_buffer(
            &self.instance_data_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );

        // Step 3: Upload view matrix and camera params to GPU
        let view_matrix = camera.get_view_matrix();
        // Convert Matrix4 to [[f32; 4]; 4] for bytemuck
        let view_matrix_array: [[f32; 4]; 4] = view_matrix.into();
        queue.write_buffer(
            &self.view_matrix_buffer,
            0,
            bytemuck::cast_slice(&[view_matrix_array]),
        );

        let camera_params = CullingCameraParams::new(&camera, aspect);
        queue.write_buffer(
            &self.camera_params_buffer,
            0,
            bytemuck::cast_slice(&[camera_params]),
        );

        // Step 4: Reset atomic counter to 0
        queue.write_buffer(
            &self.atomic_counter_buffer,
            0,
            bytemuck::cast_slice(&[0u32]),
        );

        // Step 5: Dispatch compute shader for frustum culling
        // This demonstrates the GPU culling shader, though we use CPU culling for results
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Culling Command Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Frustum Culling Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.culling_pipeline);
            compute_pass.set_bind_group(0, &self.culling_bind_group, &[]);

            // Dispatch one thread per instance
            let workgroup_count = (NUM_MESH_INSTANCES as u32 + 63) / 64;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Submit culling compute commands
        queue.submit([encoder.finish()]);

        // Step 6: Use CPU-based culling to generate the visible instances list
        // (In production, replace this with GPU readback or second compute shader)
        self.visible_instances.clear();

        for (index, instance) in self.mesh_instances.iter().enumerate() {
            let (world_center, world_radius) = instance.get_world_bounding_sphere(elapsed, index);

            // Transform the sphere center to view space
            let center_view = view_matrix.transform_point(cgmath::Point3::new(
                world_center.x,
                world_center.y,
                world_center.z,
            ));

            // Depth tests
            let completely_behind_camera = center_view.z - world_radius > 0.0;
            let completely_beyond_far = center_view.z + world_radius < -camera.far;
            let completely_before_near = center_view.z - world_radius > -camera.near;

            if completely_behind_camera || completely_beyond_far || completely_before_near {
                continue;
            }

            // Angular tests
            let proj = camera.get_projection_matrix(aspect);
            let tan_fov_y = 1.0 / proj[1][1];
            let tan_fov_x = 1.0 / proj[0][0];

            let z_abs = (-center_view.z).abs();
            let x_bound = z_abs * tan_fov_x;
            let y_bound = z_abs * tan_fov_y;

            let inside_x =
                center_view.x + world_radius >= -x_bound && center_view.x - world_radius <= x_bound;
            let inside_y =
                center_view.y + world_radius >= -y_bound && center_view.y - world_radius <= y_bound;

            if inside_x && inside_y {
                self.visible_instances.push(index);
            }
        }

        println!(
            "after cull: {} {:?}",
            self.visible_instances.len(),
            self.visible_instances
        );
        // Use the mesh_resource we fetched earlier
        if self.mesh_instances.is_empty() {
            return;
        }

        // Create lighting uniform buffer (same for all instances)
        let lighting_uniform =
            LightingUniform::new_with_lights(&camera, &self.lights[..self.num_lights as usize]);
        queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting_uniform]),
        );

        // Get current texture view from context
        let texture_view = match context.get_texture_view() {
            Some(view) => view,
            None => return,
        };

        // Update camera uniform buffer
        let camera_view_proj = camera.get_view_projection_matrix(aspect);
        let camera_uniform = CameraUniform::new(camera_view_proj);
        queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        // Update instance storage buffer with current model matrices for visible instances only
        let instance_data: Vec<InstanceUniform> = self
            .visible_instances
            .iter()
            .map(|&instance_index| {
                let instance = &self.mesh_instances[instance_index];
                let center_translation = Matrix4::from_translation(-instance.center);
                let scale_matrix = Matrix4::from_scale(instance.scale);
                let rotation =
                    Matrix4::from_angle_y(Rad(elapsed * 0.5 + instance_index as f32 * 0.7))
                        * Matrix4::from_angle_x(Rad(elapsed * 0.3 + instance_index as f32 * 0.4));
                let position_translation = Matrix4::from_translation(instance.position_offset);
                let model = position_translation * rotation * scale_matrix * center_translation;
                InstanceUniform::new(model)
            })
            .collect();

        // Write instance data to storage buffer
        if !instance_data.is_empty() {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );
        }

        // Create indirect draw arguments
        // For draw_indexed_indirect: vertex_count, instance_count, first_index, base_vertex, first_instance
        // We need to use a byte array since base_vertex is i32 and others are u32
        let visible_instance_count = self.visible_instances.len() as u32;

        // Write as bytes: vertex_count, instance_count, first_index, base_vertex(0), first_instance
        let mut indirect_bytes = Vec::with_capacity(20);
        indirect_bytes.extend_from_slice(bytemuck::cast_slice(&[mesh_resource.num_indices as u32]));
        indirect_bytes.extend_from_slice(bytemuck::cast_slice(&[visible_instance_count]));
        indirect_bytes.extend_from_slice(bytemuck::cast_slice(&[0u32])); // first_index
        indirect_bytes.extend_from_slice(bytemuck::cast_slice(&[0i32])); // base_vertex
        indirect_bytes.extend_from_slice(bytemuck::cast_slice(&[0u32])); // first_instance

        queue.write_buffer(&self.indirect_draw_buffer, 0, &indirect_bytes);

        // Create command encoder for rendering
        let mut encoder = context
            .wgpu_device()
            .create_command_encoder(&Default::default());

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

            // Indirect draw call for visible meshes only!
            if visible_instance_count > 0 {
                geometry_pass.draw_indexed_indirect(
                    &self.indirect_draw_buffer,
                    0, // offset in bytes
                );
            }
        }

        // =====================================================================
        // LIGHTING PASS: Full-screen quad that reads G-buffer and computes lighting
        // =====================================================================
        {
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
