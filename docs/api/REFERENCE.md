# Renderlib API Reference

This document provides a comprehensive reference to all public APIs in renderlib.

## Table of Contents

1. [Crate Documentation](#1-crate-documentation)
2. [Module Index](#2-module-index)
3. [App Module](#3-app-module)
4. [Camera Module](#4-camera-module)
5. [Context Module](#5-context-module)
6. [Deferred Module](#6-deferred-module)
7. [Device Helpers Module](#7-device-helpers-module)
8. [Geometry Module](#8-geometry-module)
9. [Mesh Module](#9-mesh-module)
10. [Type Index](#10-type-index)
11. [Trait Index](#11-trait-index)
12. [Function Index](#12-function-index)

---

## 1. Crate Documentation

### Library Documentation

```rust
//! Renderlib - A wgpu/winit framework for graphics applications.
//!
//! This library provides a foundation for building graphics applications with wgpu and winit,
//! including application framework, graphics context management, device helpers, and
//! common geometry types.
```

### Dependencies

```toml
[dependencies]
wgpu = "30"
winit = { version = "0.30", features = ["x11", "rwh_06"], default-features = false}
pollster = "1"
bytemuck = { version = "1.16", features = ["derive"] }
anyhow = "1.0.104"
thiserror = "2.0.19"
env_logger = "0.11.11"
cgmath = "0.18"
gltf = { version = "1.4.1", features = ["import"] }
```

### Features

- **Application Framework**: Event loop, window management, renderer trait
- **Graphics Context**: wgpu device, surface, and swap chain management
- **Device Helpers**: Buffer, shader, and pipeline creation utilities
- **Camera System**: View, projection, and model matrices with orbit controls
- **Geometry**: Vertex types and primitive generators
- **Mesh Loading**: GLTF/GLB loading with automatic scaling and centering
- **Deferred Rendering**: G-buffer management for deferred shading
- **Hot Reloading**: Live shader reloading during development

---

## 2. Module Index

| Module | Description | Key Types |
|--------|-------------|-----------|
| `app` | Application framework | `App`, `AppRenderer` |
| `camera` | Camera and lighting | `Camera`, `CameraUniform`, `Light`, `LightingUniform`, `GeometryUniform`, `Transform` |
| `context` | Graphics context | `GraphicsContext` |
| `deferred` | Deferred rendering | `GBuffer` |
| `device_helpers` | wgpu utilities | `RenderPipelineBuilder` |
| `geometry` | Vertex types and primitives | `PosColorVertex`, `PosColorNormalVertex`, `QuadVertex` |
| `mesh` | Mesh loading | `Mesh`, `BoundingBox`, `MeshLoadError` |

---

## 3. App Module

### `AppRenderer` Trait

**Trait for application-specific rendering.**

```rust
pub trait AppRenderer: Sized {
    /// Initialize rendering resources asynchronously.
    fn init(context: &GraphicsContext) -> impl std::future::Future<Output = Self>;

    /// Called when the window needs to be redrawn.
    fn render(&mut self, context: &mut GraphicsContext);

    /// Called on window resize (after the surface has been reconfigured).
    fn resize(&mut self, context: &mut GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>);

    /// Called when an input event occurs (e.g., key press).
    /// Default implementation does nothing.
    fn input(&mut self, _event: &WindowEvent) {}
}
```

**Implementors:**
- `TriangleRenderer` (in `triangle.rs`)
- `ForwardRenderer` (in `forward.rs`)
- `DeferredRenderer` (in `deferred.rs`)

### `App<R>` Struct

**Main application struct that handles the event loop and manages the graphics context.**

```rust
pub struct App<R: AppRenderer> {
    context: Option<GraphicsContext>,
    renderer: Option<R>,
}
```

**Methods:**

```rust
impl<R: AppRenderer> App<R> {
    /// Create a new application instance.
    pub fn new() -> Self
}

impl<R: AppRenderer> Default for App<R> {
    fn default() -> Self
}

impl<R: AppRenderer + 'static> ApplicationHandler for App<R> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop)
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent)
}
```

**Example:**

```rust
let mut app = App::<MyRenderer>::new();
let event_loop = EventLoop::new().unwrap();
event_loop.run_app(&mut app).unwrap();
```

---

## 4. Camera Module

### Constants

```rust
/// Maximum number of lights supported by the rendering system.
pub const MAX_LIGHTS: usize = 32;
```

### Default Configuration

```rust
pub mod defaults {
    use cgmath::{Point3, Vector3};

    /// Default field of view in degrees.
    pub const FOV: f32 = 45.0;
    /// Default near clipping plane distance.
    pub const NEAR: f32 = 0.1;
    /// Default far clipping plane distance.
    pub const FAR: f32 = 100.0;
    /// Default camera position (looking at origin from z=5).
    pub fn position() -> Point3<f32>
    /// Default camera target (origin).
    pub fn target() -> Point3<f32>
    /// Default up vector (Y-axis).
    pub fn up() -> Vector3<f32>
}
```

### `Camera` Struct

**A 3D camera that defines a view into the scene.**

```rust
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space.
    pub position: Point3<f32>,
    /// Point the camera is looking at.
    pub target: Point3<f32>,
    /// Up vector defining the camera's orientation.
    pub up: Vector3<f32>,
    /// Vertical field of view in degrees.
    pub fov: f32,
    /// Near clipping plane distance.
    pub near: f32,
    /// Far clipping plane distance.
    pub far: f32,
}
```

**Constructors:**

```rust
impl Camera {
    /// Create a new camera with default parameters.
    pub fn new() -> Self
    
    /// Create a camera looking at a specific target from a position.
    pub fn look_at(position: Point3<f32>, target: Point3<f32>, up: Vector3<f32>) -> Self
    
    /// Create a camera with full customization.
    pub fn with_params(
        position: Point3<f32>,
        target: Point3<f32>,
        up: Vector3<f32>,
        fov: f32,
        near: f32,
        far: f32,
    ) -> Self
}
```

**Matrix Methods:**

```rust
impl Camera {
    /// Get the view matrix for this camera.
    pub fn get_view_matrix(&self) -> Matrix4<f32>
    
    /// Get the perspective projection matrix for this camera.
    pub fn get_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32>
    
    /// Get the combined view-projection matrix.
    pub fn get_view_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32>
}
```

**Accessors:**

```rust
impl Camera {
    /// Get the camera's position as a cgmath Point3.
    pub fn get_position(&self) -> Point3<f32>
    
    /// Get the camera's target as a cgmath Point3.
    pub fn get_target(&self) -> Point3<f32>
    
    /// Get the camera's forward direction (normalized).
    pub fn get_forward(&self) -> Vector3<f32>
    
    /// Get the camera's right direction (normalized).
    pub fn get_right(&self) -> Vector3<f32>
}
```

**Mutators (Builder Pattern):**

```rust
impl Camera {
    /// Set the camera's position.
    pub fn set_position(&mut self, position: Point3<f32>) -> &mut Self
    
    /// Set the camera's target.
    pub fn set_target(&mut self, target: Point3<f32>) -> &mut Self
    
    /// Set the camera's up vector.
    pub fn set_up(&mut self, up: Vector3<f32>) -> &mut Self
    
    /// Set the field of view in degrees.
    pub fn set_fov(&mut self, fov: f32) -> &mut Self
    
    /// Set the near clipping plane.
    pub fn set_near(&mut self, near: f32) -> &mut Self
    
    /// Set the far clipping plane.
    pub fn set_far(&mut self, far: f32) -> &mut Self
    
    /// Move the camera position by the given delta.
    pub fn translate(&mut self, delta: Vector3<f32>) -> &mut Self
    
    /// Orbit the camera around the target.
    pub fn orbit(&mut self, yaw: f32, pitch: f32) -> &mut Self
}
```

### `CameraUniform` Struct

**Uniform data structure for passing camera matrices to shaders.**

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// View matrix (world to view space).
    pub view: [[f32; 4]; 4],
    /// Projection matrix.
    pub projection: [[f32; 4]; 4],
    /// Combined view-projection matrix.
    pub view_projection: [[f32; 4]; 4],
    /// View position (camera position in world space).
    pub view_position: [f32; 4],
}
```

**Methods:**

```rust
impl CameraUniform {
    /// Create camera uniform data from a camera and aspect ratio.
    pub fn from_camera(camera: &Camera, aspect_ratio: f32) -> Self
    
    /// Create camera uniform with identity matrices.
    pub fn identity() -> Self
}
```

### `Transform` Struct

**Helper struct for building model matrices with common transformations.**

```rust
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// Translation component.
    pub translation: Vector3<f32>,
    /// Rotation as Euler angles in radians (x, y, z).
    pub rotation: Vector3<f32>,
    /// Scale component.
    pub scale: Vector3<f32>,
}
```

**Constructors:**

```rust
impl Transform {
    /// Create a new transform with default values (identity).
    pub fn new() -> Self
    
    /// Create a transform with translation only.
    pub fn with_translation(translation: Vector3<f32>) -> Self
    
    /// Create a transform with rotation only.
    pub fn with_rotation(rotation: Vector3<f32>) -> Self
    
    /// Create a transform with scale only.
    pub fn with_scale(scale: Vector3<f32>) -> Self
    
    /// Create a transform with all components.
    pub fn with_all(
        translation: Vector3<f32>,
        rotation: Vector3<f32>,
        scale: Vector3<f32>,
    ) -> Self
}
```

**Methods:**

```rust
impl Transform {
    /// Get the model matrix for this transform.
    pub fn get_model_matrix(&self) -> Matrix4<f32>
    
    /// Apply rotation based on elapsed time.
    pub fn with_time_based_rotation(&self, elapsed: f32, speeds: Vector3<f32>) -> Self
    
    /// Set translation.
    pub fn set_translation(&mut self, translation: Vector3<f32>) -> &mut Self
    
    /// Set rotation.
    pub fn set_rotation(&mut self, rotation: Vector3<f32>) -> &mut Self
    
    /// Set scale.
    pub fn set_scale(&mut self, scale: Vector3<f32>) -> &mut Self
}
```

### `GeometryUniform` Struct

**Per-object transformation data for vertex shading.**

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryUniform {
    /// Model-view-projection matrix (combines camera and model transformations).
    pub mvp: [[f32; 4]; 4],
    /// Model matrix (local to world space transformation).
    pub model: [[f32; 4]; 4],
}
```

**Methods:**

```rust
impl GeometryUniform {
    /// Creates a geometry uniform from camera, model matrix, and aspect ratio.
    pub fn new(camera: &Camera, model_matrix: Matrix4<f32>, aspect_ratio: f32) -> Self
}
```

### `Light` Struct

**A single light source for rendering.**

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Light {
    /// Light position in world space (xyz), w unused.
    pub position: [f32; 4],
    /// Light color as RGB (xyz), intensity multiplier in w or use separate intensity field.
    pub color: [f32; 4],
}
```

**Constructors:**

```rust
impl Light {
    /// Creates a new light with the given position and color.
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self
    
    /// Creates a new light with the given position, color, and intensity.
    pub fn with_intensity(position: [f32; 3], color: [f32; 3], intensity: f32) -> Self
}
```

### `LightingUniform` Struct

**Lighting parameters for fragment shading.**

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniform {
    /// Camera position in world space (used for specular highlights).
    pub view_position: [f32; 4],
    /// Number of active lights in the lights array.
    pub num_lights: u32,
    /// Padding to maintain 16-byte alignment for the lights array.
    pub _padding: [f32; 3],
    /// Array of light sources.
    pub lights: [Light; MAX_LIGHTS],
}
```

**Constructors:**

```rust
impl LightingUniform {
    /// Creates a lighting uniform from camera and a single light position.
    pub fn new(camera: &Camera, light_position: [f32; 3]) -> Self
    
    /// Creates a lighting uniform from camera and an array of lights.
    pub fn new_with_lights(camera: &Camera, lights: &[Light]) -> Self
    
    /// Creates a lighting uniform with multiple light positions (for convenience).
    pub fn new_with_positions(camera: &Camera, light_positions: &[[f32; 3]]) -> Self
}
```

---

## 5. Context Module

### `GraphicsContext` Struct

**Generic graphics context managing wgpu device, surface, and swap chain.**

```rust
pub struct GraphicsContext {
    pub window: Arc<Window>,
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub size: winit::dpi::PhysicalSize<u32>,
}
```

**Methods:**

```rust
impl GraphicsContext {
    /// Create a new graphics context from a window and display handle.
    pub async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> GraphicsContext
    
    /// Reconfigure the surface with current size and format.
    pub fn configure_surface(&self)
    
    /// Handle window resize.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>)
    
    /// Try to acquire the current surface texture for rendering.
    pub fn get_current_texture(&mut self) -> Option<wgpu::SurfaceTexture>
    
    /// Create a texture view from a surface texture using the context's surface format.
    pub fn create_texture_view(&self, surface_texture: &wgpu::SurfaceTexture) -> wgpu::TextureView
    
    /// Request a redraw of the window.
    pub fn request_redraw(&self)
    
    /// Notify the window before presenting.
    pub fn pre_present_notify(&self)
}
```

---

## 6. Deferred Module

### `GBuffer` Struct

**G-buffer texture format for deferred rendering.**

```rust
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
```

**Methods:**

```rust
impl GBuffer {
    /// Create a new G-buffer with the given dimensions.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, label_prefix: Option<&str>) -> Self
    
    /// Resize the G-buffer to new dimensions.
    pub fn resize(&mut self, device: &wgpu::Device, new_width: u32, new_height: u32)
    
    /// Create a bind group layout for accessing this G-buffer.
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout
    
    /// Create a bind group for this G-buffer with the given device.
    pub fn create_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup
    
    /// Returns the texture formats used by the G-buffer color attachments.
    pub fn color_formats() -> [wgpu::TextureFormat; 3]
    
    /// Creates color target states for the G-buffer render pass.
    pub fn color_targets() -> [wgpu::ColorTargetState; 3]
    
    /// Creates render pass color attachments for this G-buffer's texture views.
    pub fn color_attachments(&self) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 3]
}
```

**Texture Formats:**
- Position: `wgpu::TextureFormat::Rgba16Float`
- Normal: `wgpu::TextureFormat::Rgba16Float`
- Albedo: `wgpu::TextureFormat::Rgba16Float`

---

## 7. Device Helpers Module

### Buffer Functions

```rust
/// Generic helper to create a buffer from any Pod type
pub fn create_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    data: &T,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer

/// Generic helper to create a buffer from a slice of Pod types
pub fn create_buffer_from_slice<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    data: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer
```

### Shader Functions

```rust
/// Load shader source code from a file.
pub fn load_shader_source<P: AsRef<Path>>(path: P) -> Result<String, String>

/// Generic helper to create a shader module from WGSL source
pub fn create_shader_module(
    device: &wgpu::Device,
    label: Option<&str>,
    wgsl_source: &str,
) -> wgpu::ShaderModule
```

### Pipeline Functions

```rust
/// Generic helper to create a pipeline layout from bind group layouts
pub fn create_pipeline_layout(
    device: &wgpu::Device,
    label: Option<&str>,
    bind_group_layouts: &[Option<&wgpu::BindGroupLayout>],
) -> wgpu::PipelineLayout
```

### `RenderPipelineBuilder` Struct

**Builder for creating render pipelines with a fluent API.**

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

```rust
impl<'a> RenderPipelineBuilder<'a> {
    /// Create a new builder for a render pipeline.
    pub fn new(device: &'a wgpu::Device) -> Self
    
    /// Set the pipeline label.
    pub fn with_label(mut self, label: Option<&'a str>) -> Self
    
    /// Set the pipeline layout.
    pub fn with_layout(mut self, layout: Option<&'a wgpu::PipelineLayout>) -> Self
    
    /// Set the shader module.
    pub fn with_shader_module(mut self, module: &'a wgpu::ShaderModule) -> Self
    
    /// Set the vertex shader entry point.
    pub fn with_vertex_entry(mut self, entry: &'a str) -> Self
    
    /// Set the fragment shader entry point.
    pub fn with_fragment_entry(mut self, entry: &'a str>) -> Self
    
    /// Set the vertex buffer layouts.
    pub fn with_vertex_buffers(
        mut self,
        buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    ) -> Self
    
    /// Set color formats for pipelines with one or more render targets.
    pub fn with_color_formats(mut self, formats: &[wgpu::TextureFormat]) -> Self
    
    /// Set blend states for each color attachment.
    pub fn with_blend_states(mut self, states: &[Option<wgpu::BlendState>]) -> Self
    
    /// Set the depth and stencil state.
    pub fn with_depth_stencil(mut self, state: Option<wgpu::DepthStencilState>) -> Self
    
    /// Set the primitive state.
    pub fn with_primitive(mut self, primitive: wgpu::PrimitiveState) -> Self
    
    /// Build the render pipeline.
    pub fn build(self) -> wgpu::RenderPipeline
}
```

### Bind Group Functions

```rust
/// Generic helper to create a bind group layout for a uniform buffer
pub fn create_uniform_bind_group_layout(
    device: &wgpu::Device,
    label: Option<&str>,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout

/// Generic helper to create a bind group for a uniform buffer
pub fn create_uniform_bind_group(
    device: &wgpu::Device,
    label: Option<&str>,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup
```

### Depth Texture Function

```rust
/// Creates a depth texture and view for depth testing.
pub fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: Option<&str>,
) -> (wgpu::Texture, wgpu::TextureView)
```

---

## 8. Geometry Module

### Submodules

- `primitives`: Primitive mesh generators

### Vertex Types

#### `PosColorVertex`

**Vertex with position and color attributes.**

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl PosColorVertex {
    /// Returns the vertex buffer layout for this vertex type.
    pub fn desc() -> VertexBufferLayout<'static>
}
```

**Shader Layout:**
- Location 0: position (`Float32x3`)
- Location 1: color (`Float32x3`)

#### `PosColorNormalVertex`

**Vertex with position, color, and normal attributes.**

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosColorNormalVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
}

impl PosColorNormalVertex {
    /// Returns the vertex buffer layout for this vertex type.
    pub fn desc() -> VertexBufferLayout<'static>
}
```

**Shader Layout:**
- Location 0: position (`Float32x3`)
- Location 1: color (`Float32x3`)
- Location 2: normal (`Float32x3`)

### `QuadVertex` (in `mesh.rs`)

**Vertex for full-screen quad (2D coordinates).**

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
}

impl QuadVertex {
    /// Returns the vertex buffer layout for this vertex type.
    pub fn desc() -> VertexBufferLayout<'static>
}
```

### Primitive Generators (`primitives.rs`)

```rust
/// Returns vertices for a simple triangle with red, green, blue corners.
pub fn triangle_vertices() -> &'static [PosColorVertex]

/// Returns vertices and indices for a cube centered at the origin with side length 2.
pub fn cube_vertices() -> (Vec<PosColorNormalVertex>, Vec<u16>)
```

---

## 9. Mesh Module

### `BoundingBox` Struct

**Bounding box for a mesh, used for calculating scale and center.**

```rust
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>,
}

impl BoundingBox {
    /// Create a new bounding box from min and max points.
    pub fn new(min: Vector3<f32>, max: Vector3<f32>) -> Self
    
    /// Calculate the scale factor to fit the mesh in a unit size.
    pub fn scale_factor(&self) -> f32
    
    /// Calculate the center point of the bounding box.
    pub fn center(&self) -> Vector3<f32>
}
```

### `Mesh` Struct

**A 3D mesh with vertices and indices.**

```rust
#[derive(Debug)]
pub struct Mesh {
    /// The vertices of the mesh.
    pub vertices: Vec<PosColorNormalVertex>,
    /// The indices of the mesh (triangles).
    pub indices: Vec<u16>,
    /// The bounding box of the mesh.
    pub bounding_box: BoundingBox,
    /// Scale factor to normalize the mesh to approximately unit size.
    pub scale: f32,
    /// Center point of the mesh for translation to origin.
    pub center: Vector3<f32>,
}
```

**Methods:**

```rust
impl Mesh {
    /// Create a new mesh from vertices and indices with a default bounding box.
    pub fn new(vertices: Vec<PosColorNormalVertex>, indices: Vec<u16>) -> Self
    
    /// Create GPU buffers for this mesh on the given device.
    pub fn create_buffers(
        &self,
        device: &wgpu::Device,
        label_prefix: Option<&str>,
    ) -> (wgpu::Buffer, wgpu::Buffer, u32)
}
```

### `MeshLoadError` Enum

**Error type for mesh loading operations.**

```rust
#[derive(Debug)]
pub enum MeshLoadError {
    /// Failed to read the file.
    IoError(std::io::Error),
    /// Failed to import the GLTF/GLB file.
    ImportError(String),
    /// No meshes found in the file.
    NoMeshesFound,
    /// No vertices loaded from the mesh.
    NoVerticesLoaded,
    /// A mesh primitive has no POSITION attribute.
    NoPositionAttribute,
}

impl std::fmt::Display for MeshLoadError { /* ... */ }
impl std::error::Error for MeshLoadError { /* ... */ }
impl From<std::io::Error> for MeshLoadError { /* ... */ }
```

### Functions

```rust
/// Load a mesh from a GLTF or GLB file.
pub fn load_gltf(path: &str) -> Result<Mesh, MeshLoadError>

/// Full-screen quad vertices using 2D positions (NDC coordinates).
pub fn quad_vertices_2d() -> &'static [QuadVertex]

/// Creates a full-screen quad vertex buffer.
pub fn create_quad_buffer(device: &wgpu::Device, label: Option<&str>) -> wgpu::Buffer
```

---

## 10. Type Index

### Structs

| Type | Module | Description |
|------|--------|-------------|
| `App<R>` | `app` | Main application struct |
| `BoundingBox` | `mesh` | Bounding box for mesh |
| `Camera` | `camera` | 3D camera |
| `CameraUniform` | `camera` | Camera matrices for shaders |
| `GeometryUniform` | `camera` | MVP and model matrices |
| `GBuffer` | `deferred` | G-buffer for deferred rendering |
| `GraphicsContext` | `context` | Graphics context |
| `Light` | `camera` | Light source |
| `LightingUniform` | `camera` | Lighting parameters |
| `Mesh` | `mesh` | 3D mesh |
| `PosColorVertex` | `geometry` | Vertex with position and color |
| `PosColorNormalVertex` | `geometry` | Vertex with position, color, and normal |
| `QuadVertex` | `mesh` | Vertex for full-screen quad |
| `RenderPipelineBuilder` | `device_helpers` | Pipeline builder |
| `Transform` | `camera` | Transformation helper |

### Enums

| Type | Module | Description |
|------|--------|-------------|
| `MeshLoadError` | `mesh` | Mesh loading error |

### Traits

| Type | Module | Description |
|------|--------|-------------|
| `AppRenderer` | `app` | Renderer trait |

### Functions

See [Function Index](#12-function-index) below.

---

## 11. Trait Index

### `AppRenderer`

**Trait for application-specific rendering.**

**Required Methods:**
- `fn init(context: &GraphicsContext) -> impl Future<Output = Self>`
- `fn render(&mut self, context: &mut GraphicsContext)`
- `fn resize(&mut self, context: &mut GraphicsContext, new_size: PhysicalSize<u32>)`

**Optional Methods:**
- `fn input(&mut self, _event: &WindowEvent)`

---

## 12. Function Index

### App Module

No public functions.

### Camera Module

No public functions (all functionality through struct methods).

### Context Module

No public functions (all functionality through `GraphicsContext` methods).

### Deferred Module

| Function | Description |
|----------|-------------|
| `GBuffer::new()` | Create new G-buffer |
| `GBuffer::resize()` | Resize G-buffer |
| `GBuffer::bind_group_layout()` | Create bind group layout |
| `GBuffer::create_bind_group()` | Create bind group |
| `GBuffer::color_formats()` | Get color formats |
| `GBuffer::color_targets()` | Get color target states |
| `GBuffer::color_attachments()` | Get color attachments |

### Device Helpers Module

| Function | Description |
|----------|-------------|
| `create_buffer()` | Create buffer from single value |
| `create_buffer_from_slice()` | Create buffer from slice |
| `load_shader_source()` | Load shader from file |
| `create_shader_module()` | Create shader module |
| `create_pipeline_layout()` | Create pipeline layout |
| `create_uniform_bind_group_layout()` | Create uniform bind group layout |
| `create_uniform_bind_group()` | Create uniform bind group |
| `create_depth_texture()` | Create depth texture and view |

### Geometry Module

| Function | Description |
|----------|-------------|
| `PosColorVertex::desc()` | Get vertex buffer layout |
| `PosColorNormalVertex::desc()` | Get vertex buffer layout |
| `triangle_vertices()` | Get triangle vertices |
| `cube_vertices()` | Get cube vertices and indices |

### Mesh Module

| Function | Description |
|----------|-------------|
| `Mesh::new()` | Create new mesh |
| `Mesh::create_buffers()` | Create GPU buffers |
| `BoundingBox::new()` | Create bounding box |
| `BoundingBox::scale_factor()` | Calculate scale factor |
| `BoundingBox::center()` | Calculate center |
| `load_gltf()` | Load mesh from GLTF/GLB |
| `quad_vertices_2d()` | Get full-screen quad vertices |
| `create_quad_buffer()` | Create quad vertex buffer |

---

## Summary

This API reference documents all public types, traits, and functions in renderlib. For more information:

- [Architecture Overview](../architecture/01-OVERVIEW.md): High-level system design
- [Module Documentation](../architecture/02-MODULES.md): Detailed module descriptions
- [Component Interactions](../architecture/03-COMPONENT_INTERACTIONS.md): How components work together
- [Getting Started Guide](../guides/GETTING_STARTED.md): Create your first application
- [Rendering Pipelines Guide](../guides/RENDERING.md): Deep dive into rendering techniques

For the latest API documentation, also check the Rustdoc generated from the source code:

```bash
cargo doc --open
```
