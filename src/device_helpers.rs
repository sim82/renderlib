//! Generic helper functions for wgpu device operations.
//!
//! These helpers provide ergonomic wrappers around common wgpu operations,
//! reducing boilerplate while maintaining full generality.

use std::path::Path;

use wgpu::util::DeviceExt;

/// Generic helper to create a buffer from any Pod type
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

/// Generic helper to create a buffer from a slice of Pod types
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

/// Load shader source code from a file.
///
/// # Arguments
///
/// * `path` - Path to the shader file (WGSL format)
///
/// # Returns
///
/// The shader source code as a string, or an error if the file cannot be read or is empty.
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
/// Supports both single-color and multi-color (e.g., deferred rendering) pipelines.
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
    /// Single color format (for simple pipelines) - takes precedence if both single and multi are set
    color_format: Option<wgpu::TextureFormat>,
    /// Multiple color formats (for deferred rendering, etc.)
    color_formats: Option<&'a [wgpu::TextureFormat]>,
    /// Blend state per color attachment. If None, defaults to REPLACE for single, None for multi.
    blend_states: Option<&'a [Option<wgpu::BlendState>]>,
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
            color_format: None,
            color_formats: None,
            blend_states: None,
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

    /// Set a single color format for the pipeline's target.
    /// This is the simplest way for pipelines with one color attachment.
    pub fn with_color_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.color_format = Some(format);
        self
    }

    /// Set multiple color formats for pipelines with multiple render targets.
    /// This is useful for deferred rendering, where you might have separate
    /// targets for position, normal, and albedo.
    pub fn with_color_formats(mut self, formats: &'a [wgpu::TextureFormat]) -> Self {
        self.color_formats = Some(formats);
        self
    }

    /// Set blend states for each color attachment.
    /// Each element corresponds to a color target.
    /// Use `None` for no blending, or `Some(blend_state)` for custom blending.
    ///
    /// If not set, defaults to `Some(BlendState::REPLACE)` for single-color pipelines,
    /// and `None` for multi-color pipelines.
    pub fn with_blend_states(mut self, states: &'a [Option<wgpu::BlendState>]) -> Self {
        self.blend_states = Some(states);
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
    /// - Neither single color format nor multiple color formats are set
    pub fn build(self) -> wgpu::RenderPipeline {
        let shader_module = self.shader_module.expect("Shader module must be set");
        let vertex_buffers = self.vertex_buffers.expect("Vertex buffers must be set");

        let vertex_state = wgpu::VertexState {
            module: shader_module,
            entry_point: self.vertex_entry,
            buffers: vertex_buffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        // Build color targets based on single or multi-color configuration
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
        // Single color format takes precedence if set (backward compatibility)
        if let Some(format) = self.color_format {
            let blend = self
                .blend_states
                .and_then(|states| states.first().copied())
                .unwrap_or(Some(wgpu::BlendState::REPLACE));
            return vec![Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })];
        }

        // Multiple color formats
        let formats = self.color_formats.expect(
            "Either color_format or color_formats must be set. Use with_color_format() for single target or with_color_formats() for multiple targets."
        );

        // Determine blend states to use
        let blend_states: Vec<Option<wgpu::BlendState>> = if let Some(states) = self.blend_states {
            states.to_vec()
        } else {
            // Default: no blending for multi-target pipelines (e.g., deferred rendering)
            vec![None; formats.len()]
        };

        formats
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

/// Generic helper to create a bind group layout for a uniform buffer
pub fn create_uniform_bind_group_layout(
    device: &wgpu::Device,
    label: Option<&str>,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// Generic helper to create a bind group for a uniform buffer
pub fn create_uniform_bind_group(
    device: &wgpu::Device,
    label: Option<&str>,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
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
