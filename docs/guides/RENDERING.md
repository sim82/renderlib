# Rendering in Renderlib

## Forward Rendering

Forward rendering is the simplest rendering technique where each object is rendered in a single pass with lighting calculated in the fragment shader.

```
Mesh Data → Vertex Shader → Fragment Shader → Framebuffer
```

**In renderlib:**
- Used in `forward.rs` example
- Simple to implement
- Good for scenes with few light sources

## Deferred Rendering

Deferred rendering separates geometry processing from lighting calculation using a G-buffer:

```
Phase 1 (Geometry Pass):
Mesh Data → Vertex Shader → Fragment Shader → G-Buffer (Position, Normal, Albedo)

Phase 2 (Lighting Pass):
Full-screen Quad → Sample G-Buffer → Calculate Lighting → Framebuffer
```

**In renderlib:**
- Used in `deferred.rs` and `deferred_with_camera_controls.rs` examples
- Uses [`GBuffer`] from the `deferred` module
- Efficient for scenes with many light sources
- More memory usage due to G-buffer storage

## Custom Pipelines

Create custom rendering pipelines using [`RenderPipelineBuilder`] from `device_helpers`:

```rust
use renderlib::device_helpers::RenderPipelineBuilder;

let pipeline = RenderPipelineBuilder::new(device)
    .with_shader_module(shader_module)
    .with_vertex_entry("vs_main")
    .with_fragment_entry("fs_main")
    .with_vertex_buffers(vec![vertex_layout])
    .with_color_formats(vec![surface_format])
    .build()?;
```

## Multiple Render Passes

For techniques like deferred rendering, use multiple render passes:

```rust
// Geometry pass
let mut geometry_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &gbuffer.color_attachments(),
    depth_stencil_attachment: Some(depth_attachment),
    ..Default::default()
});
// Draw meshes...

// Lighting pass
let mut lighting_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &[Some(output_attachment)],
    ..Default::default()
});
// Draw full-screen quad...
```

## Performance Tips

- **Minimize draw calls**: Batch similar objects
- **Reuse pipelines**: Create pipelines once, reuse often
- **Efficient buffers**: Use appropriate buffer usages
- **Depth testing**: Enable for proper occlusion
- **Shader hot-reload**: Press 'R' to reload shaders during development
