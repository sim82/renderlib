//! Lighting system for deferred rendering.
//!
//! Provides a reusable component for managing lights and lighting passes
//! in deferred rendering scenarios.

use wgpu;

use crate::camera::{Light, LightingUniform};
use crate::device_helpers::{
    create_bind_group_auto, create_buffer, create_buffer_from_slice, create_shader_module,
    load_shader_source, BindGroupLayoutBuilder, RenderPipelineBuilder,
};

/// Maximum number of lights supported by the lighting system.
pub const MAX_LIGHTS: usize = 4;

/// Default lighting configuration with 4 colored lights.
pub fn create_default_lights() -> [Light; MAX_LIGHTS] {
    let mut lights = [Light::default(); MAX_LIGHTS];
    lights[0] = Light::new([2.0, 3.0, 4.0], [1.0, 1.0, 1.0]); // White light
    lights[1] = Light::new([-3.0, 2.0, 2.0], [1.0, 0.0, 0.0]); // Red light
    lights[2] = Light::new([0.0, -2.0, 3.0], [0.0, 0.0, 1.0]); // Blue light
    lights[3] = Light::new([0.0, 2.0, -3.0], [0.0, 1.0, 0.0]); // Green light
    lights
}

/// Lighting system for deferred rendering.
///
/// Manages lights, lighting uniforms, and the lighting render pipeline.
pub struct LightingSystem {
    /// Array of lights in the scene
    pub lights: [Light; MAX_LIGHTS],
    /// Number of active lights
    pub num_lights: u32,
    /// Surface format for the pipeline
    pub surface_format: wgpu::TextureFormat,
    /// Uniform buffer for lighting parameters
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group layout for lighting uniforms
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for lighting uniforms
    pub bind_group: wgpu::BindGroup,
    /// Render pipeline for lighting pass
    pub pipeline: wgpu::RenderPipeline,
    /// Path to the lighting shader
    pub shader_path: String,
    /// Vertex buffer for full-screen quad
    pub quad_vertex_buffer: wgpu::Buffer,
}

impl LightingSystem {
    /// Creates a new lighting system with default configuration.
    ///
    /// # Arguments
    ///
    /// * `device` - The WGPU device to create resources with
    /// * `gbuffer_bind_group_layout` - Bind group layout for G-buffer textures
    /// * `surface_format` - Surface texture format for the pipeline
    /// * `shader_path` - Path to the lighting shader file
    ///
    /// # Returns
    ///
    /// A new `LightingSystem` ready for use, or an error if creation fails.
    pub fn new(
        device: &wgpu::Device,
        gbuffer_bind_group_layout: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
        shader_path: &str,
    ) -> Result<Self, String> {
        // Create default lights
        let lights = create_default_lights();
        let num_lights = lights.len() as u32;

        // Create quad vertex buffer for full-screen rendering
        let quad_vertex_buffer = Self::create_quad_vertex_buffer(device);

        // Create lighting uniform buffer
        let camera = crate::camera::Camera::default(); // Temporary camera for initialization
        let lighting_uniform_init = LightingUniform::new_with_lights(&camera, &lights);
        let uniform_buffer = create_buffer(
            device,
            Some("Lighting Uniform Buffer"),
            &lighting_uniform_init,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Create lighting uniform bind group layout
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label(Some("Lighting Uniform Bind Group Layout"))
            .with_uniform_buffer(wgpu::ShaderStages::FRAGMENT, None)
            .build();

        // Create lighting uniform bind group
        let bind_group = crate::device_helpers::create_bind_group_auto(
            device,
            Some("Lighting Uniform Bind Group"),
            &bind_group_layout,
            &[uniform_buffer.as_entire_binding()],
        );

        // Load shader and create pipeline
        let shader_src = load_shader_source(shader_path)
            .unwrap_or_else(|_| panic!("Failed to load lighting shader from: {}", shader_path));
        let pipeline = Self::create_pipeline(
            device,
            gbuffer_bind_group_layout,
            &bind_group_layout,
            surface_format,
            &shader_src,
        )?;

        Ok(Self {
            lights,
            num_lights,
            uniform_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            surface_format,
            shader_path: shader_path.to_string(),
            quad_vertex_buffer,
        })
    }

    /// Creates the vertex buffer for full-screen quad rendering.
    fn create_quad_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        use crate::mesh::quad_vertices_2d;
        create_buffer_from_slice(
            device,
            Some("Lighting Quad Vertex Buffer"),
            &quad_vertices_2d(),
            wgpu::BufferUsages::VERTEX,
        )
    }

