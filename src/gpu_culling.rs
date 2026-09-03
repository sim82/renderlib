//! GPU-based frustum culling system.
//!
//! Provides a reusable component for performing frustum culling on the GPU
//! using compute shaders and atomic operations.

use cgmath::Vector3;
use wgpu;

use crate::camera::Camera;
use crate::device_helpers::{create_shader_module, load_shader_source, BindGroupLayoutBuilder};

/// Parameters for the culling camera (used in compute shader).
///
/// These parameters define the view frustum for GPU-based culling.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CullingCameraParams {
    /// Near plane distance
    pub near: f32,
    /// Far plane distance
    pub far: f32,
    /// Tangent of the horizontal field of view
    pub tan_fov_x: f32,
    /// Tangent of the vertical field of view
    pub tan_fov_y: f32,
}

impl CullingCameraParams {
    /// Creates new culling camera parameters from a camera and aspect ratio.
    ///
    /// # Arguments
    ///
    /// * `camera` - The camera to extract parameters from
    /// * `aspect` - The aspect ratio (width/height)
    pub fn new(camera: &Camera, aspect: f32) -> Self {
        let proj = camera.get_projection_matrix(aspect);
        Self {
            near: camera.near,
            far: camera.far,
            tan_fov_x: 1.0 / proj[0][0],
            tan_fov_y: 1.0 / proj[1][1],
        }
    }
}

/// Data for each instance used in culling.
///
/// Contains the world-space bounding sphere for frustum culling.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuInstanceData {
    /// World-space center of the bounding sphere
    pub world_center: [f32; 3],
    /// Radius of the bounding sphere
    pub world_radius: f32,
}

impl GpuInstanceData {
    /// Creates new GPU instance data from a center and radius.
    ///
    /// # Arguments
    ///
    /// * `center` - The world-space center of the bounding sphere
    /// * `radius` - The radius of the bounding sphere
    pub fn new(center: Vector3<f32>, radius: f32) -> Self {
        Self {
            world_center: [center.x, center.y, center.z],
            world_radius: radius,
        }
    }
}

/// GPU-based frustum culling system.
///
/// This system performs frustum culling on the GPU using a compute shader.
/// It uses atomic operations to count visible instances and store their indices.
pub struct GpuCullingSystem {
    /// The compute pipeline for frustum culling
    pub pipeline: wgpu::ComputePipeline,
    /// Buffer containing instance data (world centers and radii)
    pub instance_data_buffer: wgpu::Buffer,
    /// Buffer containing the view matrix
    pub view_matrix_buffer: wgpu::Buffer,
    /// Buffer containing camera parameters
    pub camera_params_buffer: wgpu::Buffer,
    /// Buffer to store indices of visible instances
    pub visible_indices_buffer: wgpu::Buffer,
    /// Buffer for atomic counter of visible instances
    pub atomic_counter_buffer: wgpu::Buffer,
    /// Bind group layout for the culling pipeline
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for the culling pipeline
    pub bind_group: wgpu::BindGroup,
    /// Maximum number of instances this system can handle
    pub max_instances: usize,
}

