//! Deferred rendering demo with GLTF mesh loading.
//!
//! Combines GLTF mesh loading with deferred shading pipeline:
//! - Loads a GLTF mesh from disk
//! - Geometry pass: renders mesh to G-buffer (position, normal, albedo)
//! - Lighting pass: full-screen quad that reads G-buffer and computes lighting
//! - Auto-scales and centers the mesh
//! - Press R to reload shaders

use std::time::Instant;

use cgmath::{perspective, Deg, Matrix4, Point3, Rad, SquareMatrix, Vector3};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{App, AppRenderer};
use renderlib::context::GraphicsContext;
use renderlib::device_helpers::*;
use renderlib::geometry::{primitives, PosColorNormalVertex};

/// Paths to the shader files.
const GEOMETRY_SHADER_PATH: &str = "src/shaders/cube_deferred_geometry.wgsl";
const LIGHTING_SHADER_PATH: &str = "src/shaders/cube_deferred_lighting.wgsl";

/// Default GLTF file path.
const GLTF_PATH: &str = "assets/duck.glb";

/// G-buffer texture format for deferred rendering.
/// Each texture stores one component of the deferred data:
/// - Position: world space position (RGB, A unused)
/// - Normal: world space normal (RGB, A unused)
/// - Albedo: surface color (RGB, A unused)
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
    /// Create a new G-buffer with the given dimensions.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, label_prefix: Option<&str>) -> Self {
        let texture_format = wgpu::TextureFormat::Rgba16Float;
        let texture_usage = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST;

        // Create position texture
        let position_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: label_prefix.map(|p| format!("{}_Position", p)).as_deref(),
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
            label: label_prefix.map(|p| format!("{}_Normal", p)).as_deref(),
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
            label: label_prefix.map(|p| format!("{}_Albedo", p)).as_deref(),
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
            label: label_prefix.map(|p| format!("{}_Sampler", p)).as_deref(),
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

        // Recreate textures with new size
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
    fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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

    /// Create a bind group for this G-buffer with the given layout.
    pub fn create_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GBuffer Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
}

/// Vertex with just position for full-screen quad (2D coordinates).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
}

impl QuadVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

/// Full-screen quad vertices using 2D positions.
const QUAD_VERTICES_2D: &[QuadVertex] = &[
    // First triangle
    QuadVertex {
        position: [0.0, 0.0],
    },
    QuadVertex {
        position: [1.0, 0.0],
    },
    QuadVertex {
        position: [0.0, 1.0],
    },
    // Second triangle
    QuadVertex {
        position: [1.0, 0.0],
    },
    QuadVertex {
        position: [1.0, 1.0],
    },
    QuadVertex {
        position: [0.0, 1.0],
    },
];

/// Uniform data for the geometry pass (MVP and model matrices).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryUniforms {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
}

/// Uniform data for the lighting pass (camera and light positions).
/// Uses std140 layout: each vec3 occupies 16 bytes, f32 occupies 16 bytes with padding
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniforms {
    pub view_position: [f32; 4],  // vec3 padded to vec4 for 16-byte alignment
    pub light_position: [f32; 4], // vec3 padded to vec4 for 16-byte alignment
}

/// Vertex data loaded from GLTF mesh.
struct GltfMesh {
    vertices: Vec<PosColorNormalVertex>,
    indices: Vec<u16>,
    /// Scale factor to normalize the model to approximately unit size.
    scale: f32,
    /// Center point of the mesh (bounding box center) to translate to origin.
    center: Vector3<f32>,
}

/// Renderer for deferred rendering demo with GLTF mesh.
pub struct GltfDeferredRenderer {
    // GLTF mesh resources
    mesh_vertex_buffer: wgpu::Buffer,
    mesh_index_buffer: wgpu::Buffer,
    num_indices: u32,

