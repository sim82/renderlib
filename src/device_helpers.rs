//! Device helper functions for common wgpu operations.
//!
//! Provides ergonomic wrappers around common wgpu operations to reduce boilerplate.

use std::path::Path;

use cgmath::Vector3;
use wgpu::util::DeviceExt;

/// Creates a buffer initialized with data.
pub fn create_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    data: &T,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label,
        contents: bytemuck::cast_slice(std::slice::from_ref(data)),
        usage,
    })
}

/// Creates a buffer initialized with slice data.
pub fn create_buffer_from_slice<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    data: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label,
        contents: bytemuck::cast_slice(data),
        usage,
    })
}

/// Loads shader source code from a file.
///
/// # Arguments
///
/// * `path` - Path to the shader file (WGSL format)
///
/// # Returns
///
/// The shader source code as a string, or an error if the file cannot be read.
pub fn load_shader_source<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let path_str = path.as_ref().display().to_string();
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read shader file {}: {}", path_str, e))?;
    if source.is_empty() {
        return Err(format!("Shader file {} is empty", path_str));
    }
    Ok(source)
}

/// Generic helper to create a shader module from WGSL source
pub fn create_shader_module(
    device: &wgpu::Device,
    label: Option<&str>,
    wgsl_source: &str,
) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label,
        source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
    })
}

/// Generic helper to create a pipeline layout from bind group layouts
pub fn create_pipeline_layout(
    device: &wgpu::Device,
    label: Option<&str>,
    bind_group_layouts: &[Option<&wgpu::BindGroupLayout>],
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label,
        bind_group_layouts,
        immediate_size: 0,
    })
}

/// Builder for creating render pipelines with a fluent API.
///
/// Supports both single-color and multi-color (e.g., deferred rendering) pipelines
/// through a unified `with_color_formats()` method.
///
/// # Example - Single Color Attachment
///
/// ```ignore
/// let pipeline = RenderPipelineBuilder::new(&device)
///     .with_label("My Pipeline")
///     .with_layout(&pipeline_layout)
///     .with_shader_module(&shader_module)
///     .with_vertex_entry("vs_main")
///     .with_fragment_entry("fs_main")
///     .with_vertex_buffers(&[Some(Vertex::desc())])
///     .with_color_format(surface_format.add_srgb_suffix())
///     .build();
/// ```
///
/// # Example - Multiple Color Attachments (Deferred Rendering)
///
/// ```ignore
/// let pipeline = RenderPipelineBuilder::new(&device)
///     .with_label("Deferred Geometry Pipeline")
///     .with_layout(&pipeline_layout)
///     .with_shader_module(&shader_module)
///     .with_vertex_entry("vs_main")
///     .with_fragment_entry("fs_main")
///     .with_vertex_buffers(&[Some(Vertex::desc())])
///     .with_color_formats(&[format1, format2, format3])
///     .with_blend_states(&[None, None, None])
///     .with_depth_stencil(Some(depth_stencil_state))
///     .build();
/// ```
pub struct RenderPipelineBuilder<'a> {
    device: &'a wgpu::Device,
    label: Option<&'a str>,
    layout: Option<&'a wgpu::PipelineLayout>,
    shader_module: Option<&'a wgpu::ShaderModule>,
    vertex_entry: Option<&'a str>,
    fragment_entry: Option<&'a str>,
    vertex_buffers: Option<&'a [Option<wgpu::VertexBufferLayout<'a>>]>,
    /// Color formats for the pipeline's color attachments.
    color_formats: Vec<wgpu::TextureFormat>,
    /// Blend state per color attachment. Defaults to None (no blending) if not set.
    blend_states: Vec<Option<wgpu::BlendState>>,
    /// Depth and stencil state
    depth_stencil: Option<wgpu::DepthStencilState>,
    primitive: wgpu::PrimitiveState,
}