    /// Creates the lighting render pipeline.
    fn create_pipeline(
        device: &wgpu::Device,
        gbuffer_bind_group_layout: &wgpu::BindGroupLayout,
        lighting_uniform_bind_group_layout: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
        shader_src: &str,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_module =
            create_shader_module(device, Some("Deferred Lighting Shader"), shader_src);

        let pipeline_layout = crate::device_helpers::create_pipeline_layout(
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
            .with_vertex_buffers(&[Some(crate::mesh::QuadVertex::desc())])
            .with_color_formats(&[surface_format.add_srgb_suffix()])
            .with_blend_states(&[None])
            .with_depth_stencil(None)
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

    /// Reloads the lighting shader.
    ///
    /// # Arguments
    ///
    /// * `device` - The WGPU device to create the new pipeline with
    /// * `gbuffer_bind_group_layout` - Bind group layout for G-buffer textures
    ///
    /// # Returns
    ///
    /// `Ok(())` if reload succeeded, or an error if it failed.
    pub fn reload_shader(
        &mut self,
        device: &wgpu::Device,
        gbuffer_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<(), String> {
        let shader_src = load_shader_source(&self.shader_path).unwrap_or_else(|_| {
            panic!(
                "Failed to reload lighting shader from: {}",
                self.shader_path
            )
        });
        self.pipeline = Self::create_pipeline(
            device,
            gbuffer_bind_group_layout,
            &self.bind_group_layout,
            self.surface_format,
            &shader_src,
        )?;
        Ok(())
    }

    /// Updates the lighting uniform buffer with current camera and light data.
    ///
    /// # Arguments
    ///
    /// * `queue` - The WGPU queue to use for buffer updates
    /// * `camera` - The current camera for view-projection matrix
    pub fn update_uniforms(&self, queue: &wgpu::Queue, camera: &crate::camera::Camera) {
        let lighting_uniform =
            LightingUniform::new_with_lights(camera, &self.lights[..self.num_lights as usize]);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting_uniform]),
        );
    }

    /// Renders the lighting pass.
    ///
    /// # Arguments
    ///
    /// * `encoder` - The command encoder to record commands into
    /// * `device` - The WGPU device (used for creating bind groups)
    /// * `gbuffer` - The G-buffer to read from
    /// * `texture_view` - The target texture view to render to
    pub fn render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        gbuffer: &crate::deferred::GBuffer,
        texture_view: &wgpu::TextureView,
    ) {
        // Create G-buffer bind group for this frame
        let gbuffer_bind_group = create_bind_group_auto(
            device,
            Some("GBuffer Bind Group"),
            &gbuffer.bind_group_layout,
            &[
                wgpu::BindingResource::TextureView(&gbuffer.position_view),
                wgpu::BindingResource::TextureView(&gbuffer.normal_view),
                wgpu::BindingResource::TextureView(&gbuffer.albedo_view),
                wgpu::BindingResource::Sampler(&gbuffer.sampler),
            ],
        );

        let mut lighting_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Lighting Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: texture_view,
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
        lighting_pass.set_pipeline(&self.pipeline);
        lighting_pass.set_bind_group(0, &gbuffer_bind_group, &[]);
        lighting_pass.set_bind_group(1, &self.bind_group, &[]);
        lighting_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        lighting_pass.draw(0..6, 0..1);
    }
}