    // Geometry pass resources
    geometry_uniform_buffer: wgpu::Buffer,
    geometry_bind_group_layout: wgpu::BindGroupLayout,
    geometry_bind_group: wgpu::BindGroup,
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

    // G-buffer
    gbuffer: GBuffer,

    // Pipeline state
    surface_format: wgpu::TextureFormat,

    // Mesh transforms
    model_scale: f32,
    mesh_center: Vector3<f32>,

    // Hot-reload state
    should_reload_geometry: bool,
    should_reload_lighting: bool,

    // Timing
    start_time: Instant,
}

impl GltfDeferredRenderer {
    /// Load shader source from file.
    fn load_shader_source(path: &str) -> Result<String, String> {
        std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read shader file {}: {}", path, e))
    }

    /// Loads a GLTF file and extracts the first mesh's vertex and index data.
    fn load_gltf_mesh(path: &str) -> Result<GltfMesh, String> {
        // Use the gltf::import function which loads document + buffers + images
        let (document, buffers, _images) = if path.ends_with(".glb") {
            let data = std::fs::read(path)
                .map_err(|e| format!("Failed to read GLB file '{}': {}", path, e))?;
            gltf::import_slice(&data)
                .map_err(|e| format!("Failed to import GLB '{}': {}", path, e))?
        } else {
            gltf::import(path).map_err(|e| format!("Failed to import GLTF '{}': {}", path, e))?
        };

        let mesh = document
            .meshes()
            .next()
            .ok_or("No meshes found in GLTF file".to_string())?;

        // First pass: collect all positions to calculate bounding box
        let mut all_positions: Vec<[f32; 3]> = Vec::new();
        let mut all_indices: Vec<u16> = Vec::new();
        let mut vertex_offset: u32 = 0;

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|b| &*b.0));
            let positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(positions) => positions.collect(),
                None => return Err("Mesh primitive has no POSITION attribute".to_string()),
            };

            let num_positions = positions.len();
            all_positions.extend(positions);

            let primitive_indices: Vec<u16> = match reader.read_indices() {
                Some(indices) => {
                    use gltf::mesh::util::ReadIndices;
                    match indices {
                        ReadIndices::U8(iter) => iter.map(|x| x as u16).collect(),
                        ReadIndices::U16(iter) => iter.collect(),
                        ReadIndices::U32(iter) => iter.map(|x| x as u16).collect(),
                    }
                }
                None => (0..num_positions as u16).collect(),
            };

            let offset_indices: Vec<u16> = primitive_indices
                .iter()
                .map(|&idx| idx.saturating_add(vertex_offset as u16))
                .collect();
            all_indices.extend(offset_indices);
            vertex_offset += num_positions as u32;
        }

        // Calculate scale factor and center based on bounding box
        let (scale, center) = if !all_positions.is_empty() {
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut min_z = f32::INFINITY;
            let mut max_z = f32::NEG_INFINITY;

            for &[x, y, z] in &all_positions {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            }

            let width = max_x - min_x;
            let height = max_y - min_y;
            let depth = max_z - min_z;
            let max_dim = width.max(height).max(depth);

            // Scale to approximately fit in a 2x2x2 box (similar to the cube demo)
            let scale = if max_dim > 0.0 && max_dim.is_finite() {
                2.0 / max_dim
            } else {
                1.0
            };

            // Center is the midpoint of the bounding box
            let center = Vector3::new(
                (min_x + max_x) / 2.0,
                (min_y + max_y) / 2.0,
                (min_z + max_z) / 2.0,
            );

            (scale, center)
        } else {
            (1.0, Vector3::new(0.0, 0.0, 0.0))
        };

        // Second pass: build vertices with normals and color
        let mut all_vertices: Vec<PosColorNormalVertex> = Vec::new();

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|b| &*b.0));

            let positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(positions) => positions.collect(),
                None => return Err("Mesh primitive has no POSITION attribute".to_string()),
            };

            let normals: Vec<[f32; 3]> = match reader.read_normals() {
                Some(normals) => normals.collect(),
                None => {
                    vec![[0.0, 1.0, 0.0]; positions.len()]
                }
            };

            let default_color: [f32; 3] = [0.8, 0.8, 0.8];
            let mut mesh_vertices = Vec::new();
            for (i, position) in positions.iter().enumerate() {
                let normal = normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
                mesh_vertices.push(PosColorNormalVertex {
                    position: *position,
                    color: default_color,
                    normal,
                });
            }

            all_vertices.extend(mesh_vertices);
        }

        if all_vertices.is_empty() {
            return Err("No vertices loaded from GLTF mesh".to_string());
        }

        Ok(GltfMesh {
            vertices: all_vertices,
            indices: all_indices,
            scale,
            center,
        })
    }

    /// Create the geometry pass pipeline.
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

        // Create pipeline with multiple color attachments for G-buffer
        let color_attachment_formats = [
            wgpu::TextureFormat::Rgba16Float, // Position
            wgpu::TextureFormat::Rgba16Float, // Normal
            wgpu::TextureFormat::Rgba16Float, // Albedo
        ];

        let color_targets: Vec<Option<wgpu::ColorTargetState>> = color_attachment_formats
            .iter()
            .map(|&format| {
                Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })
            })
            .collect();

        let fragment_state = wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            targets: &color_targets,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        // Enable depth testing for geometry pass
        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Deferred Geometry Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[Some(PosColorNormalVertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(fragment_state),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

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
            .with_color_format(surface_format.add_srgb_suffix())
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
        let shader_src = Self::load_shader_source(&self.geometry_shader_path)?;
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
        let shader_src = Self::load_shader_source(&self.lighting_shader_path)?;
        let gbuffer_bind_group_layout = GBuffer::bind_group_layout(device);
        self.lighting_pipeline = Self::create_lighting_pipeline(
            device,
            &gbuffer_bind_group_layout,
            &self.lighting_uniform_bind_group_layout,
            self.surface_format,
            &shader_src,
        )?;
        self.should_reload_lighting = false;
        Ok(())
    }

    /// Creates a depth texture and view for depth testing.
    fn create_depth_texture(
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
}

impl AppRenderer for GltfDeferredRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;
        let size = context.size;

        // Load shaders
        let geometry_shader_src =
            Self::load_shader_source(GEOMETRY_SHADER_PATH).expect("Failed to load geometry shader");
        let lighting_shader_src =
            Self::load_shader_source(LIGHTING_SHADER_PATH).expect("Failed to load lighting shader");

        // Load GLTF mesh, or fall back to cube if file doesn't exist
        let (vertices, indices, model_scale, mesh_center) = match Self::load_gltf_mesh(GLTF_PATH) {
            Ok(mesh) => {
                eprintln!(
                    "Loaded GLTF mesh: {} vertices, {} indices, scale: {:.2}, center: ({:.2}, {:.2}, {:.2})",
                    mesh.vertices.len(),
                    mesh.indices.len(),
                    mesh.scale,
                    mesh.center.x,
                    mesh.center.y,
                    mesh.center.z
                );
                (mesh.vertices, mesh.indices, mesh.scale, mesh.center)
            }
            Err(e) => {
                eprintln!("Failed to load GLTF mesh: {}", e);
                eprintln!("Falling back to hardcoded cube.");
                eprintln!("To use a custom GLTF/GLB file, place it at '{}'", GLTF_PATH);
                let (v, i) = primitives::cube_vertices();
                (v, i, 1.0, Vector3::new(0.0, 0.0, 0.0))
            }
        };
        let num_indices = indices.len() as u32;

        // Create mesh buffers
        let mesh_vertex_buffer = create_buffer_from_slice(
            device,
            Some("GLTF Vertex Buffer"),
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );

        let mesh_index_buffer = create_buffer_from_slice(
            device,
            Some("GLTF Index Buffer"),
            &indices,
            wgpu::BufferUsages::INDEX,
        );

        // Create quad buffer for lighting pass
        let quad_vertex_buffer = create_buffer_from_slice(
            device,
            Some("Quad Vertex Buffer"),
            QUAD_VERTICES_2D,
            wgpu::BufferUsages::VERTEX,
        );

        // Create G-buffer
        let gbuffer = GBuffer::new(device, size.width, size.height, Some("Deferred"));

        // Create depth texture for geometry pass
        let (depth_texture, depth_texture_view) = Self::create_depth_texture(
            device,
            size.width,
            size.height,
            Some("GLTF Deferred Depth Texture"),
        );

        // Create geometry pass uniform buffer
        let geometry_uniform_init = GeometryUniforms {
            mvp: Matrix4::<f32>::identity().into(),
            model: Matrix4::<f32>::identity().into(),
        };
        let geometry_uniform_buffer = create_buffer(
            device,
            Some("Geometry Uniform Buffer"),
            &geometry_uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create geometry bind group layout and bind group
        let geometry_bind_group_layout = create_uniform_bind_group_layout(
            device,
            Some("Geometry Uniform Bind Group Layout"),
            wgpu::ShaderStages::VERTEX,
        );

        let geometry_bind_group = create_uniform_bind_group(
            device,
            Some("Geometry Uniform Bind Group"),
            &geometry_bind_group_layout,
            &geometry_uniform_buffer,
        );

        // Create geometry pipeline
        let geometry_pipeline = Self::create_geometry_pipeline(
            device,
            &geometry_bind_group_layout,
            context.surface_format,
            &geometry_shader_src,
        )
        .expect("Failed to create geometry pipeline");

        // Create lighting pass uniform buffer
        let lighting_uniform_init = LightingUniforms {
            view_position: [0.0, 0.0, 5.0, 0.0],
            light_position: [2.0, 3.0, 4.0, 0.0],
        };
        let lighting_uniform_buffer = create_buffer(
            device,
            Some("Lighting Uniform Buffer"),
            &lighting_uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create lighting uniform bind group layout and bind group
        let lighting_uniform_bind_group_layout = create_uniform_bind_group_layout(
            device,
            Some("Lighting Uniform Bind Group Layout"),
            wgpu::ShaderStages::FRAGMENT,
        );

        let lighting_uniform_bind_group = create_uniform_bind_group(
            device,
            Some("Lighting Uniform Bind Group"),
            &lighting_uniform_bind_group_layout,
            &lighting_uniform_buffer,
        );

        // Create lighting pipeline
        let lighting_pipeline = Self::create_lighting_pipeline(
            device,
            &gbuffer.bind_group_layout,
            &lighting_uniform_bind_group_layout,
            context.surface_format,
            &lighting_shader_src,
        )
        .expect("Failed to create lighting pipeline");

        GltfDeferredRenderer {
            mesh_vertex_buffer,
            mesh_index_buffer,
            num_indices,
            geometry_uniform_buffer,
            geometry_bind_group_layout,
            geometry_bind_group,
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
            surface_format: context.surface_format,
            model_scale,
            mesh_center,
            should_reload_geometry: false,
            should_reload_lighting: false,
            start_time: Instant::now(),
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Handle shader reload
        if self.should_reload_geometry {
            eprintln!("Reloading geometry shader...");
            if let Err(e) = self.reload_geometry_shader(&context.device) {
                eprintln!("Geometry shader reload failed: {}", e);
            } else {
                eprintln!("Geometry shader reloaded successfully!");
            }
        }

        if self.should_reload_lighting {
            eprintln!("Reloading lighting shader...");
            if let Err(e) = self.reload_lighting_shader(&context.device) {
                eprintln!("Lighting shader reload failed: {}", e);
            } else {
                eprintln!("Lighting shader reloaded successfully!");
            }
        }

        // Resize G-buffer and depth texture if needed
        if self.gbuffer.width != context.size.width || self.gbuffer.height != context.size.height {
            self.gbuffer
                .resize(&context.device, context.size.width, context.size.height);

            // Recreate depth texture with new size
            let (depth_texture, depth_texture_view) = Self::create_depth_texture(
                &context.device,
                context.size.width,
                context.size.height,
                Some("GLTF Deferred Depth Texture"),
            );
            self.depth_texture = depth_texture;
            self.depth_texture_view = depth_texture_view;
        }

        // Calculate matrices
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Create model matrix with translation, scaling, and rotation
        // In column-major matrices (cgmath), transformations are applied right-to-left
        let translation = Matrix4::from_translation(-self.mesh_center);
        let scale_matrix = Matrix4::from_scale(self.model_scale);
        let model = Matrix4::from_angle_y(Rad(elapsed * 0.5))
            * Matrix4::from_angle_x(Rad(elapsed * 0.3))
            * scale_matrix
            * translation;

        // Create view matrix (camera looking at origin from (0, 0, 5))
        let eye = Point3::new(0.0, 0.0, 5.0);
        let target = Point3::new(0.0, 0.0, 0.0);
        let up = Vector3::new(0.0, 1.0, 0.0);
        let view = Matrix4::look_at_rh(eye, target, up);

        // Create perspective projection matrix
        let aspect = context.size.width as f32 / context.size.height as f32;
        let proj = perspective::<f32, Deg<f32>>(Deg(45.0), aspect, 0.1, 100.0);

        // MVP = projection * view * model
        let mvp = proj * view * model;

        // Update geometry uniform buffer
        context.queue.write_buffer(
            &self.geometry_uniform_buffer,
            0,
            bytemuck::cast_slice(&[GeometryUniforms {
                mvp: mvp.into(),
                model: model.into(),
            }]),
        );

        // Update lighting uniform buffer
        context.queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[LightingUniforms {
                view_position: [0.0, 0.0, 5.0, 0.0],
                light_position: [2.0, 3.0, 4.0, 0.0],
            }]),
        );

        // Get current surface texture
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let surface_view = context.create_texture_view(&surface_texture);

        // Create command encoder
        let mut encoder = context.device.create_command_encoder(&Default::default());

        // Create G-buffer bind group for lighting pass
        let gbuffer_bind_group = self.gbuffer.create_bind_group(&context.device);

        // =====================================================================
        // GEOMETRY PASS: Render mesh to G-buffer
        // =====================================================================
        {
            let gbuffer_color_attachments = [
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.gbuffer.position_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.gbuffer.normal_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.gbuffer.albedo_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ];

            let mut geometry_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Geometry Pass"),
                color_attachments: &gbuffer_color_attachments,
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

            // Draw mesh
            geometry_pass.set_pipeline(&self.geometry_pipeline);
            geometry_pass.set_bind_group(0, &self.geometry_bind_group, &[]);
            geometry_pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
            geometry_pass
                .set_index_buffer(self.mesh_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            geometry_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // =====================================================================
        // LIGHTING PASS: Full-screen quad that reads G-buffer and computes lighting
        // =====================================================================
        {
            let mut lighting_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Lighting Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
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

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
    }

    fn resize(&mut self, context: &mut GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>) {
        // Resize G-buffer
        self.gbuffer
            .resize(&context.device, new_size.width, new_size.height);

        // Recreate depth texture with new size
        let (depth_texture, depth_texture_view) = Self::create_depth_texture(
            &context.device,
            new_size.width,
            new_size.height,
            Some("GLTF Deferred Depth Texture"),
        );
        self.depth_texture = depth_texture;
        self.depth_texture_view = depth_texture_view;
    }

    fn input(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
        {
            if let Key::Character(c) = &key_event.logical_key {
                if c.to_ascii_lowercase() == "r" && key_event.state.is_pressed() {
                    // Reload both shaders
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

    let mut app = App::<GltfDeferredRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
