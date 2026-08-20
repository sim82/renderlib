# Renderlib Examples Documentation

This document describes the example applications included with renderlib, explaining their purpose, implementation, and what you can learn from each.

## Table of Contents

1. [Overview](#1-overview)
2. [Triangle Demo](#2-triangle-demo)
3. [Forward Rendering Demo](#3-forward-rendering-demo)
4. [Deferred Rendering Demo](#4-deferred-rendering-demo)
5. [Running the Examples](#5-running-the-examples)
6. [Learning from the Examples](#6-learning-from-the-examples)
7. [Creating Your Own Examples](#7-creating-your-own-examples)

---

## 1. Overview

Renderlib includes three main example applications that demonstrate different aspects of the framework:

| Example | File | Complexity | Focus Area |
|---------|------|------------|------------|
| Triangle | `src/bin/triangle.rs` | ⭐ | Basic rendering, shader hot-reload |
| Forward | `src/bin/forward.rs` | ⭐⭐⭐ | Mesh loading, lighting, camera |
| Deferred | `src/bin/deferred.rs` | ⭐⭐⭐⭐ | Deferred rendering, G-buffer, multi-pass |

Each example builds on the previous one, adding more features and complexity.

---

## 2. Triangle Demo

**File:** `src/bin/triangle.rs`  
**Complexity:** Beginner  
**Focus:** Basic rendering pipeline, shader hot-reloading

### Purpose

The triangle demo is the simplest example, demonstrating:
- Basic application structure
- Simple rendering pipeline
- Shader hot-reloading
- Uniform buffer usage

### Features

- Renders a single colored triangle
- Triangle rotates over time
- Press 'R' to reload shaders
- Uses `PosColorVertex` for vertex data

### Code Structure

```rust
// Main components
struct TriangleRenderer {
    vertex_buffer: wgpu::Buffer,           // Triangle vertices
    uniform_buffer: wgpu::Buffer,          // Rotation matrix
    uniform_bind_group: wgpu::BindGroup,   // Bind group for uniforms
    render_pipeline: wgpu::RenderPipeline, // Rendering pipeline
    bind_group_layout: wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    should_reload: bool,                   // Shader reload flag
    start_time: Instant,                   // For animation timing
}

// Shader
const SHADER_PATH: &str = "src/shaders/triangle.wgsl";

// Uniform data
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    rotation: [[f32; 4]; 4],  // Rotation matrix
}
```

### Key Concepts Demonstrated

1. **AppRenderer Trait Implementation**
   - `init()`: Creates buffers, pipeline, and loads shader
   - `render()`: Updates uniforms, draws triangle
   - `input()`: Handles 'R' key for shader reload

2. **Simple Rendering Pipeline**
   - Vertex shader: Applies rotation matrix
   - Fragment shader: Outputs vertex color
   - Single render pass

3. **Uniform Buffers**
   - Rotation matrix updated every frame
   - Written to GPU buffer with `queue.write_buffer()`

4. **Shader Hot-Reloading**
   - `load_shader_source()` reads shader from file
   - `reload_shader()` recreates pipeline with new shader
   - Flag-based reload (checked at start of render)

### Shader (triangle.wgsl)

```wgsl
// Vertex shader
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.rotation * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
```

### What You Can Learn

✅ Basic renderlib application structure  
✅ Creating vertex buffers  
✅ Using uniform buffers  
✅ Creating render pipelines  
✅ Shader hot-reloading  
✅ Simple animation (rotation over time)  
✅ Event handling (keyboard input)  

---

## 3. Forward Rendering Demo

**File:** `src/bin/forward.rs`  
**Complexity:** Intermediate  
**Focus:** Mesh loading, lighting, camera, depth testing

### Purpose

The forward rendering demo builds on the triangle example, adding:
- Mesh loading (GLTF or built-in cube)
- 3D camera with perspective projection
- Multiple lights with diffuse lighting
- Depth testing for proper occlusion
- Automatic mesh scaling and centering

### Features

- Loads GLTF/GLB mesh from `assets/duck.glb` (falls back to cube)
- Auto-scales and centers mesh based on bounding box
- Smooth rotation animation
- Multiple colored lights
- Depth buffer for occlusion
- Press 'R' to reload shaders

### Code Structure

```rust
struct ForwardRenderer {
    // Mesh resources
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    
    // Uniform buffers
    geometry_uniform_buffer: wgpu::Buffer,    // MVP and model matrices
    lighting_uniform_buffer: wgpu::Buffer,   // View position and lights
    
    // Pipeline
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    
    // Depth buffer
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    
    // Animation and state
    should_reload: bool,
    start_time: Instant,
    model_scale: f32,
    mesh_center: Vector3<f32>,
    camera: Camera,
    lights: [Light; MAX_LIGHTS],
    num_lights: u32,
}
```

### Key Concepts Demonstrated

1. **Mesh Loading**
   - `load_gltf()`: Loads mesh from file
   - Fallback to `primitives::cube_vertices()` if file not found
   - Automatic bounding box calculation
   - Scale and center for proper positioning

2. **Camera System**
   - `Camera::new()`: Creates default camera
   - `get_view_matrix()`: World to view space
   - `get_projection_matrix()`: Perspective projection
   - `get_view_projection_matrix()`: Combined MVP

3. **Lighting**
   - `Light::new()`: Creates point light
   - `LightingUniform::new_with_lights()`: Creates lighting uniform
   - Diffuse lighting calculation in shader
   - Multiple lights (up to MAX_LIGHTS = 32)

4. **Depth Testing**
   - `create_depth_texture()`: Creates depth buffer
   - Depth stencil state with `CompareFunction::Less`
   - Clears depth buffer every frame

5. **Geometry Uniform**
   - `GeometryUniform::new()`: Creates MVP + model matrices
   - Separates camera matrices from model matrix
   - Supports animated transformations

### Shader (forward.wgsl)

```wgsl
// Uniforms
@group(0) @binding(0)
var<uniform> geometry: GeometryUniform;

@group(0) @binding(1)
var<uniform> lighting: LightingUniform;

// Vertex input
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

// Vertex output
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let world_pos = vec3<f32>((geometry.model * vec4<f32>(in.position, 1.0)).xyz);
    return VertexOutput(
        geometry.view_projection * vec4<f32>(world_pos, 1.0),
        in.color,
        world_pos,
        mat3<f32>(geometry.model) * in.normal
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let view_dir = normalize(lighting.view_position.xyz - in.world_position);
    
    // Diffuse lighting
    let mut diffuse = vec3<f32>(0.0);
    for (var i: u32 = 0; i < lighting.num_lights; i = i + 1) {
        let light_pos = lighting.lights[i].position.xyz;
        let light_color = lighting.lights[i].color.xyz;
        let light_dir = normalize(light_pos - in.world_position);
        diffuse = diffuse + light_color * max(dot(normal, light_dir), 0.0);
    }
    
    // Combine with base color and ambient
    let color = in.color * (diffuse + 0.1);
    return vec4<f32>(color, 1.0);
}
```

### What You Can Learn

✅ All concepts from Triangle Demo  
✅ Mesh loading and management  
✅ Camera system with projection  
✅ Multiple lights with diffuse lighting  
✅ Depth testing for occlusion  
✅ Normal transformation (world space)  
✅ Bind groups with multiple entries  
✅ Automatic mesh scaling and centering  
✅ Indexed rendering (draw_indexed)  

---

## 4. Deferred Rendering Demo

**File:** `src/bin/deferred.rs`  
**Complexity:** Advanced  
**Focus:** Deferred rendering, G-buffer, multi-pass rendering

### Purpose

The deferred rendering demo demonstrates the most advanced rendering technique in renderlib:
- Two-pass rendering (geometry + lighting)
- G-buffer management
- Screen-space lighting
- Multiple render targets
- Complex bind group organization

### Features

- Same mesh loading as forward demo (GLTF or cube)
- G-buffer with position, normal, and albedo textures
- Geometry pass: Renders mesh to G-buffer
- Lighting pass: Full-screen quad with G-buffer sampling
- Multiple lights with deferred shading
- Depth testing in geometry pass
- Press 'R' to reload both shaders

### Code Structure

```rust
struct DeferredRenderer {
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
    
    // G-buffer from framework
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
    
    // Camera
    camera: Camera,
    
    // Lighting
    lights: [Light; MAX_LIGHTS],
    num_lights: u32,
}
```

### Key Concepts Demonstrated

1. **G-Buffer Management**
   - `GBuffer::new()`: Creates position, normal, albedo textures
   - `GBuffer::resize()`: Handles window resize
   - `GBuffer::create_bind_group()`: For lighting pass
   - `GBuffer::color_attachments()`: For geometry pass

2. **Two-Pass Rendering**
   - Geometry pass: Renders to G-buffer
   - Lighting pass: Renders full-screen quad
   - Separate pipelines for each pass

3. **Multiple Render Targets**
   - Position, normal, albedo textures
   - All use `Rgba16Float` format
   - Cleared every frame in geometry pass

4. **Complex Bind Group Organization**
   - Geometry pass: 1 bind group (geometry uniforms)
   - Lighting pass: 2 bind groups (G-buffer + lighting uniforms)
   - G-buffer bind group: 4 bindings (3 textures + sampler)

5. **Screen-Space Lighting**
   - Samples G-buffer in fragment shader
   - Computes lighting per-pixel
   - Same lighting model as forward rendering

6. **Separate Shaders**
   - `deferred_geometry.wgsl`: Geometry pass shader
   - `deferred_lighting.wgsl`: Lighting pass shader
   - Both can be reloaded independently

### Geometry Pass Shader (deferred_geometry.wgsl)

```wgsl
// Uniforms
@group(0) @binding(0)
var<uniform> geometry: GeometryUniform;

// Vertex input
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

// Fragment output (3 render targets)
struct FragmentOutput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) albedo: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let world_pos = vec3<f32>((geometry.model * vec4<f32>(in.position, 1.0)).xyz);
    return VertexOutput(
        geometry.view_projection * vec4<f32>(world_pos, 1.0),
        in.color,
        world_pos,
        mat3<f32>(geometry.model) * in.normal
    );
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    return FragmentOutput(
        vec4<f32>(in.world_position, 1.0),    // Position
        vec4<f32>(normalize(in.world_normal), 0.0),  // Normal
        vec4<f32>(in.color, 1.0)             // Albedo
    );
}
```

### Lighting Pass Shader (deferred_lighting.wgsl)

```wgsl
// G-buffer textures
@group(0) @binding(0)
var position_texture: texture_2d<f32>;
@group(0) @binding(1)
var normal_texture: texture_2d<f32>;
@group(0) @binding(2)
var albedo_texture: texture_2d<f32>;
@group(0) @binding(3)
var sampler: sampler;

// Lighting uniforms
@group(1) @binding(0)
var<uniform> lighting: LightingUniform;

// Full-screen quad vertex input
struct VertexInput {
    @location(0) position: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    // Full-screen quad in NDC
    let uv = in.position * vec2<f32>(2.0, 2.0) - vec2<f32>(1.0, 1.0);
    return vec4<f32>(uv, 0.0, 1.0);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Sample G-buffer
    let position = textureSample(position_texture, sampler, uv).xyz;
    let normal = textureSample(normal_texture, sampler, uv).xyz;
    let albedo = textureSample(albedo_texture, sampler, uv).rgb;
    
    // Calculate lighting
    let n = normalize(normal);
    let mut color = vec3<f32>(0.0);
    
    for (var i: u32 = 0; i < lighting.num_lights; i = i + 1) {
        let light_pos = lighting.lights[i].position.xyz;
        let light_color = lighting.lights[i].color.xyz;
        let light_dir = normalize(light_pos - position);
        color = color + albedo * light_color * max(dot(n, light_dir), 0.0);
    }
    
    // Add ambient
    color = color + albedo * 0.1;
    
    return vec4<f32>(color, 1.0);
}
```

### Render Method Structure

```rust
fn render(&mut self, context: &mut GraphicsContext) {
    // 1. Handle shader reload
    if self.should_reload_geometry { /* ... */ }
    if self.should_reload_lighting { /* ... */ }
    
    // 2. Resize G-buffer if needed
    if self.gbuffer.width != context.size.width || self.gbuffer.height != context.size.height {
        self.gbuffer.resize(&context.device, context.size.width, context.size.height);
        // Recreate depth texture
    }
    
    // 3. Update uniforms
    // Update geometry uniform (MVP + model)
    // Update lighting uniform (camera + lights)
    
    // 4. Get surface texture
    let surface_texture = match context.get_current_texture() {
        Some(texture) => texture,
        None => return,
    };
    let surface_view = context.create_texture_view(&surface_texture);
    
    // 5. Create command encoder
    let mut encoder = context.device.create_command_encoder(&Default::default());
    
    // 6. GEOMETRY PASS
    {
        let mut geometry_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Geometry Pass"),
            color_attachments: &self.gbuffer.color_attachments(),
            depth_stencil_attachment: Some(/* depth attachment */),
            ..Default::default()
        });
        
        // Draw mesh
        geometry_pass.set_pipeline(&self.geometry_pipeline);
        geometry_pass.set_bind_group(0, &self.geometry_bind_group, &[]);
        geometry_pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
        geometry_pass.set_index_buffer(self.mesh_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        geometry_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
    
    // 7. LIGHTING PASS
    {
        let gbuffer_bind_group = self.gbuffer.create_bind_group(&context.device);
        
        let mut lighting_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Lighting Pass"),
            color_attachments: &[Some(/* surface attachment */)],
            ..Default::default()
        });
        
        // Draw full-screen quad
        lighting_pass.set_pipeline(&self.lighting_pipeline);
        lighting_pass.set_bind_group(0, &gbuffer_bind_group, &[]);
        lighting_pass.set_bind_group(1, &self.lighting_uniform_bind_group, &[]);
        lighting_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        lighting_pass.draw(0..6, 0..1);
    }
    
    // 8. Submit and present
    context.queue.submit([encoder.finish()]);
    context.pre_present_notify();
    context.queue.present(surface_texture);
}
```

### What You Can Learn

✅ All concepts from Forward Demo  
✅ Deferred rendering architecture  
✅ G-buffer management  
✅ Multi-pass rendering  
✅ Multiple render targets  
✅ Screen-space lighting  
✅ Complex bind group organization  
✅ Full-screen quad rendering  
✅ Separate shaders for different passes  
✅ Independent shader hot-reloading  

---

## 5. Running the Examples

### Basic Usage

Run any example with Cargo:

```bash
# Triangle demo
cargo run --bin triangle

# Forward rendering demo
cargo run --bin forward

# Deferred rendering demo
cargo run --bin deferred
```

### Release Mode

For better performance:

```bash
cargo run --release --bin forward
```

### With Logging

Enable debug logging:

```bash
RUST_LOG=debug cargo run --bin forward
```

### Available Log Levels

- `error`: Only errors
- `warn`: Warnings and errors
- `info`: General information
- `debug`: Detailed debugging
- `trace`: Very verbose (may impact performance)

### Keyboard Controls

All examples support:

| Key | Action |
|-----|--------|
| R | Reload shaders |

Forward and Deferred examples also support:

| Key | Action |
|-----|--------|
| W | Move camera forward |
| S | Move camera backward |
| A | Move camera left |
| D | Move camera right |

### Asset Files

The forward and deferred examples look for mesh files in the `assets/` directory:

```
assets/
└── duck.glb          # Default mesh (not included in repo)
```

If `assets/duck.glb` doesn't exist, the examples fall back to a built-in cube.

### Downloading Test Models

You can download free test models:

- **Duck.glb**: [KhronosGroup/glTF-Sample-Models](https://github.com/KhronosGroup/glTF-Sample-Models/raw/master/2.0/Duck/glTF-Embedded/Duck.glb)
- **Cube.glb**: Create your own or use any GLTF/GLB file
- **More models**: [Sketchfab](https://sketchfab.com/) (many free models available)

---

## 6. Learning from the Examples

### Progression Path

1. **Start with Triangle**
   - Understand basic structure
   - Learn simple rendering
   - See shader hot-reloading

2. **Move to Forward**
   - Add mesh loading
   - Implement camera
   - Add lighting
   - Use depth testing

3. **Study Deferred**
   - Understand multi-pass rendering
   - Learn G-buffer management
   - See complex bind groups
   - Implement screen-space effects

### Key Patterns to Notice

#### Resource Initialization

All examples follow the same initialization pattern in `init()`:

```rust
async fn init(context: &GraphicsContext) -> Self {
    // 1. Load assets (meshes, shaders)
    // 2. Create buffers (vertex, index, uniform)
    // 3. Create textures (depth, G-buffer)
    // 4. Create bind group layouts
    // 5. Create bind groups
    // 6. Create pipelines
    // 7. Return initialized struct
}
```

#### Render Loop Structure

All examples follow a similar render pattern:

```rust
fn render(&mut self, context: &mut GraphicsContext) {
    // 1. Handle shader reload if requested
    // 2. Update uniforms (matrices, lighting, etc.)
    // 3. Get surface texture
    // 4. Create command encoder
    // 5. Begin render pass(es)
    // 6. Set pipeline, bind groups, buffers
    // 7. Draw commands
    // 8. Submit and present
    // 9. Request next redraw
}
```

#### Error Handling

Examples use different error handling approaches:

- **Triangle**: Panics on errors (simple demo)
- **Forward**: Uses `expect()` with descriptive messages
- **Deferred**: Similar to forward, with more complex error handling

For production code, consider using `anyhow::Result` or `thiserror`.

#### Shader Organization

- **Triangle**: Inline shader string
- **Forward**: Loads from `src/shaders/forward.wgsl`
- **Deferred**: Separate shaders for geometry and lighting passes

### Comparison of Examples

| Feature | Triangle | Forward | Deferred |
|---------|----------|---------|----------|
| Vertex Count | 3 | Variable (mesh) | Variable (mesh) |
| Shaders | 1 | 1 | 2 |
| Render Passes | 1 | 1 | 2 |
| Bind Groups | 1 | 1 | 3 |
| Textures | 0 | 1 (depth) | 4 (G-buffer + depth) |
| Lights | 0 | Multiple | Multiple |
| Camera | None | Yes | Yes |
| Depth Testing | No | Yes | Yes (geometry pass) |
| Complexity | Low | Medium | High |

---

## 7. Creating Your Own Examples

### Adding a New Example

1. Create a new file in `src/bin/`:

```bash
touch src/bin/my_example.rs
```

2. Add the example to `Cargo.toml`:

```toml
[[bin]]
name = "my_example"
path = "src/bin/my_example.rs"
```

3. Implement the `AppRenderer` trait:

```rust
use renderlib::app::{App, AppRenderer};
use renderlib::context::GraphicsContext;

struct MyRenderer {
    // Your resources
}

impl AppRenderer for MyRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        // Initialize resources
        Self { /* ... */ }
    }
    
    fn render(&mut self, context: &mut GraphicsContext) {
        // Render a frame
    }
    
    fn resize(&mut self, _context: &mut GraphicsContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        // Handle resize
    }
}

fn main() {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

4. Run your example:

```bash
cargo run --bin my_example
```

### Example Ideas

Here are some example ideas to try:

1. **Instanced Rendering**: Render many copies of the same mesh
2. **Particle System**: Simple 2D or 3D particle system
3. **Skybox**: Render a textured skybox
4. **Post-Processing**: Add bloom or other effects
5. **Shadow Mapping**: Implement real-time shadows
6. **Normal Mapping**: Add detail with normal maps
7. **PBR Materials**: Physically-based rendering
8. **Animation**: Load and play animated GLTF models
9. **Compute Shader**: Use GPU compute for particle simulation
10. **Multi-View**: Render to multiple views (split-screen, etc.)

### Example Template

Here's a template for creating new examples:

```rust
//! [Example Name] - Brief description
//!
//! This example demonstrates:
//! - Feature 1
//! - Feature 2
//! - Feature 3

use std::time::Instant;

use winit::event_loop::EventLoop;
use renderlib::app::{App, AppRenderer};
use renderlib::context::GraphicsContext;
use renderlib::device_helpers::*;

/// Path to shader file
const SHADER_PATH: &str = "src/shaders/my_example.wgsl";

/// Uniform data
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    // Your uniform data
}

/// Renderer
struct MyRenderer {
    // Your resources
    start_time: Instant,
}

impl AppRenderer for MyRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;
        
        // Load shader
        let shader_src = load_shader_source(SHADER_PATH)
            .expect("Failed to load shader");
        let shader_module = create_shader_module(device, Some("My Shader"), &shader_src);
        
        // Create pipeline
        let render_pipeline = RenderPipelineBuilder::new(device)
            .with_label(Some("My Pipeline"))
            // ... configure pipeline
            .build();
        
        MyRenderer {
            // Initialize fields
            start_time: Instant::now(),
        }
    }
    
    fn render(&mut self, context: &mut GraphicsContext) {
        // Get elapsed time
        let elapsed = self.start_time.elapsed().as_secs_f32();
        
        // Get surface texture
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let texture_view = context.create_texture_view(&surface_texture);
        
        // Create encoder
        let mut encoder = context.device.create_command_encoder(&Default::default());
        
        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                ..Default::default()
            })],
            ..Default::default()
        });
        
        // Draw
        render_pass.set_pipeline(&self.render_pipeline);
        // Set bind groups, buffers, etc.
        // render_pass.draw(...) or render_pass.draw_indexed(...)
        
        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
        context.request_redraw();
    }
    
    fn resize(&mut self, _context: &mut GraphicsContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        // Handle resize if needed
    }
}

fn main() {
    env_logger::init();
    
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    
    let mut app = App::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

---

## Summary

The renderlib examples provide a progressive learning path:

1. **Triangle**: Basic rendering and shader hot-reloading
2. **Forward**: Mesh loading, camera, lighting, depth testing
3. **Deferred**: Multi-pass rendering, G-buffer, screen-space lighting

Each example builds on the previous one, adding more features and complexity. By studying these examples, you can learn:

- How to structure a renderlib application
- How to use the various framework components
- How to implement different rendering techniques
- How to organize shaders and resources
- How to handle input and animation

Use these examples as a foundation for your own projects, and don't hesitate to experiment and modify them to learn more!

---

## Additional Resources

- [Architecture Overview](../architecture/01-OVERVIEW.md): High-level system design
- [Module Documentation](../architecture/02-MODULES.md): Detailed module reference
- [Component Interactions](../architecture/03-COMPONENT_INTERACTIONS.md): How components work together
- [Getting Started Guide](../guides/GETTING_STARTED.md): Create your first application
- [Rendering Pipelines Guide](../guides/RENDERING.md): Deep dive into rendering techniques
- [API Reference](../api/REFERENCE.md): Complete API documentation
