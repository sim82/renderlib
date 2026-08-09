//! Advanced demo: 3D spinning cube with colored sides, perspective projection, and fake lighting.
//!
//! This example demonstrates a 3D cube rendering with:
//! - Perspective projection
//! - Model-view-projection matrix
//! - Colored faces
//! - Simple fake lighting (pseudo light source at camera position)
//! - Smooth rotation on multiple axes

use std::time::Instant;

use cgmath::{perspective, Deg, Matrix4, Point3, Rad, SquareMatrix, Vector3};
use wgpu::VertexBufferLayout;
use winit::event_loop::EventLoop;

use renderlib::app::{App, AppRenderer};
use renderlib::context::GraphicsContext;
use renderlib::device_helpers::*;

/// Vertex data for a 3D cube with position, color, and normal.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex {
    /// Get the vertex buffer layout description for this vertex type.
    pub fn desc() -> VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Uniform data containing the model-view-projection matrix, model matrix, and light position.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub light_pos: [f32; 3],
    pub _padding: f32,
}

/// WGSL shader source for the 3D cube with MVP transformation and lighting.
const SHADER_SRC: &str = r#"
    struct Uniforms {
        mvp: mat4x4<f32>,
        model: mat4x4<f32>,
        light_pos: vec3<f32>,
        _padding: f32,
    };

    struct VertexInput {
        @location(0) position: vec3<f32>,
        @location(1) color: vec3<f32>,
        @location(2) normal: vec3<f32>,
    };

    struct VertexOutput {
        @builtin(position) clip_position: vec4<f32>,
        @location(0) color: vec3<f32>,
        @location(1) normal: vec3<f32>,
        @location(2) world_pos: vec3<f32>,
    };

    @group(0) @binding(0)
    var<uniform> uniforms: Uniforms;

    @vertex
    fn vs_main(
        model: VertexInput,
    ) -> VertexOutput {
        var out: VertexOutput;
        out.clip_position = uniforms.mvp * vec4<f32>(model.position, 1.0);
        out.color = model.color;
        let normal_matrix = mat3x3<f32>(
            uniforms.model[0].xyz,
            uniforms.model[1].xyz,
            uniforms.model[2].xyz
        );
        out.normal = normal_matrix * model.normal;
        out.world_pos = (uniforms.model * vec4<f32>(model.position, 1.0)).xyz;
        return out;
    }

    @fragment
    fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
        // Calculate lighting
        let light_dir = normalize(uniforms.light_pos - in.world_pos);
        let normal = normalize(in.normal);

        // Diffuse lighting factor (dot product of normal and light direction)
        let diffuse = max(dot(normal, light_dir), 0.0);

        // Ambient + diffuse lighting
        let ambient = 0.2;
        let lighting = ambient + diffuse * 0.8;

        // Apply lighting to the face color
        return vec4<f32>(in.color * lighting, 1.0);
    }
"#;