impl GpuCullingSystem {
    /// Creates a new GPU culling system.
    ///
    /// # Arguments
    ///
    /// * `device` - The WGPU device to create resources with
    /// * `max_instances` - Maximum number of instances to support
    /// * `shader_path` - Path to the frustum culling compute shader
    ///
    /// # Returns
    ///
    /// A new `GpuCullingSystem` ready for use, or an error if creation fails.
    pub fn new(
        device: &wgpu::Device,
        max_instances: usize,
        shader_path: &str,
    ) -> Result<Self, String> {
        // Create buffers
        let instance_data_buffer = Self::create_instance_data_buffer(device, max_instances);
        let view_matrix_buffer = Self::create_view_matrix_buffer(device);
        let camera_params_buffer = Self::create_camera_params_buffer(device);
        let visible_indices_buffer = Self::create_visible_indices_buffer(device, max_instances);
        let atomic_counter_buffer = Self::create_atomic_counter_buffer(device);

        // Create bind group layout
        let bind_group_layout = Self::create_bind_group_layout(device)?;

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GPU Culling Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &instance_data_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            (max_instances * std::mem::size_of::<GpuInstanceData>()) as u64,
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &view_matrix_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<[[f32; 4]; 4]>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &camera_params_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            std::mem::size_of::<CullingCameraParams>() as u64
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &visible_indices_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            (max_instances * std::mem::size_of::<u32>()) as u64,
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &atomic_counter_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<u32>() as u64),
                    }),
                },
            ],
        });

        // Load shader and create pipeline
        let shader_src = load_shader_source(shader_path)
            .unwrap_or_else(|_| include_str!("shaders/frustum_culling.wgsl").to_string());
        let shader_module = create_shader_module(device, Some("GPU Culling Shader"), &shader_src);
        let pipeline = Self::create_pipeline(device, &bind_group_layout, shader_module);

        Ok(Self {
            pipeline,
            instance_data_buffer,
            view_matrix_buffer,
            camera_params_buffer,
            visible_indices_buffer,
            atomic_counter_buffer,
            bind_group_layout,
            bind_group,
            max_instances,
        })
    }

    /// Creates the instance data buffer.
    fn create_instance_data_buffer(device: &wgpu::Device, max_instances: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Culling Instance Data Buffer"),
            size: (max_instances * std::mem::size_of::<GpuInstanceData>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Creates the view matrix buffer.
    fn create_view_matrix_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Culling View Matrix Buffer"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Creates the camera parameters buffer.
    fn create_camera_params_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Culling Camera Params Buffer"),
            size: std::mem::size_of::<CullingCameraParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Creates the visible indices buffer.
    fn create_visible_indices_buffer(device: &wgpu::Device, max_instances: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Culling Visible Indices Buffer"),
            size: (max_instances * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Creates the atomic counter buffer.
    fn create_atomic_counter_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Culling Atomic Counter Buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Creates the bind group layout for the culling pipeline.
    fn create_bind_group_layout(device: &wgpu::Device) -> Result<wgpu::BindGroupLayout, String> {
        let mut builder =
            BindGroupLayoutBuilder::new(device).with_label(Some("GPU Culling Bind Group Layout"));

        // Binding 0: Instance data buffer (storage, read-only) - matches shader var<storage, read>
        builder = builder.with_storage_buffer(wgpu::ShaderStages::COMPUTE, true);
        // Binding 1: View matrix buffer (uniform)
        builder = builder.with_uniform_buffer(wgpu::ShaderStages::COMPUTE, None);
        // Binding 2: Camera params buffer (uniform)
        builder = builder.with_uniform_buffer(wgpu::ShaderStages::COMPUTE, None);
        // Binding 3: Visible indices buffer (storage, read-write) - matches shader var<storage, read_write>
        builder = builder.with_storage_buffer(wgpu::ShaderStages::COMPUTE, false);
        // Binding 4: Atomic counter buffer (storage, read-write) - matches shader var<storage, read_write>
        builder = builder.with_storage_buffer(wgpu::ShaderStages::COMPUTE, false);

        Ok(builder.build())
    }

    /// Creates the compute pipeline.
    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        shader_module: wgpu::ShaderModule,
    ) -> wgpu::ComputePipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GPU Culling Pipeline Layout"),
            bind_group_layouts: &[Some(bind_group_layout)],
            immediate_size: 0,
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GPU Frustum Culling Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }

    /// Updates the camera parameters for culling.
    ///
    /// # Arguments
    ///
    /// * `queue` - The WGPU queue to use for buffer updates
    /// * `camera` - The camera to extract view parameters from
    /// * `aspect` - The aspect ratio (width/height)
    pub fn update_camera_params(&self, queue: &wgpu::Queue, camera: &Camera, aspect: f32) {
        // Update view matrix
        let view_matrix: [[f32; 4]; 4] = camera.get_view_matrix().into();
        queue.write_buffer(
            &self.view_matrix_buffer,
            0,
            bytemuck::cast_slice(&[view_matrix]),
        );

        // Update camera parameters
        let camera_params = CullingCameraParams::new(camera, aspect);
        queue.write_buffer(
            &self.camera_params_buffer,
            0,
            bytemuck::cast_slice(&[camera_params]),
        );
    }

    /// Updates the instance data for culling.
    ///
    /// # Arguments
    ///
    /// * `queue` - The WGPU queue to use for buffer updates
    /// * `instance_data` - The instance data to upload
    pub fn update_instance_data(&self, queue: &wgpu::Queue, instance_data: &[GpuInstanceData]) {
        queue.write_buffer(
            &self.instance_data_buffer,
            0,
            bytemuck::cast_slice(instance_data),
        );
    }

    /// Resets the atomic counter to zero.
    ///
    /// # Arguments
    ///
    /// * `queue` - The WGPU queue to use for buffer updates
    pub fn reset_atomic_counter(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.atomic_counter_buffer,
            0,
            bytemuck::cast_slice(&[0u32]),
        );
    }

    /// Dispatches the culling compute shader.
    ///
    /// # Arguments
    ///
    /// * `encoder` - The command encoder to record commands into
    /// * `instance_count` - The number of instances to cull
    pub fn dispatch(&self, encoder: &mut wgpu::CommandEncoder, instance_count: usize) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Frustum Culling Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);

        // Dispatch one thread per instance, with 64 threads per workgroup
        let workgroup_count = (instance_count as u32 + 63) / 64;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }
}