impl<'a> RenderPipelineBuilder<'a> {
    /// Create a new builder for a render pipeline.
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self {
            device,
            label: None,
            layout: None,
            shader_module: None,
            vertex_entry: None,
            fragment_entry: None,
            vertex_buffers: None,
            color_formats: Vec::new(),
            blend_states: Vec::new(),
            depth_stencil: None,
            primitive: wgpu::PrimitiveState::default(),
        }
    }

    /// Set the pipeline label.
    pub fn with_label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }

    /// Set the pipeline layout.
    pub fn with_layout(mut self, layout: Option<&'a wgpu::PipelineLayout>) -> Self {
        self.layout = layout;
        self
    }

    /// Set the shader module.
    pub fn with_shader_module(mut self, module: &'a wgpu::ShaderModule) -> Self {
        self.shader_module = Some(module);
        self
    }

    /// Set the vertex shader entry point.
    pub fn with_vertex_entry(mut self, entry: &'a str) -> Self {
        self.vertex_entry = Some(entry);
        self
    }

    /// Set the fragment shader entry point.
    pub fn with_fragment_entry(mut self, entry: &'a str) -> Self {
        self.fragment_entry = Some(entry);
        self
    }

    /// Set the vertex buffer layouts.
    pub fn with_vertex_buffers(
        mut self,
        buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    ) -> Self {
        self.vertex_buffers = Some(buffers);
        self
    }

    /// Set color formats for pipelines with one or more render targets.
    /// For a single target, pass `&[format]`.
    /// For multiple targets (e.g., deferred rendering), pass the full array.
    pub fn with_color_formats(mut self, formats: &[wgpu::TextureFormat]) -> Self {
        self.color_formats = formats.to_vec();
        self
    }

    /// Set blend states for each color attachment.
    /// Each element corresponds to a color target.
    /// Use `None` for no blending, or `Some(blend_state)` for custom blending.
    /// If not set, defaults to `None` (no blending) for all attachments.
    pub fn with_blend_states(mut self, states: &[Option<wgpu::BlendState>]) -> Self {
        self.blend_states = states.to_vec();
        self
    }

    /// Set the depth and stencil state.
    /// This is required for pipelines that need depth testing (e.g., 3D rendering).
    pub fn with_depth_stencil(mut self, state: Option<wgpu::DepthStencilState>) -> Self {
        self.depth_stencil = state;
        self
    }

    /// Set the primitive state.
    pub fn with_primitive(mut self, primitive: wgpu::PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    /// Build the render pipeline.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Shader module is not set
    /// - Vertex buffers are not set
    /// - Color formats are not set
    pub fn build(self) -> wgpu::RenderPipeline {
        let shader_module = self.shader_module.expect("Shader module must be set");
        let vertex_buffers = self.vertex_buffers.expect("Vertex buffers must be set");

        let vertex_state = wgpu::VertexState {
            module: shader_module,
            entry_point: self.vertex_entry,
            buffers: vertex_buffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        let color_targets = self.build_color_targets();

        let fragment_state = wgpu::FragmentState {
            module: shader_module,
            entry_point: self.fragment_entry,
            targets: &color_targets,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: self.label,
                layout: self.layout,
                vertex: vertex_state,
                fragment: Some(fragment_state),
                primitive: self.primitive,
                depth_stencil: self.depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
    }

    /// Build color target states from the configured formats and blend states.
    fn build_color_targets(&self) -> Vec<Option<wgpu::ColorTargetState>> {
        if self.color_formats.is_empty() {
            panic!("Color formats must be set. Use with_color_format() or with_color_formats().");
        }

        // Use explicitly set blend states, or default to None (no blending) for all
        let blend_states: Vec<Option<wgpu::BlendState>> = if self.blend_states.is_empty() {
            vec![None; self.color_formats.len()]
        } else {
            self.blend_states.clone()
        };

        self.color_formats
            .iter()
            .zip(blend_states.iter())
            .map(|(&format, &blend)| {
                Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })
            })
            .collect()
    }
}

/// Generic helper to create a bind group with automatic binding numbering
///
/// Creates bind group entries with sequential binding numbers starting from 0.
/// Each resource in the slice gets assigned the next binding number automatically.
///
/// # Arguments
///
/// * `device` - The wgpu device
/// * `label` - Optional label for the bind group
/// * `layout` - The bind group layout
/// * `resources` - Slice of binding resources (texture views, samplers, buffers)
///
/// # Example
///
/// ```ignore
/// // For geometry bind group with camera and instance buffers
/// let geometry_bind_group = create_bind_group_auto(
///     device,
///     Some("Geometry Bind Group"),
///     &geometry_bind_group_layout,
///     &[
///         camera_uniform_buffer.as_entire_binding(),
///         instance_buffer.as_entire_binding(),
///     ],
/// );
///
/// // For GBuffer with textures and sampler
/// let gbuffer_bind_group = create_bind_group_auto(
///     context.wgpu_device(),
///     Some("GBuffer Bind Group"),
///     &self.gbuffer.bind_group_layout,
///     &[
///         wgpu::BindingResource::TextureView(&self.gbuffer.position_view),
///         wgpu::BindingResource::TextureView(&self.gbuffer.normal_view),
///         wgpu::BindingResource::TextureView(&self.gbuffer.albedo_view),
///         wgpu::BindingResource::Sampler(&self.gbuffer.sampler),
///     ],
/// );
/// ```
pub fn create_bind_group_auto(
    device: &wgpu::Device,
    label: Option<&str>,
    layout: &wgpu::BindGroupLayout,
    resources: &[wgpu::BindingResource],
) -> wgpu::BindGroup {
    let entries: Vec<wgpu::BindGroupEntry> = resources
        .iter()
        .enumerate()
        .map(|(binding, resource)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: resource.clone(),
        })
        .collect();

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout,
        entries: &entries,
    })
}

