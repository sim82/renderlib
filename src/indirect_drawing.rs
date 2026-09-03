//! Indirect drawing system for GPU-driven rendering.
//!
//! Provides a reusable component for generating indirect draw arguments on the GPU
//! using compute shaders. This enables efficient rendering of large numbers of
//! instances with minimal CPU overhead.

use wgpu;

use crate::device_helpers::{create_shader_module, load_shader_source, BindGroupLayoutBuilder};

/// System for generating indirect draw arguments on the GPU.
///
/// This system uses a compute shader to generate draw arguments based on
/// the results of GPU frustum culling, enabling efficient indirect rendering.
pub struct IndirectArgsGenerator {
    /// The compute pipeline for indirect args generation
    pub pipeline: wgpu::ComputePipeline,
    /// Buffer to store the count of visible instances
    pub visible_count_buffer: wgpu::Buffer,
    /// Buffer to store compacted indices of visible instances
    pub compacted_indices_buffer: wgpu::Buffer,
    /// Buffer to store the final indirect draw arguments
    pub indirect_draw_buffer: wgpu::Buffer,
    /// Buffer containing mesh information (index count, vertex count)
    pub mesh_info_uniform_buffer: wgpu::Buffer,
    /// Bind group layout for the indirect args pipeline
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for the indirect args pipeline
    pub bind_group: wgpu::BindGroup,
}

impl IndirectArgsGenerator {
    /// Creates a new indirect args generator.
    ///
    /// # Arguments
    ///
    /// * `device` - The WGPU device to create resources with
    /// * `max_instances` - Maximum number of instances to support
    /// * `visible_indices_buffer` - Buffer containing indices of visible instances (from culling)
    /// * `atomic_counter_buffer` - Buffer containing the count of visible instances (from culling)
    /// * `indirect_draw_buffer` - Buffer to store the generated indirect draw arguments
    /// * `shader_path` - Path to the indirect args generation compute shader
    ///
    /// # Returns
    ///
    /// A new `IndirectArgsGenerator` ready for use, or an error if creation fails.
    pub fn new(
        device: &wgpu::Device,
        max_instances: usize,
        visible_indices_buffer: &wgpu::Buffer,
        atomic_counter_buffer: &wgpu::Buffer,
        indirect_draw_buffer: &wgpu::Buffer,
        shader_path: &str,
    ) -> Result<Self, String> {
        // Create buffers
        let visible_count_buffer = Self::create_visible_count_buffer(device);
        let compacted_indices_buffer = Self::create_compacted_indices_buffer(device, max_instances);
        let mesh_info_uniform_buffer = Self::create_mesh_info_uniform_buffer(device);

        // Create bind group layout
        let bind_group_layout = Self::create_bind_group_layout(device)?;

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Indirect Args Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: visible_indices_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            (max_instances * std::mem::size_of::<u32>()) as u64,
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: atomic_counter_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<u32>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &visible_count_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<u32>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &compacted_indices_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            (max_instances * std::mem::size_of::<u32>()) as u64,
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: indirect_draw_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(20), // 5 u32s for DrawIndexedIndirectArgs
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &mesh_info_uniform_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(8), // 2 u32s: index_count, vertex_count
                    }),
                },
            ],
        });

        // Load shader and create pipeline
        let shader_src = load_shader_source(shader_path)
            .unwrap_or_else(|_| include_str!("shaders/indirect_args_generation.wgsl").to_string());
        let shader_module =
            create_shader_module(device, Some("Indirect Args Generation Shader"), &shader_src);
        let pipeline = Self::create_pipeline(device, &bind_group_layout, shader_module);

        Ok(Self {
            pipeline,
            visible_count_buffer,
            compacted_indices_buffer,
            indirect_draw_buffer: indirect_draw_buffer.clone(),
            mesh_info_uniform_buffer,
            bind_group_layout,
            bind_group,
        })
    }

    /// Creates the visible count buffer.
    fn create_visible_count_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Visible Count Buffer"),
            size: 4, // Single u32
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Creates the compacted indices buffer.
    fn create_compacted_indices_buffer(
        device: &wgpu::Device,
        max_instances: usize,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compacted Indices Buffer"),
            size: (max_instances * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Creates the mesh info uniform buffer.
    fn create_mesh_info_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Info Uniform Buffer"),
            size: 8, // Two u32s: index_count, vertex_count
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Creates the bind group layout for the indirect args pipeline.
    fn create_bind_group_layout(device: &wgpu::Device) -> Result<wgpu::BindGroupLayout, String> {
        let mut builder =
            BindGroupLayoutBuilder::new(device).with_label(Some("Indirect Args Bind Group Layout"));

        // Binding 0: Visible indices buffer (read-only storage)
        builder = builder.with_storage_buffer(wgpu::ShaderStages::COMPUTE, true);
        // Binding 1: Atomic counter buffer (read-only storage)
        builder = builder.with_storage_buffer(wgpu::ShaderStages::COMPUTE, true);
        // Binding 2: Visible count buffer (write-only storage)
        builder = builder.with_storage_buffer(wgpu::ShaderStages::COMPUTE, false);
        // Binding 3: Compacted indices buffer (write-only storage)
        builder = builder.with_storage_buffer(wgpu::ShaderStages::COMPUTE, false);
        // Binding 4: Indirect draw buffer (write-only storage)
        builder = builder.with_storage_buffer(wgpu::ShaderStages::COMPUTE, false);
        // Binding 5: Mesh info uniform buffer (uniform)
        builder = builder.with_uniform_buffer(wgpu::ShaderStages::COMPUTE, None);

        Ok(builder.build())
    }

    /// Creates the compute pipeline.
    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        shader_module: wgpu::ShaderModule,
    ) -> wgpu::ComputePipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Indirect Args Pipeline Layout"),
            bind_group_layouts: &[Some(bind_group_layout)],
            immediate_size: 0,
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Indirect Args Generation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }

    /// Updates the mesh info uniform buffer.
    ///
    /// # Arguments
    ///
    /// * `queue` - The WGPU queue to use for buffer updates
    /// * `index_count` - Number of indices in the mesh
    /// * `vertex_count` - Number of vertices in the mesh
    pub fn update_mesh_info(&self, queue: &wgpu::Queue, index_count: u32, vertex_count: u32) {
        let mesh_info = [index_count, vertex_count];
        queue.write_buffer(
            &self.mesh_info_uniform_buffer,
            0,
            bytemuck::cast_slice(&mesh_info),
        );
    }

    /// Dispatches the indirect args generation compute shader.
    ///
    /// # Arguments
    ///
    /// * `encoder` - The command encoder to record commands into
    pub fn dispatch(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Indirect Args Generation Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);

        // Single workgroup for indirect args generation
        compute_pass.dispatch_workgroups(1, 1, 1);
    }
}
