//! Deferred rendering module.
//!
//! Provides G-buffer management for deferred shading. A G-buffer stores geometry
//! data (position, normal, albedo) in separate textures for later lighting computation.

use crate::device_helpers::create_bind_group_auto;

/// G-buffer for deferred rendering.
///
/// Contains three textures (position, normal, albedo) and a sampler for lighting pass access.
#[derive(Debug)]
pub struct GBuffer {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub position_texture: wgpu::Texture,
    pub normal_texture: wgpu::Texture,
    pub albedo_texture: wgpu::Texture,
    pub position_view: wgpu::TextureView,
    pub normal_view: wgpu::TextureView,
    pub albedo_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

impl GBuffer {
    /// Creates a new G-buffer with the given dimensions.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, label_prefix: Option<&str>) -> Self {
        let texture_format = wgpu::TextureFormat::Rgba16Float;
        let texture_usage = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST;

        // Create position texture
        let position_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: label_prefix.map(|p| format!("{} Position", p)).as_deref(),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: texture_usage,
            view_formats: &[texture_format],
        });

        // Create normal texture
        let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: label_prefix.map(|p| format!("{} Normal", p)).as_deref(),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: texture_usage,
            view_formats: &[texture_format],
        });

        // Create albedo texture
        let albedo_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: label_prefix.map(|p| format!("{} Albedo", p)).as_deref(),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: texture_usage,
            view_formats: &[texture_format],
        });

        // Create texture views
        let position_view = position_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let albedo_view = albedo_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler for G-buffer sampling
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: label_prefix.map(|p| format!("{} Sampler", p)).as_deref(),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        GBuffer {
            bind_group_layout: Self::bind_group_layout(device),
            position_texture,
            normal_texture,
            albedo_texture,
            position_view,
            normal_view,
            albedo_view,
            sampler,
            width,
            height,
        }
    }

    /// Resize the G-buffer to new dimensions.
    pub fn resize(&mut self, device: &wgpu::Device, new_width: u32, new_height: u32) {
        self.width = new_width;
        self.height = new_height;

        let texture_format = wgpu::TextureFormat::Rgba16Float;
        let texture_usage = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST;

        self.position_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer Position Texture"),
            size: wgpu::Extent3d {
                width: new_width,
                height: new_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: texture_usage,
            view_formats: &[texture_format],
        });

        self.normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer Normal Texture"),
            size: wgpu::Extent3d {
                width: new_width,
                height: new_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: texture_usage,
            view_formats: &[texture_format],
        });

        self.albedo_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer Albedo Texture"),
            size: wgpu::Extent3d {
                width: new_width,
                height: new_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: texture_usage,
            view_formats: &[texture_format],
        });

        self.position_view = self
            .position_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.normal_view = self
            .normal_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.albedo_view = self
            .albedo_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    /// Create a bind group layout for accessing this G-buffer.
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GBuffer Bind Group Layout"),
            entries: &[
                // Position texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Normal texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Albedo texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    /// Create a bind group for this G-buffer with the given device.
    pub fn create_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        create_bind_group_auto(
            device,
            Some("GBuffer Bind Group"),
            &self.bind_group_layout,
            &[
                wgpu::BindingResource::TextureView(&self.position_view),
                wgpu::BindingResource::TextureView(&self.normal_view),
                wgpu::BindingResource::TextureView(&self.albedo_view),
                wgpu::BindingResource::Sampler(&self.sampler),
            ],
        )
    }

    /// Returns the texture formats used by the G-buffer color attachments.
    pub fn color_formats() -> [wgpu::TextureFormat; 3] {
        [
            wgpu::TextureFormat::Rgba16Float, // Position
            wgpu::TextureFormat::Rgba16Float, // Normal
            wgpu::TextureFormat::Rgba16Float, // Albedo
        ]
    }

    /// Creates color target states for the G-buffer render pass.
    pub fn color_targets() -> [wgpu::ColorTargetState; 3] {
        [
            wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            },
            wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            },
            wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            },
        ]
    }

    /// Creates render pass color attachments for this G-buffer's texture views.
    ///
    /// This is useful for setting up a geometry pass render pass that writes to the G-buffer.
    /// Each attachment will use Clear(Black) as the load operation.
    pub fn color_attachments(&self) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 3] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: &self.position_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.normal_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.albedo_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
        ]
    }
}
