# Renderlib Module Documentation

This document provides detailed documentation for each module in renderlib.

## Table of Contents

1. [app Module](#1-app-module)
2. [camera Module](#2-camera-module)
3. [context Module](#3-context-module)
4. [device_helpers Module](#4-device_helpers-module)
5. [deferred Module](#5-deferred-module)
6. [geometry Module](#6-geometry-module)
7. [mesh Module](#7-mesh-module)
8. [lib.rs](#8-librs)

---

## 1. app Module

**File:** `src/app.rs`  
**Purpose:** Application framework and event loop management

### Overview

The `app` module provides a generic application handler that implements `winit::ApplicationHandler`. It manages the event loop, window creation, graphics context, and renderer lifecycle.

### Key Types

#### `AppRenderer` Trait

```rust
pub trait AppRenderer: Sized {
    fn init(context: &GraphicsContext) -> impl Future<Output = Self>;
    fn render(&mut self, context: &mut GraphicsContext);
    fn resize(&mut self, context: &mut GraphicsContext, new_size: PhysicalSize<u32>);
    fn input(&mut self, _event: &WindowEvent) {}
}
```

**Methods:**

- `init(context)`: Asynchronously initialize rendering resources. Called once when the window is created.
- `render(context)`: Render a frame. Called on redraw requests.
- `resize(context, new_size)`: Handle window resize. Called when the window size changes.
- `input(event)`: Handle input events. Default implementation does nothing.

**Usage:**

```rust
struct MyRenderer {
    // Your rendering resources
}

impl AppRenderer for MyRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        // Initialize buffers, pipelines, etc.
        Self { /* ... */ }
    }
    
    fn render(&mut self, context: &mut GraphicsContext) {
        // Render your scene
    }
}

let mut app = App::<MyRenderer>::new();
```

#### `App<R>` Struct

```rust
pub struct App<R: AppRenderer> {
    context: Option<GraphicsContext>,
    renderer: Option<R>,
}
```

**Methods:**

- `new()`: Create a new application instance
- Implements `ApplicationHandler` for winit integration

**Implementation Details:**

- `resumed()`: Creates window, initializes graphics context, initializes renderer
- `window_event()`: Handles close, redraw, and resize events; forwards input to renderer

### Dependencies

- `winit`: Window creation and event handling
- `crate::context`: Graphics context management

---

## 2. camera Module

**File:** `src/camera.rs`  
**Purpose:** Camera, transformation, and lighting utilities

### Overview

The `camera` module provides comprehensive support for 3D scene viewing, including camera abstractions, transformation matrices, and lighting data structures. All types are designed to work seamlessly with wgpu uniform buffers (std140 layout).

### Constants

- `MAX_LIGHTS: usize = 32`: Maximum number of lights supported by the rendering system

### Default Configuration (`defaults` module)

```rust
pub const FOV: f32 = 45.0;      // Field of view in degrees
pub const NEAR: f32 = 0.1;      // Near clipping plane
pub const FAR: f32 = 100.0;     // Far clipping plane
pub fn position() -> Point3<f32>  // (0, 0, 5)
pub fn target() -> Point3<f32>    // (0, 0, 0)
pub fn up() -> Vector3<f32>       // (0, 1, 0)
```

### Key Types

#### `Camera` Struct

```rust
pub struct Camera {
    pub position: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}
```

**Constructors:**

- `new()`: Create with default parameters (position at (0,0,5), looking at origin)
- `look_at(position, target, up)`: Create looking at a specific target
- `with_params(position, target, up, fov, near, far)`: Full customization

**Matrix Methods:**

- `get_view_matrix()`: Returns view matrix (world to view space)
- `get_projection_matrix(aspect_ratio)`: Returns perspective projection matrix
- `get_view_projection_matrix(aspect_ratio)`: Returns combined view-projection matrix

**Accessors:**

- `get_position()`, `get_target()`: Get camera position and target
- `get_forward()`, `get_right()`: Get camera direction vectors

**Mutators (Builder Pattern):**

- `set_position()`, `set_target()`, `set_up()`: Set individual properties
- `set_fov()`, `set_near()`, `set_far()`: Set projection parameters
- `translate(delta)`: Move camera by delta vector
- `orbit(yaw, pitch)`: Orbit camera around target

**Usage Example:**

```rust
let mut camera = Camera::new();
let view = camera.get_view_matrix();
let proj = camera.get_projection_matrix(16.0 / 9.0);
let mvp = camera.get_view_projection_matrix(16.0 / 9.0);

// Orbit the camera
camera.orbit(0.1, 0.05);
```

#### `CameraUniform` Struct

Uniform buffer data for passing camera matrices to shaders.

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
    pub view_projection: [[f32; 4]; 4],
    pub view_position: [f32; 4],
}
```

**Methods:**

- `from_camera(camera, aspect_ratio)`: Create from Camera and aspect ratio
- `identity()`: Create with identity matrices

#### `Transform` Struct

Helper for building model matrices with common transformations.

```rust
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: Vector3<f32>,    // Euler angles in radians (x, y, z)
    pub scale: Vector3<f32>,
}
```

**Constructors:**

- `new()`: Identity transform
- `with_translation()`, `with_rotation()`, `with_scale()`: Single-component constructors
- `with_all(translation, rotation, scale)`: Full constructor

**Methods:**

- `get_model_matrix()`: Returns the combined model matrix (translation * rotation * scale)
- `with_time_based_rotation(elapsed, speeds)`: Create animated rotation
- `set_translation()`, `set_rotation()`, `set_scale()`: Builder pattern mutators

#### `GeometryUniform` Struct

Per-object transformation data for vertex shading.

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryUniform {
    pub mvp: [[f32; 4]; 4],      // Model-view-projection matrix
    pub model: [[f32; 4]; 4],    // Model matrix
}
```

**Methods:**

- `new(camera, model_matrix, aspect_ratio)`: Create from camera, model matrix, and aspect ratio

#### `Light` Struct

A single light source for rendering.

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Light {
    pub position: [f32; 4],
    pub color: [f32; 4],
}
```

**Constructors:**

- `new(position, color)`: Create with position and color
- `with_intensity(position, color, intensity)`: Create with intensity multiplier

#### `LightingUniform` Struct

Lighting parameters for fragment shading.

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniform {
    pub view_position: [f32; 4],
    pub num_lights: u32,
    pub _padding: [f32; 3],
    pub lights: [Light; MAX_LIGHTS],
}
```

**Constructors:**

- `new(camera, light_position)`: Create with single light (backward compatibility)
- `new_with_lights(camera, lights)`: Create with array of lights
- `new_with_positions(camera, light_positions)`: Create with array of positions (white color)

### Tests

The module includes unit tests for:
- Camera default values
- View and projection matrix generation
- Transform model matrix calculation
- Camera uniform creation

---

## 3. context Module

**File:** `src/context.rs`  
**Purpose:** Graphics context management

### Overview

The `context` module provides the `GraphicsContext` struct, which encapsulates all wgpu resources needed for rendering. It handles device initialization, surface management, and resize operations.

### Key Type

#### `GraphicsContext` Struct

```rust
pub struct GraphicsContext {
    pub window: Arc<Window>,
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub size: PhysicalSize<u32>,
}
```

**Methods:**

- `new(display, window)`: Async constructor that creates all wgpu resources
- `configure_surface()`: Configure surface with current size and format
- `resize(new_size)`: Handle window resize by updating size and reconfiguring surface
- `get_current_texture()`: Try to acquire current surface texture for rendering
- `create_texture_view(surface_texture)`: Create texture view from surface texture
- `request_redraw()`: Request window redraw
- `pre_present_notify()`: Notify window before presenting

**Surface Texture Acquisition:**

The `get_current_texture()` method handles various surface states:
- `Success`: Returns the texture
- `Occluded/Timeout`: Returns None
- `Suboptimal/Outdated`: Reconfigures surface and returns None
- `Lost`: Recreates surface and returns None
- `Validation`: Panics (no error scope registered)

### Dependencies

- `winit`: Window and display handle
- `wgpu`: Graphics API

---

## 4. device_helpers Module

**File:** `src/device_helpers.rs`  
**Purpose:** Generic helper functions for wgpu device operations

### Overview

The `device_helpers` module provides ergonomic wrappers around common wgpu operations, reducing boilerplate while maintaining full generality.

### Functions

#### Buffer Creation

```rust
pub fn create_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    data: &T,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer

pub fn create_buffer_from_slice<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    data: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer
```

#### Shader Management

```rust
pub fn load_shader_source<P: AsRef<Path>>(path: P) -> Result<String, String>

pub fn create_shader_module(
    device: &wgpu::Device,
    label: Option<&str>,
    wgsl_source: &str,
) -> wgpu::ShaderModule
```

#### Pipeline Building

The `RenderPipelineBuilder` provides a fluent API for creating render pipelines:

```rust
pub struct RenderPipelineBuilder<'a> {
    device: &'a wgpu::Device,
    label: Option<&'a str>,
    layout: Option<&'a wgpu::PipelineLayout>,
    shader_module: Option<&'a wgpu::ShaderModule>,
    vertex_entry: Option<&'a str>,
    fragment_entry: Option<&'a str>,
    vertex_buffers: Option<&'a [Option<wgpu::VertexBufferLayout<'a>>]>,
    color_formats: Vec<wgpu::TextureFormat>,
    blend_states: Vec<Option<wgpu::BlendState>>,
    depth_stencil: Option<wgpu::DepthStencilState>,
    primitive: wgpu::PrimitiveState,
}
```

**Builder Methods:**

- `new(device)`: Create new builder
- `with_label(label)`: Set pipeline label
- `with_layout(layout)`: Set pipeline layout
- `with_shader_module(module)`: Set shader module
- `with_vertex_entry(entry)`: Set vertex shader entry point
- `with_fragment_entry(entry)`: Set fragment shader entry point
- `with_vertex_buffers(buffers)`: Set vertex buffer layouts
- `with_color_formats(formats)`: Set color formats (supports single or multiple)
- `with_blend_states(states)`: Set blend states per attachment
- `with_depth_stencil(state)`: Set depth and stencil state
- `with_primitive(primitive)`: Set primitive state
- `build()`: Build and return the render pipeline

**Usage Example:**

```rust
let pipeline = RenderPipelineBuilder::new(&device)
    .with_label(Some("My Pipeline"))
    .with_layout(Some(&pipeline_layout))
    .with_shader_module(&shader_module)
    .with_vertex_entry("vs_main")
    .with_fragment_entry("fs_main")
    .with_vertex_buffers(&[Some(Vertex::desc())])
    .with_color_formats(&[surface_format.add_srgb_suffix()])
    .with_depth_stencil(Some(depth_stencil_state))
    .build();
```

#### Bind Group Helpers

```rust
pub fn create_uniform_bind_group_layout(
    device: &wgpu::Device,
    label: Option<&str>,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout

pub fn create_uniform_bind_group(
    device: &wgpu::Device,
    label: Option<&str>,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup
```

#### Depth Texture

```rust
pub fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: Option<&str>,
) -> (wgpu::Texture, wgpu::TextureView)
```

Creates a depth texture with `Depth32Float` format and appropriate view for rendering.

---

## 5. deferred Module

**File:** `src/deferred.rs`  
**Purpose:** G-buffer management for deferred shading

### Overview

The `deferred` module provides the `GBuffer` struct for managing the multiple render targets needed for deferred rendering. It handles texture creation, resizing, and bind group management.

### Key Type

#### `GBuffer` Struct

```rust
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
```

**Texture Formats:** All textures use `Rgba16Float` format for 16-bit floating point precision.

**Methods:**

- `new(device, width, height, label_prefix)`: Create new G-buffer with specified dimensions
- `resize(device, new_width, new_height)`: Resize all textures to new dimensions
- `bind_group_layout(device)`: Create bind group layout for G-buffer access
- `create_bind_group(device)`: Create bind group for this G-buffer
- `color_formats()`: Return array of color formats for render pass
- `color_targets()`: Return array of color target states
- `color_attachments()`: Return array of render pass color attachments

**Bind Group Layout:**

The G-buffer bind group layout includes:
- Binding 0: Position texture
- Binding 1: Normal texture
- Binding 2: Albedo texture
- Binding 3: Sampler

All bindings are visible to the fragment shader stage.

**Sampler:**

Uses linear filtering with clamp-to-edge address mode for all texture coordinates.

### Usage Example

```rust
// Create G-buffer
let gbuffer = GBuffer::new(&device, width, height, Some("My GBuffer"));

// In geometry pass
let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &gbuffer.color_attachments(),
    // ...
});

// In lighting pass
let gbuffer_bind_group = gbuffer.create_bind_group(&device);
let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    // ...
});
render_pass.set_bind_group(0, &gbuffer_bind_group, &[]);
```

---

## 6. geometry Module

**File:** `src/geometry/`  
**Purpose:** Vertex types and primitive mesh generators

### Submodules

- `mod.rs`: Vertex type definitions
- `primitives.rs`: Primitive mesh generators

### Vertex Types (`mod.rs`)

#### `PosColorVertex`

Vertex with position and color attributes.

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl PosColorVertex {
    pub fn desc() -> VertexBufferLayout<'static>
}
```

**Shader Layout:**
- Location 0: position (Float32x3)
- Location 1: color (Float32x3)

**Use Case:** Simple 2D/3D rendering without lighting.

#### `PosColorNormalVertex`

Vertex with position, color, and normal attributes.

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosColorNormalVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
}

impl PosColorNormalVertex {
    pub fn desc() -> VertexBufferLayout<'static>
}
```

**Shader Layout:**
- Location 0: position (Float32x3)
- Location 1: color (Float32x3)
- Location 2: normal (Float32x3)

**Use Case:** 3D rendering with lighting (per-vertex normals for diffuse lighting).

### Primitive Generators (`primitives.rs`)

#### `triangle_vertices()`

Returns vertices for a simple triangle with red, green, blue corners.

```rust
pub fn triangle_vertices() -> &'static [PosColorVertex]
```

**Vertices:**
- Top: (0.0, 0.5, 0.0) - Red
- Bottom-left: (-0.5, -0.5, 0.0) - Green
- Bottom-right: (0.5, -0.5, 0.0) - Blue

#### `cube_vertices()`

Returns vertices and indices for a cube centered at the origin with side length 2.

```rust
pub fn cube_vertices() -> (Vec<PosColorNormalVertex>, Vec<u16>)
```

**Features:**
- Each face has a different color
- Front: Red, Back: Green, Right: Blue, Left: Yellow, Top: Magenta, Bottom: Cyan
- Outward-facing normals for each face
- 24 vertices (4 per face × 6 faces)
- 36 indices (6 per face × 6 faces, 2 triangles per face)

---

## 7. mesh Module

**File:** `src/mesh.rs`  
**Purpose:** Mesh loading and management

### Overview

The `mesh` module provides functionality for loading 3D meshes from GLTF/GLB files and managing their vertex and index buffers. It includes automatic scaling and centering based on the mesh's bounding box.

### Key Types

#### `BoundingBox` Struct

```rust
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>,
}
```

**Methods:**

- `new(min, max)`: Create new bounding box
- `scale_factor()`: Calculate scale factor to fit mesh in unit size
- `center()`: Calculate center point of bounding box

#### `Mesh` Struct

```rust
#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<PosColorNormalVertex>,
    pub indices: Vec<u16>,
    pub bounding_box: BoundingBox,
    pub scale: f32,
    pub center: Vector3<f32>,
}
```

**Methods:**

- `new(vertices, indices)`: Create mesh with automatic bounding box calculation
- `create_buffers(device, label_prefix)`: Create GPU vertex and index buffers

**Automatic Processing:**
- Calculates bounding box from vertices
- Computes scale factor to normalize mesh to approximately unit size
- Computes center point for translation to origin

#### `QuadVertex` Struct

Vertex for full-screen quad (2D coordinates).

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
}

impl QuadVertex {
    pub fn desc() -> VertexBufferLayout<'static>
}
```

**Shader Layout:**
- Location 0: position (Float32x2)

#### Functions

```rust
pub fn quad_vertices_2d() -> &'static [QuadVertex]

pub fn create_quad_buffer(device: &wgpu::Device, label: Option<&str>) -> wgpu::Buffer
```

#### `MeshLoadError` Enum

Error type for mesh loading operations.

```rust
pub enum MeshLoadError {
    IoError(std::io::Error),
    ImportError(String),
    NoMeshesFound,
    NoVerticesLoaded,
    NoPositionAttribute,
}
```

#### `load_gltf(path)` Function

Load a mesh from a GLTF or GLB file.

```rust
pub fn load_gltf(path: &str) -> Result<Mesh, MeshLoadError>
```

**Features:**
- Supports both .gltf and .glb formats
- Handles external buffers for .gltf files
- Automatically generates normals if missing (default to upward)
- Uses default light gray color for all vertices
- Falls back to built-in primitives if file loading fails

**Usage Example:**

```rust
match load_gltf("assets/model.glb") {
    Ok(mesh) => {
        let (vertex_buffer, index_buffer, num_indices) = 
            mesh.create_buffers(&device, Some("Model"));
        // Use buffers for rendering
    }
    Err(e) => {
        eprintln!("Failed to load mesh: {}", e);
        // Fall back to built-in cube
    }
}
```

### Dependencies

- `cgmath`: Vector and matrix operations
- `gltf`: GLTF file parsing
- `crate::device_helpers`: Buffer creation
- `crate::geometry`: Vertex types

---

## 8. lib.rs

**File:** `src/lib.rs`  
**Purpose:** Library root and module exports

### Overview

The `lib.rs` file serves as the library root, documenting the crate and re-exporting all public modules.

### Module Exports

```rust
pub mod app;
pub mod camera;
pub mod context;
pub mod deferred;
pub mod device_helpers;
pub mod geometry;
pub mod mesh;
```

### Documentation

The library documentation starts with:

```rust
//! Renderlib - A wgpu/winit framework for graphics applications.
//!
//! This library provides a foundation for building graphics applications with wgpu and winit,
//! including application framework, graphics context management, device helpers, and
//! common geometry types.
```

---

## Summary

Each module in renderlib is designed to be:

1. **Focused**: Single responsibility principle
2. **Independent**: Minimal dependencies on other modules
3. **Extensible**: Easy to extend or replace
4. **Well-documented**: Clear API with examples

The modules work together seamlessly, but can also be used independently if needed.