/// Define the 8 vertices of a cube centered at origin with side length 2.
/// Each face has a different color and proper normals.
fn create_cube_vertices() -> (Vec<Vertex>, Vec<u16>) {
    // Define the 8 corner positions
    let positions = [
        // Front face (z = 1)
        [-1.0, -1.0, 1.0], // 0
        [1.0, -1.0, 1.0],  // 1
        [1.0, 1.0, 1.0],   // 2
        [-1.0, 1.0, 1.0],  // 3
        // Back face (z = -1)
        [-1.0, -1.0, -1.0], // 4
        [1.0, -1.0, -1.0],  // 5
        [1.0, 1.0, -1.0],   // 6
        [-1.0, 1.0, -1.0],  // 7
    ];

    // Define face normals (pointing outward)
    let face_normals = [
        [0.0, 0.0, 1.0],  // Front face - Z+
        [0.0, 0.0, -1.0], // Back face - Z-
        [1.0, 0.0, 0.0],  // Right face - X+
        [-1.0, 0.0, 0.0], // Left face - X-
        [0.0, 1.0, 0.0],  // Top face - Y+
        [0.0, -1.0, 0.0], // Bottom face - Y-
    ];

    // Define colors for each face
    let face_colors = [
        [1.0, 0.0, 0.0], // Front - Red
        [0.0, 1.0, 0.0], // Back - Green
        [0.0, 0.0, 1.0], // Right - Blue
        [1.0, 1.0, 0.0], // Left - Yellow
        [1.0, 0.0, 1.0], // Top - Magenta
        [0.0, 1.0, 1.0], // Bottom - Cyan
    ];

    // Build vertices with colors and normals assigned per face
    let mut vertices = Vec::new();

    // Front face (z = 1)
    for &pos_idx in &[0, 1, 2, 3] {
        vertices.push(Vertex {
            position: positions[pos_idx],
            color: face_colors[0],
            normal: face_normals[0],
        });
    }
    // Back face (z = -1) - note: vertex order reversed for correct normal orientation
    for &pos_idx in &[5, 4, 7, 6] {
        vertices.push(Vertex {
            position: positions[pos_idx],
            color: face_colors[1],
            normal: face_normals[1],
        });
    }
    // Right face (x = 1)
    for &pos_idx in &[1, 5, 6, 2] {
        vertices.push(Vertex {
            position: positions[pos_idx],
            color: face_colors[2],
            normal: face_normals[2],
        });
    }
    // Left face (x = -1) - note: vertex order reversed
    for &pos_idx in &[4, 0, 3, 7] {
        vertices.push(Vertex {
            position: positions[pos_idx],
            color: face_colors[3],
            normal: face_normals[3],
        });
    }
    // Top face (y = 1)
    for &pos_idx in &[3, 2, 6, 7] {
        vertices.push(Vertex {
            position: positions[pos_idx],
            color: face_colors[4],
            normal: face_normals[4],
        });
    }
    // Bottom face (y = -1) - note: vertex order reversed
    for &pos_idx in &[0, 4, 5, 1] {
        vertices.push(Vertex {
            position: positions[pos_idx],
            color: face_colors[5],
            normal: face_normals[5],
        });
    }

    // Indices for each face (2 triangles per face, 6 faces = 36 indices)
    let mut indices = Vec::new();
    for face_offset in 0..6 {
        let base = face_offset * 4;
        // First triangle
        indices.push(base as u16);
        indices.push((base + 1) as u16);
        indices.push((base + 2) as u16);
        // Second triangle
        indices.push(base as u16);
        indices.push((base + 2) as u16);
        indices.push((base + 3) as u16);
    }

    (vertices, indices)
}

/// Renderer for the 3D spinning cube demo with lighting.
pub struct CubeRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    start_time: Instant,
}

impl AppRenderer for CubeRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Create cube vertices and indices
        let (vertices, indices) = create_cube_vertices();

        // Create vertex buffer
        let vertex_buffer = create_buffer_from_slice(
            device,
            Some("Cube Vertex Buffer"),
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );

        // Create index buffer
        let index_buffer = create_buffer_from_slice(
            device,
            Some("Cube Index Buffer"),
            &indices,
            wgpu::BufferUsages::INDEX,
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

        // Create bind group layout - needs to be accessible from both vertex and fragment shaders
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

        // Create shader module
        let shader_module = create_shader_module(device, Some("Cube Shader"), SHADER_SRC);

        // Create render pipeline layout
        let render_pipeline_layout = create_pipeline_layout(
            device,
            Some("Cube Render Pipeline Layout"),
            &[Some(&bind_group_layout)],
        );

        // Create render pipeline with back-face culling
        let render_pipeline = RenderPipelineBuilder::new(device)
            .with_label(Some("Cube Render Pipeline"))
            .with_layout(Some(&render_pipeline_layout))
            .with_shader_module(&shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(&[Some(Vertex::desc())])
            .with_color_format(context.surface_format.add_srgb_suffix())
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            })
            .build();

        CubeRenderer {
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            render_pipeline,
            start_time: Instant::now(),
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Calculate MVP and model matrices
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Create model matrix with rotation
        let model =
            Matrix4::from_angle_y(Rad(elapsed * 0.5)) * Matrix4::from_angle_x(Rad(elapsed * 0.3));

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

        // Clear the screen and draw the cube
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
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // Draw the cube
        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        renderpass.draw_indexed(0..36, 0, 0..1);

        // End the renderpass
        drop(renderpass);

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
    }

    fn resize(&mut self, _context: &mut GraphicsContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        // Resize is handled by the aspect ratio in the projection matrix
    }
}

fn main() {
    // Initialize logger
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // Use Poll control flow for smooth animation
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::<CubeRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
