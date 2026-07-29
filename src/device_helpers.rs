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

/// Generic helper to create a render pipeline with full configuration
pub fn create_render_pipeline<'a>(
    device: &wgpu::Device,
    label: Option<&str>,
    layout: Option<&wgpu::PipelineLayout>,
    shader_module: &'a wgpu::ShaderModule,
    vertex_entry: Option<&'a str>,
    fragment_entry: Option<&'a str>,
    vertex_buffers: &[Option<wgpu::VertexBufferLayout<'a>>],
    color_format: wgpu::TextureFormat,
    primitive: wgpu::PrimitiveState,
) -> wgpu::RenderPipeline {
    let vertex_state = wgpu::VertexState {
        module: shader_module,
        entry_point: vertex_entry,
        buffers: vertex_buffers,
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    };

    let fragment_state = wgpu::FragmentState {
        module: shader_module,
        entry_point: fragment_entry,
        targets: &[Some(wgpu::ColorTargetState {
            format: color_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    };

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label,
        layout,
        vertex: vertex_state,
        fragment: Some(fragment_state),
        primitive,
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
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
