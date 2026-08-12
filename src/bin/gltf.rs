//! GLTF mesh loading demo: Loads and renders a GLTF mesh instead of a cube.
//!
//! This demo demonstrates:
//! - Loading a GLTF mesh from disk using the `import` feature
//! - Extracting vertex positions and normals
//! - Rendering with perspective projection and lighting
//! - Smooth rotation of the loaded mesh
//!
//! Press R to reload shaders.

use std::time::Instant;

use cgmath::{perspective, Deg, Matrix4, Point3, Rad, SquareMatrix, Vector3};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::Key;

use renderlib::app::{App, AppRenderer};
use renderlib::context::GraphicsContext;
use renderlib::device_helpers::*;
use renderlib::geometry::{primitives, PosColorNormalVertex};

/// Path to the shader file (reuse cube shader).
const SHADER_PATH: &str = "src/shaders/cube.wgsl";

/// Default GLTF file path.
/// The model should be a GLTF 2.0 (.gltf/.glb) file with at least POSITION attributes.
/// NORMAL attributes are optional (will use defaults if missing).
const GLTF_PATH: &str = "assets/duck.glb";

/// Uniform data containing the model-view-projection matrix, model matrix, and light position.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub light_pos: [f32; 3],
    pub _padding: f32,
}

/// Vertex data loaded from GLTF mesh.
/// We use PosColorNormalVertex with a default color for all vertices.
struct GltfMesh {
    vertices: Vec<PosColorNormalVertex>,
    indices: Vec<u16>,
    /// Scale factor to normalize the model to approximately unit size.
    scale: f32,
    /// Center point of the mesh (bounding box center) to translate to origin.
    center: cgmath::Vector3<f32>,
}

/// Renderer for the GLTF mesh demo with lighting.
pub struct GltfRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    should_reload: bool,
    start_time: Instant,
    /// Scale factor to normalize model size (computed from bounding box).
    model_scale: f32,
    /// Translation to center the mesh at origin (computed from bounding box center).
    mesh_center: cgmath::Vector3<f32>,
}

impl GltfRenderer {
    /// Loads a GLTF file and extracts the first mesh's vertex and index data.
    /// Returns vertices with a default color (light gray), indices, and a scale factor.
    fn load_gltf_mesh(path: &str) -> Result<GltfMesh, String> {
        // Use the gltf::import function which loads document + buffers + images
        let (document, buffers, _images) = if path.ends_with(".glb") {
            // For .glb files, read as bytes first
            let data = std::fs::read(path)
                .map_err(|e| format!("Failed to read GLB file '{}': {}", path, e))?;
            gltf::import_slice(&data)
                .map_err(|e| format!("Failed to import GLB '{}': {}", path, e))?
        } else {
            // For .gltf files, import from path (handles external buffers)
            gltf::import(path).map_err(|e| format!("Failed to import GLTF '{}': {}", path, e))?
        };

        // Get the first mesh
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
            let center = cgmath::Vector3::new(
                (min_x + max_x) / 2.0,
                (min_y + max_y) / 2.0,
                (min_z + max_z) / 2.0,
            );

            (scale, center)
        } else {
            (1.0, cgmath::Vector3::new(0.0, 0.0, 0.0))
        };

