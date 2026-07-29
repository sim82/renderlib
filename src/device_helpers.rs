//! Generic helper functions for wgpu device operations.
//!
//! These helpers provide ergonomic wrappers around common wgpu operations,
//! reducing boilerplate while maintaining full generality.

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
/// # Example
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
pub struct RenderPipelineBuilder<'a> {
    device: &'a wgpu::Device,
    label: Option<&'a str>,
    layout: Option<&'a wgpu::PipelineLayout>,
    shader_module: Option<&'a wgpu::ShaderModule>,
    vertex_entry: Option<&'a str>,
    fragment_entry: Option<&'a str>,
    vertex_buffers: Option<&'a [Option<wgpu::VertexBufferLayout<'a>>]>,
    color_format: Option<wgpu::TextureFormat>,
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

    /// Set the color format for the pipeline's target.
    pub fn with_color_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.color_format = Some(format);
        self
    }

    /// Set the primitive state.
    pub fn with_primitive(mut self, primitive: wgpu::PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    /// Build the render pipeline.
    pub fn build(self) -> wgpu::RenderPipeline {
        let shader_module = self.shader_module.expect("Shader module must be set");
        let color_format = self.color_format.expect("Color format must be set");
        let vertex_buffers = self.vertex_buffers.expect("Vertex buffers must be set");

        let vertex_state = wgpu::VertexState {
            module: shader_module,
            entry_point: self.vertex_entry,
            buffers: vertex_buffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        let fragment_state = wgpu::FragmentState {
            module: shader_module,
            entry_point: self.fragment_entry,
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: self.label,
                layout: self.layout,
                vertex: vertex_state,
                fragment: Some(fragment_state),
                primitive: self.primitive,
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
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
