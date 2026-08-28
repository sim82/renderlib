# Renderlib Examples Documentation

**Version:** 0.2.0  
**Architecture:** Radical Separation (Phases 1-4 Complete)  
**Last Updated:** 2026-08-29

## Table of Contents

1. [Overview](#1-overview)
2. [Triangle Demo](#2-triangle-demo)
3. [Forward Rendering Demo](#3-forward-rendering-demo)
4. [Deferred Rendering Demo](#4-deferred-rendering-demo)
5. [Deferred with Camera Controls Demo](#5-deferred-with-camera-controls-demo)
6. [Running the Examples](#6-running-the-examples)
7. [Learning from the Examples](#7-learning-from-the-examples)
8. [Creating Your Own Examples](#8-creating-your-own-examples)

---

## 1. Overview

Renderlib includes **four demonstration programs** that showcase different aspects of the framework. All examples currently use the **old architecture** for backward compatibility, but they work perfectly with the new framework.

### Examples Summary

| Example | File | Complexity | Architecture | Description |
|---------|------|------------|-------------|-------------|
| Triangle | `src/bin/triangle.rs` | Simple | New | Rotating triangle with shader hot-reload |
| Forward | `src/bin/forward.rs` | Medium | New | Forward rendering with mesh loading and lighting |
| Deferred | `src/bin/deferred.rs` | Complex | New | Deferred rendering with G-buffer |
| Deferred with Camera | `src/bin/deferred_with_camera_controls.rs` | Complex | New | Deferred rendering with first-person camera controls |

**Note:** All examples use the **new Radical Separation architecture** with `Application<R>`, `RenderContext`, `GraphicsDevice`, and `AppState`.

### Architecture Note

All examples use the **new Radical Separation architecture** which provides:
- **Clean separation** between immutable GPU infrastructure and mutable application state
- **Type safety** with no interior mutability in core types
- **Thread safety** with Arc-wrapped GPU resources
- **Modern design** following Rust best practices

The examples demonstrate the recommended way to use renderlib. For details on the architecture, see [Architecture Overview](../architecture/01-OVERVIEW.md).

---

## 2. Triangle Demo

### Purpose

The **Triangle Demo** is the simplest example that demonstrates:
- Basic application setup
- Simple rendering pipeline
- Vertex buffer creation
- Shader usage
- **Shader hot-reloading** (press 'R' to reload)

### Features

- Renders a single colored triangle
- Triangle rotates over time
- Press 'R' to reload the shader without restarting
- Demonstrates the basic rendering loop

### Code Structure

```rust
// Main struct
struct TriangleRenderer {
    vertex_buffer: wgpu::Buffer,          // GPU buffer for triangle vertices
    uniform_buffer: wgpu::Buffer,        // GPU buffer for rotation uniform
    uniform_bind_group: wgpu::BindGroup, // Bind group for uniform buffer
    render_pipeline: wgpu::RenderPipeline, // Render pipeline
    bind_group_layout: wgpu::BindGroupLayout, // Bind group layout
    surface_format: wgpu::TextureFormat,  // Surface format
    should_reload: bool,                  // Flag for shader reload
    start_time: std::time::Instant,       // Start time for animation
}

// Uniform struct (matches shader)
struct Uniforms {
    rotation: f32,  // Rotation angle
}

// Constants
const SHADER_PATH: &str = "triangle.wgsl";
```

### Key Concepts Demonstrated

1. **Application Setup**
   - Creating `Application<TriangleRenderer>`
   - Running the event loop

2. **Resource Initialization**
   - Creating vertex buffer with `create_buffer_from_slice`
   - Creating uniform buffer
   - Loading shader from file
   - Creating render pipeline with `RenderPipelineBuilder`

3. **Rendering Loop**
   - Getting current surface texture
   - Creating texture view
   - Beginning render pass
   - Setting pipeline and bind groups
   - Drawing
   - Presenting

4. **Shader Hot-Reloading**
   - Detecting 'R' key press
   - Reloading shader source
   - Creating new shader module
   - Creating new pipeline
   - Replacing old pipeline

5. **Uniform Updates**
   - Calculating rotation based on time
   - Writing uniform data to buffer

### Shader (triangle.wgsl)

```wgsl
// Vertex shader
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
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply rotation to position
    let rotation_matrix = mat3x3<f32>(
        cos(uniforms.rotation), -sin(uniforms.rotation), 0.0,
        sin(uniforms.rotation), cos(uniforms.rotation), 0.0,
        0.0, 0.0, 1.0
    );
    
    out.clip_position = vec4<f32>(rotation_matrix * in.position, 1.0);
    out.color = in.color;
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

// Uniform definition
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;
```

### What You Can Learn

✅ **Basic Application Structure**: How to set up a renderlib application
✅ **Vertex Buffers**: Creating and using vertex buffers
✅ **Uniform Buffers**: Passing data from CPU to GPU
✅ **Shaders**: Writing WGSL shaders
✅ **Render Pipeline**: Creating a render pipeline
✅ **Rendering Loop**: The basic structure of rendering
✅ **Shader Hot-Reloading**: Live shader updates
✅ **Bind Groups**: Organizing resources for shaders

---

## 3. Forward Rendering Demo

### Purpose

The **Forward Rendering Demo** demonstrates:
- Mesh loading from GLTF/GLB files
- Forward rendering with lighting
- Depth testing
- Camera controls (basic)
- Multiple resource management

### Features

- Loads a 3D mesh (defaults to built-in cube if no file found)
- Applies forward rendering with lighting
- Uses depth buffer for proper occlusion
- Rotates the mesh over time
- Press 'R' to reload shaders
- Basic camera positioning

### Code Structure

```rust
// Main struct
struct ForwardRenderer {
    vertex_buffer: wgpu::Buffer,              // GPU buffer for mesh vertices
    index_buffer: wgpu::Buffer,               // GPU buffer for mesh indices
    num_indices: u32,                          // Number of indices to draw
    geometry_uniform_buffer: wgpu::Buffer,   // Uniform buffer for geometry (MVP, model)
    lighting_uniform_buffer: wgpu::Buffer,   // Uniform buffer for lighting
    uniform_bind_group: wgpu::BindGroup,       // Bind group for uniforms
    render_pipeline: wgpu::RenderPipeline,     // Render pipeline
    bind_group_layout: wgpu::BindGroupLayout, // Bind group layout
    surface_format: wgpu::TextureFormat,      // Surface format
    depth_texture: wgpu::Texture,              // Depth texture
    depth_texture_view: wgpu::TextureView,    // Depth texture view
    should_reload: bool,                      // Flag for shader reload
    start_time: std::time::Instant,           // Start time for animation
    model_scale: f32,                         // Scale for the model
    mesh_center: [f32; 3],                    // Center of the mesh
    camera: Camera,                           // Camera
    lights: Vec<Light>,                       // Light sources
    num_lights: usize,                        // Number of lights
}

// Uniform structs
struct CameraUniform { ... }  // View, projection, view-projection matrices
struct GeometryUniform { ... } // MVP and model matrices
struct LightingUniform { ... } // View position, lights array
```

### Key Concepts Demonstrated

1. **Mesh Loading**
   - Using `MeshCache` to load meshes
   - Handling both file-based and built-in meshes
   - Creating GPU buffers from mesh data

2. **Forward Rendering**
   - Single-pass rendering with lighting
   - Vertex and fragment shader coordination
   - Depth testing for proper occlusion

3. **Camera System**
   - Using the `Camera` struct
   - Calculating view and projection matrices
   - Creating camera uniforms

4. **Lighting**
   - Defining light sources
   - Creating lighting uniforms
   - Applying lighting in fragment shader

5. **Depth Buffer**
   - Creating depth texture
   - Configuring depth stencil state
   - Using depth in render pass

6. **Resource Management**
   - Managing multiple buffers
   - Organizing bind groups
   - Handling resize events

### Shader (forward.wgsl)

```wgsl
// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> geometry: GeometryUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = geometry.mvp * vec4<f32>(in.position, 1.0);
    out.world_position = (geometry.model * vec4<f32>(in.position, 1.0)).xyz;
    out.world_normal = normalize((geometry.model * vec4<f32>(in.normal, 0.0)).xyz);
    out.color = in.color;
    return out;
}

// Fragment shader
@group(1) @binding(0)
var<uniform> lighting: LightingUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting calculation
    let view_dir = normalize(lighting.view_position.xyz - in.world_position);
    let normal = normalize(in.world_normal);
    
    // Ambient
    let ambient = 0.1 * in.color;
    
    // Diffuse (simplified - assumes one directional light)
    let light_dir = normalize(vec3<f32>(1.0, -1.0, -1.0));
    let diffuse = max(dot(normal, light_dir), 0.0) * in.color;
    
    // Combine
    let color = (ambient + diffuse) * in.color;
    
    return vec4<f32>(color, 1.0);
}
```

### What You Can Learn

✅ **Mesh Loading**: Loading and using 3D meshes
✅ **Forward Rendering**: Single-pass rendering with lighting
✅ **Depth Testing**: Proper occlusion handling
✅ **Camera System**: Using the camera for view transforms
✅ **Lighting**: Basic lighting calculations
✅ **Multiple Buffers**: Managing vertex, index, and uniform buffers
✅ **Bind Group Organization**: Organizing resources for shaders
✅ **Resize Handling**: Recreating size-dependent resources

---

## 4. Deferred Rendering Demo

### Purpose

The **Deferred Rendering Demo** demonstrates:
- Deferred rendering technique
- G-buffer management
- Multi-pass rendering
- Efficient lighting for many light sources

### Features

- Uses G-buffer for deferred shading
- Two-pass rendering (geometry + lighting)
- Efficient lighting calculation
- Rotating mesh with lighting
- Press 'R' to reload shaders

### Code Structure

```rust
// Main struct
struct DeferredRenderer {
    // Geometry pass resources
    mesh_vertex_buffer: wgpu::Buffer,
    mesh_index_buffer: wgpu::Buffer,
    num_indices: u32,
    geometry_uniform_buffer: wgpu::Buffer,
    geometry_bind_group_layout: wgpu::BindGroupLayout,
    geometry_bind_group: wgpu::BindGroup,
    geometry_pipeline: wgpu::RenderPipeline,
    geometry_shader_path: &'static str,
    
    // Lighting pass resources
    quad_vertex_buffer: wgpu::Buffer,          // Full-screen quad
    lighting_uniform_buffer: wgpu::Buffer,
    lighting_uniform_bind_group_layout: wgpu::BindGroupLayout,
    lighting_uniform_bind_group: wgpu::BindGroup,
    lighting_pipeline: wgpu::RenderPipeline,
    lighting_shader_path: &'static str,
    
    // Shared resources
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    gbuffer: GBuffer,                            // The G-buffer
    surface_format: wgpu::TextureFormat,
    model_scale: f32,
    mesh_center: [f32; 3],
    should_reload_geometry: bool,
    should_reload_lighting: bool,
    start_time: std::time::Instant,
    camera: Camera,
    lights: Vec<Light>,
    num_lights: usize,
}
```

### Key Concepts Demonstrated

1. **G-Buffer Management**
   - Creating G-buffer with position, normal, albedo textures
   - Resizing G-buffer on window resize
   - Creating bind groups for G-buffer access

2. **Two-Pass Rendering**
   - Geometry pass: Render mesh to G-buffer
   - Lighting pass: Apply lighting to full-screen quad

3. **Deferred Lighting**
   - Sampling from G-buffer textures
   - Calculating lighting per-pixel
   - Efficient for many light sources

4. **Multiple Pipelines**
   - Separate pipelines for geometry and lighting passes
   - Different shader programs for each pass

5. **Full-Screen Quad**
   - Rendering a quad that covers the entire screen
   - Using for lighting pass

### Geometry Pass Shader (deferred_geometry.wgsl)

```wgsl
// Outputs to G-buffer
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) albedo: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> geometry: GeometryUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = geometry.mvp * vec4<f32>(in.position, 1.0);
    out.world_position = (geometry.model * vec4<f32>(in.position, 1.0)).xyz;
    out.world_normal = normalize((geometry.model * vec4<f32>(in.normal, 0.0)).xyz);
    out.albedo = in.color;
    return out;
}

// Fragment shader writes to multiple render targets
@fragment
fn fs_main(in: VertexOutput) -> vec4<f32> {
    // In actual WGSL, you need to use @location for each output
    // This is simplified for documentation
    return vec4<f32>(in.world_position, 1.0); // Position
    return vec4<f32>(in.world_normal, 1.0);   // Normal
    return vec4<f32>(in.albedo, 1.0);          // Albedo
}
```

### Lighting Pass Shader (deferred_lighting.wgsl)

```wgsl
// Sample from G-buffer
@group(0) @binding(0)
var gbuffer_position: texture_2d<f32>;
@group(0) @binding(1)
var gbuffer_normal: texture_2d<f32>;
@group(0) @binding(2)
var gbuffer_albedo: texture_2d<f32>;
@group(0) @binding(3)
var gbuffer_sampler: sampler;

@group(1) @binding(0)
var<uniform> lighting: LightingUniform;

@fragment
fn fs_main(@builtin(position) pixel_coord: vec2<u32>) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(f32(pixel_coord.x), f32(pixel_coord.y)) / 
             vec2<f32>(textureDimensions(gbuffer_position));
    
    // Sample G-buffer
    let position = textureSample(gbuffer_position, gbuffer_sampler, uv).xyz;
    let normal = textureSample(gbuffer_normal, gbuffer_sampler, uv).xyz;
    let albedo = textureSample(gbuffer_albedo, gbuffer_sampler, uv).xyz;
    
    // Calculate lighting
    let view_dir = normalize(lighting.view_position.xyz - position);
    let normal = normalize(normal);
    
    // Simple directional light
    let light_dir = normalize(vec3<f32>(1.0, -1.0, -1.0));
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    // Combine with albedo
    let color = albedo * diffuse * vec3<f32>(1.0, 0.9, 0.8);
    
    return vec4<f32>(color, 1.0);
}
```

### Render Method Structure

```rust
fn render(&mut self, context: &mut GraphicsContext) {
    // 1. Get current texture
    let texture_view = context.get_current_texture()?;
    
    // 2. Geometry Pass
    // Create command encoder
    let mut encoder = context.device.create_command_encoder(...);
    
    // Begin geometry render pass with G-buffer as targets
    let mut geometry_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &self.gbuffer.color_attachments(),
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });
    
    // Set geometry pipeline and buffers
    geometry_pass.set_pipeline(&self.geometry_pipeline);
    geometry_pass.set_bind_group(0, &self.geometry_bind_group, &[]);
    geometry_pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
    geometry_pass.set_index_buffer(self.mesh_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    
    // Draw mesh
    geometry_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    
    // 3. Lighting Pass
    let mut lighting_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        ..Default::default()
    });
    
    // Set lighting pipeline and full-screen quad
    lighting_pass.set_pipeline(&self.lighting_pipeline);
    lighting_pass.set_bind_group(0, &self.gbuffer.create_bind_group(&context.device), &[]);
    lighting_pass.set_bind_group(1, &self.lighting_uniform_bind_group, &[]);
    lighting_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
    
    // Draw full-screen quad
    lighting_pass.draw(0..6, 0..1);
    
    // 4. Submit and present
    context.queue.submit(std::iter::once(encoder.finish()));
}
```

### What You Can Learn

✅ **Deferred Rendering**: Two-pass rendering technique
✅ **G-Buffer Management**: Creating and using G-buffer
✅ **Multi-Pass Rendering**: Geometry and lighting passes
✅ **Efficient Lighting**: Per-pixel lighting with many lights
✅ **Multiple Pipelines**: Using different pipelines for different passes
✅ **Full-Screen Quad**: Rendering technique for post-processing
✅ **Texture Sampling**: Reading from multiple textures in shader
✅ **Bind Group Organization**: Complex bind group setup

---

## 5. Deferred with Camera Controls Demo

### Purpose

The **Deferred with Camera Controls Demo** extends the deferred rendering demo with:
- First-person camera controls
- Mouse look
- Keyboard movement (WASD)
- Full 3D navigation

### Features

- All features of deferred rendering demo
- First-person camera with WASD movement
- Mouse look for camera orientation
- Smooth, frame-rate independent movement
- Press 'R' to reload shaders
- Toggle mouse mode with tilde key (`)

### Code Structure

```rust
// Main struct (extends DeferredRenderer)
struct DeferredWithCameraControlsRenderer {
    // All fields from DeferredRenderer...
    
    // Additional camera control fields
    camera_controller: InputController,  // Tracks input state
    camera_speed: f32,                   // Movement speed
    mouse_sensitivity: f32,              // Mouse look sensitivity
}
```

### Key Concepts Demonstrated

1. **Camera Controls**
   - First-person camera movement
   - Mouse look for orientation
   - Smooth movement with acceleration/deceleration

2. **Input Handling**
   - Keyboard input for movement (WASD)
   - Mouse input for look
   - Input state tracking

3. **Frame-Rate Independence**
   - Using delta time for movement
   - Smooth camera updates regardless of framerate

4. **Mouse Modes**
   - Normal mode: Mouse look only when Shift is pressed
   - Grabbed mode: Mouse look always active
   - Toggle with tilde key (`)

### Input Handling

```rust
fn input(&mut self, context: &mut GraphicsContext, event: &WindowEvent) {
    // Forward to input controller
    self.camera_controller.handle_window_event(event);
    
    // Handle shader reload
    if let WindowEvent::KeyboardInput { event: key_event, .. } = event {
        if let Key::Character(c) = &key_event.logical_key {
            if c.to_ascii_lowercase() == "r" && key_event.state.is_pressed() {
                self.should_reload_geometry = true;
                self.should_reload_lighting = true;
            }
        }
    }
    
    // Update camera based on input
    self.update_camera(context);
}

fn update_camera(&mut self, context: &mut GraphicsContext) {
    let delta_time = 1.0 / 60.0; // Should use actual delta time
    
    // Get player input from controller
    let player_input = self.camera_controller.get_player_input();
    
    // Create temporary player state for camera update
    let mut player = PlayerState::new(self.camera.clone());
    player.apply_input(&player_input, delta_time);
    
    // Update camera
    self.camera = player.get_camera().clone();
}
```

### What You Can Learn

✅ **Camera Controls**: First-person camera implementation
✅ **Input Handling**: Keyboard and mouse input
✅ **Frame-Rate Independence**: Using delta time
✅ **Mouse Look**: Camera orientation from mouse
✅ **Movement System**: WASD movement with smooth controls
✅ **Input State Tracking**: Maintaining input state across frames
✅ **Mouse Modes**: Different input modes for camera control

---

## 6. Running the Examples

### Basic Usage

To run any of the examples:

```bash
# Triangle demo
cargo run --bin triangle

# Forward rendering demo
cargo run --bin forward

# Deferred rendering demo
cargo run --bin deferred

# Deferred with camera controls demo
cargo run --bin deferred_with_camera_controls
```

### Release Mode

For better performance, run in release mode:

```bash
cargo run --release --bin forward
```

### With Logging

To see debug output, enable logging:

```bash
RUST_LOG=debug cargo run --bin forward
```

Available log levels:
- `error`: Only errors
- `warn`: Warnings and errors
- `info`: General information
- `debug`: Detailed debug information
- `trace`: Very verbose tracing

### Keyboard Controls

| Key | Action | Applies To |
|-----|--------|------------|
| R | Reload shaders | All examples |
| ` (tilde) | Toggle mouse mode | Deferred with Camera |
| W | Move forward | Deferred with Camera |
| A | Move left | Deferred with Camera |
| S | Move backward | Deferred with Camera |
| D | Move right | Deferred with Camera |
| Shift | Enable mouse look (Normal mode) | Deferred with Camera |

### Asset Files

The examples look for mesh files in the `assets/` directory:

```
assets/
├── meshes/
│   └── (any .gltf or .glb files)
└── shaders/
    ├── triangle.wgsl
    ├── forward.wgsl
    ├── deferred_geometry.wgsl
    └── deferred_lighting.wgsl
```

If no mesh file is found, the examples fall back to built-in primitives (triangle or cube).

### Downloading Test Models

You can download free GLTF/GLB models from:
- [Sketchfab](https://sketchfab.com/) (free models available)
- [Poly Haven](https://polyhaven.com/) (CC0 licensed)
- [Quaternius](https://quaternius.com/) (free models)

Place downloaded models in `assets/meshes/` and update the mesh path in the example code.

---

## 7. Learning from the Examples

### Progression Path

The examples are designed to be studied in order:

1. **Start with Triangle**
   - Learn basic application structure
   - Understand rendering loop
   - See shader usage

2. **Move to Forward**
   - Learn mesh loading
   - Understand lighting
   - See depth testing

3. **Study Deferred**
   - Learn deferred rendering
   - Understand G-buffer
   - See multi-pass rendering

4. **Explore Deferred with Camera**
   - Learn camera controls
   - Understand input handling
   - See frame-rate independent movement

### Key Patterns to Notice

#### Resource Initialization

All examples follow a similar initialization pattern:

```rust
async fn init(context: &GraphicsContext) -> Self {
    // 1. Get device and queue
    let device = &context.device;
    let queue = &context.queue;
    
    // 2. Load resources (shaders, meshes)
    let shader_module = create_shader_module_from_file(device, SHADER_PATH)?;
    let mesh_handle = context.mesh_cache.load(&MeshSource::Path("mesh.gltf".to_string()))?;
    
    // 3. Create GPU resources (buffers, pipelines)
    let vertex_buffer = create_buffer_from_slice(device, &vertices, ...);
    let render_pipeline = RenderPipelineBuilder::new(device)
        .with_shader_module(shader_module)
        // ... other settings
        .build()?;
    
    // 4. Return initialized renderer
    Ok(Self { /* ... */ })
}
```

#### Render Loop Structure

The render loop follows a consistent pattern:

```rust
fn render(&mut self, context: &mut GraphicsContext) {
    // 1. Get current texture
    let texture_view = context.get_current_texture()?;
    
    // 2. Update uniforms (time-based or input-based)
    let uniforms = calculate_uniforms();
    queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    
    // 3. Create command encoder
    let mut encoder = context.device.create_command_encoder(...);
    
    // 4. Begin render pass
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &texture_view,
            // ...
        })],
        // ...
    });
    
    // 5. Set pipeline and resources
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.set_bind_group(0, &self.bind_group, &[]);
    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    
    // 6. Draw
    render_pass.draw(/* ... */);
    
    // 7. Submit and present
    context.queue.submit(std::iter::once(encoder.finish()));
}
```

#### Error Handling

Examples use Rust's `?` operator for error handling:

```rust
// In async init:
let shader_module = create_shader_module_from_file(device, path)?;
// If error, returns early with error

// For fallbacks:
let mesh_handle = context.mesh_cache.load(&source).unwrap_or_else(|_| {
    // Fallback to built-in primitive
    context.mesh_cache.load(&MeshSource::Primitive(PrimitiveType::Cube)).unwrap()
});
```

#### Shader Organization

Shaders are organized with:
- Clear input/output structures
- Proper binding declarations
- Consistent naming conventions

```wgsl
// Input from vertex buffer
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

// Output to fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

// Uniforms
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Vertex shader
@vertex
fn vs_main(in: VertexInput) -> VertexOutput { ... }

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { ... }
```

### Comparison of Examples

| Feature | Triangle | Forward | Deferred | Deferred w/ Camera |
|---------|----------|---------|----------|---------------------|
| Vertex Buffer | ✅ | ✅ | ✅ | ✅ |
| Index Buffer | ❌ | ✅ | ✅ | ✅ |
| Uniform Buffers | ✅ | ✅ | ✅ | ✅ |
| Mesh Loading | ❌ | ✅ | ✅ | ✅ |
| Depth Testing | ❌ | ✅ | ✅ | ✅ |
| Lighting | ❌ | ✅ | ✅ | ✅ |
| G-Buffer | ❌ | ❌ | ✅ | ✅ |
| Multi-Pass | ❌ | ❌ | ✅ | ✅ |
| Camera Controls | ❌ | ❌ | ❌ | ✅ |
| Input Handling | ❌ | ❌ | ❌ | ✅ |
| Shader Reload | ✅ | ✅ | ✅ | ✅ |

---

## 8. Creating Your Own Examples

### Adding a New Example

To add a new example to renderlib:

1. **Create the example file** in `src/bin/`:

```bash
# Create new example file
touch src/bin/my_example.rs
```

2. **Add the binary target** to `Cargo.toml`:

```toml
[[bin]]
name = "my_example"
path = "src/bin/my_example.rs"
```

3. **Write the example code**:

```rust
use renderlib::app::{AppRenderer, Application};
use renderlib::context::RenderContext;
use renderlib::camera::Camera;

struct MyRenderer {
    // Your renderer fields
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
}

impl AppRenderer for MyRenderer {
    async fn init(mut context: RenderContext<'_>) -> Self {
        // Initialize your renderer
        let device = context.wgpu_device();
        let surface_format = context.device().surface_format();
        
        // Create vertex buffer
        let vertices = vec![/* ... */];
        let vertex_buffer = renderlib::device_helpers::create_buffer_from_slice(
            device, &vertices, wgpu::BufferUsages::VERTEX
        );
        
        // Create render pipeline
        let shader_module = renderlib::device_helpers::create_shader_module_from_file(
            device, "my_shader.wgsl"
        ).expect("Failed to load shader");
        
        let render_pipeline = renderlib::device_helpers::RenderPipelineBuilder::new(device)
            .with_shader_module(shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(vec![/* your vertex layout */])
            .with_color_formats(vec![surface_format])
            .build()
            .expect("Failed to create pipeline");
        
        Self { render_pipeline, vertex_buffer }
    }
    
    fn render(&mut self, mut context: RenderContext<'_>) {
        // Your rendering logic
        let texture_view = context.get_texture_view().expect("No texture");
        let device = context.wgpu_device();
        let queue = context.wgpu_queue();
        
        let mut encoder = device.create_command_encoder(&Default::default());
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
        
        queue.submit(std::iter::once(encoder.finish()));
    }
    
    fn resize(&mut self, mut context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>) {
        // Handle resize if needed
    }
    
    fn input(&mut self, mut context: RenderContext<'_>, event: &WindowEvent) {
        // Handle input if needed
        // Access state via context.state()
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = Application::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

4. **Run your example**:

```bash
cargo run --bin my_example
```

### Example Ideas

Here are some ideas for new examples:

1. **Instanced Rendering**: Render many instances of the same mesh
2. **Particle System**: Simple particle effects
3. **Skybox**: Render a cubic environment map
4. **Shadow Mapping**: Add shadows to forward rendering
5. **Post-Processing**: Add bloom, SSAO, or other effects
6. **Animation**: Load and play animated GLTF models
7. **Physics**: Add simple physics with collision detection
8. **UI**: Add simple 2D UI elements
9. **Multi-View**: Render to multiple viewports
10. **Compute Shader**: Use compute shaders for GPU computation

### Example Template

Here's a complete template for a new example:

```rust
//! My Example - Brief description
//!
//! Demonstrates: List what this example shows

use std::sync::Arc;

use winit::{
    event::WindowEvent,
    event_loop::{EventLoop, ActiveEventLoop},
    window::Window,
};

use renderlib::app::{App, AppRenderer};
use renderlib::context::GraphicsContext;
use renderlib::camera::Camera;
use renderlib::device_helpers::{create_buffer_from_slice, RenderPipelineBuilder};

const SHADER_PATH: &str = "my_shader.wgsl";

// Define your vertex type
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MyVertex {
    position: [f32; 3],
    // Add other attributes as needed
}

impl MyVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MyVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

// Define your renderer
struct MyRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    // Add other fields as needed
}

impl AppRenderer for MyRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;
        let surface_format = context.surface_format;
        
        // Create vertex data
        let vertices = vec![
            MyVertex { position: [0.0, 0.5, 0.0] },
            MyVertex { position: [-0.5, -0.5, 0.0] },
            MyVertex { position: [0.5, -0.5, 0.0] },
        ];
        
        // Create vertex buffer
        let vertex_buffer = create_buffer_from_slice(
            device,
            &vertices,
            wgpu::BufferUsages::VERTEX,
            Some("My Vertex Buffer")
        );
        
        // Load shader
        let shader_module = renderlib::device_helpers::create_shader_module_from_file(
            device, SHADER_PATH
        ).expect(&format!("Failed to load shader from {}", SHADER_PATH));
        
        // Create render pipeline
        let render_pipeline = RenderPipelineBuilder::new(device)
            .with_label("My Pipeline")
            .with_shader_module(shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(vec![MyVertex::desc()])
            .with_color_formats(vec![surface_format])
            .build()
            .expect("Failed to create render pipeline");
        
        Self {
            render_pipeline,
            vertex_buffer,
        }
    }
    
    fn render(&mut self, context: &mut GraphicsContext) {
        let texture_view = context.get_current_texture().expect("No texture view");
        let device = &context.device;
        let queue = &context.queue;
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("My Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
        
        drop(render_pass);
        
        queue.submit(std::iter::once(encoder.finish()));
    }
    
    fn resize(&mut self, _context: &mut GraphicsContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        // Handle resize if your renderer has size-dependent resources
    }
    
    fn input(&mut self, _event: &WindowEvent) {
        // Handle input events
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

---

## Summary

The renderlib examples provide a **comprehensive tour** of the framework's capabilities:

1. **Triangle**: Basic rendering, shaders, hot-reload
2. **Forward**: Mesh loading, lighting, depth testing
3. **Deferred**: Multi-pass rendering, G-buffer, efficient lighting
4. **Deferred with Camera**: All of the above + camera controls, input handling

Each example builds on the previous one, adding new concepts and techniques. By studying all four examples, you'll gain a **complete understanding** of how to use renderlib for your own graphics applications.

**Next Steps:**
- Run the examples to see them in action
- Study the source code of each example
- Modify the examples to experiment
- Create your own examples based on the templates
- Check out the [Getting Started Guide](../guides/GETTING_STARTED.md) for more guidance

---

## Additional Resources

- [wgpu Documentation](https://wgpu.rs/)
- [WGSL Shader Language](https://gpuweb.github.io/gpuweb/wgsl/)
- [Learn WGPU](https://sotrh.github.io/learn-wgpu/)
- [Rust Graphics](https://rust-gpu.dev/)

*Happy coding with renderlib! 🎮*