        // Second pass: build vertices with normals and color
        let mut all_vertices: Vec<PosColorNormalVertex> = Vec::new();
        let mut vertex_offset: u32 = 0;

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|b| &*b.0));

            // Read positions (required)
            let positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(positions) => positions.collect(),
                None => return Err("Mesh primitive has no POSITION attribute".to_string()),
            };

            // Read normals (optional, default to upward if missing)
            let normals: Vec<[f32; 3]> = match reader.read_normals() {
                Some(normals) => normals.collect(),
                None => {
                    // Generate default normals (pointing up)
                    vec![[0.0, 1.0, 0.0]; positions.len()]
                }
            };

            // Read indices (optional)
            let primitive_indices: Vec<u16> = match reader.read_indices() {
                Some(indices) => {
                    use gltf::mesh::util::ReadIndices;
                    match indices {
                        ReadIndices::U8(iter) => iter.map(|x| x as u16).collect(),
                        ReadIndices::U16(iter) => iter.collect(),
                        ReadIndices::U32(iter) => iter.map(|x| x as u16).collect(),
                    }
                }
                None => {
                    // Generate sequential indices if not present
                    (0..positions.len() as u16).collect()
                }
            };

            // Create vertices with default color (light gray)
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

            // Adjust indices with vertex offset (for multi-primitive meshes)
            let offset_indices: Vec<u16> = primitive_indices
                .iter()
                .map(|&idx| idx.saturating_add(vertex_offset as u16))
                .collect();

            all_vertices.extend(mesh_vertices);
            all_indices.extend(offset_indices);
            vertex_offset += positions.len() as u32;
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

    /// Creates the render pipeline from the shader file with depth testing enabled.
    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_src = std::fs::read_to_string(SHADER_PATH)
            .map_err(|e| format!("Failed to read shader file: {}", e))?;

        let shader_module = create_shader_module(device, Some("GLTF Shader"), &shader_src);

        let render_pipeline_layout = create_pipeline_layout(
            device,
            Some("GLTF Render Pipeline Layout"),
            &[Some(bind_group_layout)],
        );

        // Enable depth testing
        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GLTF Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[Some(PosColorNormalVertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format.add_srgb_suffix(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
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

    /// Reloads the shader from disk and recreates the pipeline.
    pub fn reload_shader(&mut self, device: &wgpu::Device) -> Result<(), String> {
        self.render_pipeline =
            Self::create_pipeline(device, &self.bind_group_layout, self.surface_format)?;
        self.should_reload = false;
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

impl AppRenderer for GltfRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Load GLTF mesh, or fall back to cube if file doesn't exist
        let (vertices, indices, model_scale, mesh_center) = match GltfRenderer::load_gltf_mesh(
            GLTF_PATH,
        ) {
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
                // Fall back to the hardcoded cube from primitives
                let (v, i) = primitives::cube_vertices();
                (v, i, 1.0, cgmath::Vector3::new(0.0, 0.0, 0.0)) // Cube is already centered
            }
        };
        let num_indices = indices.len() as u32;

        // Create vertex buffer
        let vertex_buffer = create_buffer_from_slice(
            device,
            Some("GLTF Vertex Buffer"),
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );

        // Create index buffer
        let index_buffer = create_buffer_from_slice(
            device,
            Some("GLTF Index Buffer"),
            &indices,
            wgpu::BufferUsages::INDEX,
        );

        // Create depth texture for depth testing
        let (depth_texture, depth_texture_view) = Self::create_depth_texture(
            device,
            context.size.width,
            context.size.height,
            Some("GLTF Depth Texture"),
        );

        // Create uniform buffer for MVP, model matrix, and light position
        let uniform_init = Uniforms {
            mvp: Matrix4::<f32>::identity().into(),
            model: Matrix4::<f32>::identity().into(),
            light_pos: [0.0, 0.0, 5.0], // Light at camera position
            _padding: 0.0,
        };
        let uniform_buffer = create_buffer(
            device,
            Some("MVP Uniform Buffer"),
            &uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create initial pipeline
        let render_pipeline =
            Self::create_pipeline(device, &bind_group_layout, context.surface_format)
                .expect("Failed to create initial pipeline");

        GltfRenderer {
            vertex_buffer,
            index_buffer,
            num_indices,
            uniform_buffer,
            uniform_bind_group,
            render_pipeline,
            bind_group_layout,
            surface_format: context.surface_format,
            depth_texture,
            depth_texture_view,
            should_reload: false,
            start_time: Instant::now(),
            model_scale,
            mesh_center,
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Reload shader if requested
        if self.should_reload {
            eprintln!("Reloading GLTF shader...");
            if let Err(e) = self.reload_shader(&context.device) {
                eprintln!("Shader reload failed: {}", e);
            } else {
                eprintln!("GLTF shader reloaded successfully!");
            }
        }

        // Calculate MVP and model matrices
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Create model matrix with translation, scaling, and rotation
        // In column-major matrices (cgmath), transformations are applied right-to-left:
        // M = R * S * T means vertex is transformed as M * v = R * S * T * v
        // So T (translation) is applied first, then S (scale), then R (rotation)
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
        let proj = perspective::<f32, Deg<f32>>(
            Deg(45.0),
            aspect,
            0.1,   // near plane
            100.0, // far plane
        );

        // MVP = projection * view * model
        let mvp = proj * view * model;

        // Light position (at camera)
        let light_pos = [0.0, 0.0, 5.0];

        // Update uniform buffer
        context.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms {
                mvp: mvp.into(),
                model: model.into(),
                light_pos,
                _padding: 0.0,
            }]),
        );

        // Get current surface texture
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let texture_view = context.create_texture_view(&surface_texture);

        // Clear the screen and draw the mesh
        let mut encoder = context.device.create_command_encoder(&Default::default());

        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
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

        // Draw the mesh
        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        renderpass.draw_indexed(0..self.num_indices, 0, 0..1);

        // End the renderpass
        drop(renderpass);

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
    }

    fn resize(&mut self, context: &mut GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>) {
        // Recreate depth texture with new size
        let (depth_texture, depth_texture_view) = Self::create_depth_texture(
            &context.device,
            new_size.width,
            new_size.height,
            Some("GLTF Depth Texture"),
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
                    self.should_reload = true;
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

    let mut app = App::<GltfRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