/// Builder for creating bind group layouts with a fluent API.
///
/// Supports automatic binding numbering starting from 0.
///
/// # Example
///
/// ```ignore
/// let layout = BindGroupLayoutBuilder::new(device)
///     .with_label("Geometry Bind Group Layout")
///     .with_uniform_buffer(wgpu::ShaderStages::VERTEX, Some(std::mem::size_of::<CameraUniform>() as u64))
///     .with_storage_buffer(wgpu::ShaderStages::VERTEX, true)
///     .build();
/// ```
pub struct BindGroupLayoutBuilder<'a> {
    device: &'a wgpu::Device,
    label: Option<&'a str>,
    entries: Vec<wgpu::BindGroupLayoutEntry>,
}

impl<'a> BindGroupLayoutBuilder<'a> {
    /// Create a new builder for a bind group layout.
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self {
            device,
            label: None,
            entries: Vec::new(),
        }
    }

    /// Set the layout label.
    pub fn with_label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }

    /// Add a uniform buffer entry with automatic binding numbering.
    pub fn with_uniform_buffer(
        mut self,
        visibility: wgpu::ShaderStages,
        min_binding_size: Option<u64>,
    ) -> Self {
        let binding = self.entries.len() as u32;
        let wgpu_min_binding_size = min_binding_size.and_then(|size| wgpu::BufferSize::new(size));
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu_min_binding_size,
            },
            count: None,
        });
        self
    }

    /// Add a storage buffer entry with automatic binding numbering.
    pub fn with_storage_buffer(mut self, visibility: wgpu::ShaderStages, read_only: bool) -> Self {
        let binding = self.entries.len() as u32;
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    /// Add a texture entry with automatic binding numbering.
    pub fn with_texture(
        mut self,
        visibility: wgpu::ShaderStages,
        sample_type: wgpu::TextureSampleType,
        view_dimension: wgpu::TextureViewDimension,
    ) -> Self {
        let binding = self.entries.len() as u32;
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled: false,
            },
            count: None,
        });
        self
    }

    /// Add a sampler entry with automatic binding numbering.
    pub fn with_sampler(mut self, visibility: wgpu::ShaderStages) -> Self {
        let binding = self.entries.len() as u32;
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        self
    }

    /// Build the bind group layout.
    pub fn build(self) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: self.label,
                entries: &self.entries,
            })
    }
}

/// Creates a depth texture and view for depth testing.
///
/// # Arguments
///
/// * `device` - The wgpu device to create resources with
/// * `width` - The width of the texture
/// * `height` - The height of the texture
/// * `label` - Optional label for the texture
///
/// # Returns
///
/// A tuple of (depth_texture, depth_texture_view) ready for use in render passes.
pub fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: Option<&str>,
) -> (wgpu::Texture, wgpu::TextureView) {
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor {
        label: label.map(|s| format!("{} View", s)).as_deref(),
        format: Some(wgpu::TextureFormat::Depth32Float),
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::DepthOnly,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
        usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
    });

    (depth_texture, depth_texture_view)
}

/// Generates positions in an expanding cubic grid.
///
/// Creates a 3D grid of positions centered at the origin with the specified spacing.
/// The grid dimensions are calculated as the cube root of the count, rounded up.
///
/// # Arguments
///
/// * `count` - Number of positions to generate
/// * `spacing` - Distance between adjacent positions in world units
///
/// # Returns
///
/// Vector of 3D positions arranged in a cubic grid
pub fn generate_expanding_grid_positions(count: usize, spacing: f32) -> Vec<Vector3<f32>> {
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
