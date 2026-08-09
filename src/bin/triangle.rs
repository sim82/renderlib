//! Self-contained triangle demo binary.
//!
//! This example demonstrates a rotating triangle using the renderlib framework.

use std::time::Instant;

use cgmath::SquareMatrix;
use wgpu::VertexBufferLayout;
use winit::event_loop::EventLoop;

use renderlib::app::{App, AppRenderer};
use renderlib::context::GraphicsContext;
use renderlib::device_helpers::*;

/// Vertex data for a triangle with position and color.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
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
            ],
        }
    }
}

/// Uniform data containing the rotation matrix.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub rotation: [[f32; 4]; 4],
}

/// WGSL shader source for the rotating triangle.
const SHADER_SRC: &str = r#"
    struct Uniforms {
        rotation: mat4x4<f32>,
    };

    struct VertexInput {
        @location(0) position: vec3<f32>,
        @location(1) color: vec3<f32>,
    };

    struct VertexOutput {
        @builtin(position) clip_position: vec4<f32>,
        @location(0) color: vec3<f32>,
    };

    @group(0) @binding(0)
    var<uniform> uniforms: Uniforms;

    @vertex
    fn vs_main(
        model: VertexInput,
    ) -> VertexOutput {
        var out: VertexOutput;
        out.clip_position = uniforms.rotation * vec4<f32>(model.position, 1.0);
        out.color = model.color;
        return out;
    }

    @fragment
    fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
        return vec4<f32>(in.color, 1.0);
    }
"#;

/// Triangle vertices with red, green, blue colors.
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];

/// Renderer for the rotating triangle demo.
pub struct TriangleRenderer {
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    start_time: Instant,
}

impl AppRenderer for TriangleRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Create vertex buffer from slice
        let vertex_buffer = create_buffer_from_slice(
            device,
            Some("Vertex Buffer"),
            VERTICES,
            wgpu::BufferUsages::VERTEX,
        );

        // Create uniform buffer from single struct
        let uniform_init = Uniforms {
            rotation: cgmath::Matrix4::<f32>::identity().into(),
        };
        let uniform_buffer = create_buffer(
            device,
            Some("Uniform Buffer"),
            &uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create bind group layout and bind group using helpers
        let bind_group_layout = create_uniform_bind_group_layout(
            device,
            Some("Uniform Bind Group Layout"),
            wgpu::ShaderStages::VERTEX,
        );

        let uniform_bind_group = create_uniform_bind_group(
            device,
            Some("Uniform Bind Group"),
            &bind_group_layout,
            &uniform_buffer,
        );

        // Create shader module using helper
        let shader_module = create_shader_module(device, Some("Shader"), SHADER_SRC);

        // Create render pipeline using helpers
        let render_pipeline_layout = create_pipeline_layout(
            device,
            Some("Render Pipeline Layout"),
            &[Some(&bind_group_layout)],
        );

        let render_pipeline = RenderPipelineBuilder::new(device)
            .with_label(Some("Render Pipeline"))
            .with_layout(Some(&render_pipeline_layout))
            .with_shader_module(&shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(&[Some(Vertex::desc())])
            .with_color_format(context.surface_format.add_srgb_suffix())
            .with_primitive(wgpu::PrimitiveState::default())
            .build();

        TriangleRenderer {
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group,
            render_pipeline,
            start_time: Instant::now(),
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Calculate rotation based on elapsed time
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let rotation_matrix = cgmath::Matrix4::from_angle_z(cgmath::Rad(elapsed));

        // Update uniform buffer with new rotation matrix
        context.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms {
                rotation: rotation_matrix.into(),
            }]),
        );

        // Get current surface texture and view from context
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let texture_view = context.create_texture_view(&surface_texture);

        // Clear the screen and draw the triangle
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

        // Draw the triangle
        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.draw(0..3, 0..1);

        // End the renderpass
        drop(renderpass);

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
    }

    fn resize(&mut self, _context: &mut GraphicsContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        // Demo doesn't need special resize handling beyond what GraphicsContext does
    }
}

fn main() {
    // Initialize logger
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // Use Poll control flow for games that want to render as fast as possible
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::<TriangleRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
