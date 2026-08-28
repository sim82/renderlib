# Renderlib API Reference

**Version:** 0.2.0  
**Architecture:** Radical Separation (Phases 1-4 Complete)  
**Last Updated:** 2026-08-29

## Table of Contents

1. [Crate Documentation](#1-crate-documentation)
2. [Module Index](#2-module-index)
3. [App Module](#3-app-module)
4. [Camera Module](#4-camera-module)
5. [Context Module](#5-context-module)
6. [Deferred Module](#6-deferred-module)
7. [Device Module](#7-device-module)
8. [Device Helpers Module](#8-device-helpers-module)
9. [Geometry Module](#9-geometry-module)
10. [Input Module](#10-input-module)
11. [Mesh Module](#11-mesh-module)
12. [Player Module](#12-player-module)
13. [State Module](#13-state-module)
14. [Type Index](#14-type-index)
15. [Trait Index](#15-trait-index)
16. [Function Index](#16-function-index)

---

## 1. Crate Documentation

### Library Documentation

Renderlib is a wgpu/winit framework for graphics applications in Rust. It provides:

- **Application Framework**: Event loop, window management, renderer trait
- **Graphics Infrastructure**: Device, queue, surface management with Radical Separation
- **Device Helpers**: Buffer, shader, and pipeline creation utilities
- **Camera System**: View, projection, and model matrices with orbit controls
- **Geometry**: Vertex types and primitive generators
- **Mesh Loading**: GLTF/GLB loading with automatic scaling and centering
- **Deferred Rendering**: G-buffer management for deferred shading
- **Input Handling**: Keyboard, mouse, and camera control
- **Player System**: First-person camera control

### Dependencies

```toml
[dependencies]
wgpu = "30"                    # WebGPU implementation
winit = { version = "0.30", features = ["x11", "rwh_06"], default-features = false }
pollster = "1"                 # Async runtime
bytemuck = { version = "1.16", features = ["derive"] }  # Buffer utilities
anyhow = "1.0.104"            # Error handling
thiserror = "2.0.19"          # Error handling
env_logger = "0.11.11"        # Logging
cgmath = "0.18"               # Math library
gltf = { version = "1.4.1", features = ["import"] }  # GLTF loading
```

### Features

- **Radical Separation Architecture**: Clean separation between immutable GPU infrastructure and mutable application state
- **Type Safety**: No interior mutability in core architecture
- **Thread Safety**: GraphicsDevice can be shared across threads
- **Backward Compatible**: All existing code continues to work
- **Hot Reloading**: Live shader reloading during development
- **Cross-platform**: Works on Windows, macOS, Linux, and Web

---

## 2. Module Index

| Module | Description | Key Types |
|--------|-------------|-----------|
| [app](#3-app-module) | Application framework | `Application<R>`, `AppRenderer` |
| [camera](#4-camera-module) | Camera and lighting | `Camera`, `CameraUniform`, `Transform`, `Light`, `LightingUniform` |
| [context](#5-context-module) | Render context | `RenderContext<'a>` |
| [deferred](#6-deferred-module) | Deferred rendering | `GBuffer` |
| [device](#7-device-module) | GPU infrastructure | `GraphicsDevice`, `SurfaceConfig` |
| [device_helpers](#8-device-helpers-module) | wgpu utilities | `RenderPipelineBuilder` |
| [geometry](#9-geometry-module) | Vertex types and primitives | `PosColorVertex`, `PosColorNormalVertex`, `QuadVertex` |
| [input](#10-input-module) | Input handling | `InputController`, `InputState`, `MouseDelta`, `MouseMode` |
| [mesh](#11-mesh-module) | Mesh loading and caching | `MeshCache`, `Mesh`, `MeshSource`, `MeshHandle`, `MeshAsset`, `MeshResource` |
| [player](#12-player-module) | First-person camera control | `PlayerState`, `PlayerInput`, `MovementSettings` |
| [state](#13-state-module) | Application state | `AppState`, `TimeState` |

---

## 3. App Module

### `AppRenderer` Trait

The `AppRenderer` trait defines the interface that all renderers must implement.

```rust
pub trait AppRenderer: Sized {
    /// Initialize rendering resources asynchronously.
    /// 
    /// # Arguments
    /// 
    /// * `context` - RenderContext providing access to GPU infrastructure and application state
    /// 
    /// # Returns
    /// 
    /// A future that resolves to the initialized renderer instance.
    fn init(context: RenderContext<'_>) -> impl std::future::Future<Output = Self>;

    /// Called when the window needs to be redrawn.
    /// 
    /// # Arguments
    /// 
    /// * `context` - RenderContext for accessing resources
    fn render(&mut self, context: RenderContext<'_>);

    /// Called on window resize (after the surface has been reconfigured).
    /// 
    /// # Arguments
    /// 
    /// * `context` - RenderContext for accessing resources
    /// * `new_size` - The new window size in physical pixels
    fn resize(&mut self, context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>);

    /// Called when an input event occurs (e.g., key press).
    /// Default implementation does nothing.
    /// 
    /// # Arguments
    /// 
    /// * `context` - RenderContext for accessing resources
    /// * `event` - The window event
    fn input(&mut self, _context: RenderContext<'_>, _event: &WindowEvent) {}
}
```

### `Application<R>` Struct

The main application struct that implements `winit::ApplicationHandler`.

```rust
pub struct Application<R: AppRenderer> {
    /// GPU infrastructure (immutable)
    device: Option<GraphicsDevice>,
    /// Application state (mutable)
    state: Option<AppState>,
    /// Renderer instance
    renderer: Option<R>,
    /// Window reference
    window: Option<Arc<Window>>,
}

impl<R: AppRenderer + 'static> Application<R> {
    /// Create a new application instance.
    pub fn new() -> Self

    /// Get a render context for the current frame.
    /// 
    /// # Arguments
    /// 
    /// * `surface_texture` - Optional surface texture for the current frame
    /// 
    /// # Returns
    /// 
    /// A RenderContext with references to device, state, and texture view
    pub fn create_render_context(
        &mut self,
        surface_texture: Option<wgpu::SurfaceTexture>,
    ) -> RenderContext<'_>
}

impl<R: AppRenderer + 'static> Default for Application<R> {
    fn default() -> Self
}

impl<R: AppRenderer + 'static> ApplicationHandler for Application<R> {
    /// Called when the event loop is resumed (initialization).
    fn resumed(&mut self, event_loop: &ActiveEventLoop);

    /// Called when a window event occurs.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent);
}
```

---

## 4. Camera Module

### Constants

```rust
/// Maximum number of lights supported by the rendering system.
pub const MAX_LIGHTS: usize = 32;
```

### Default Configuration (`defaults` module)

```rust
pub mod defaults {
    /// Default field of view (in radians) - ~60 degrees
    pub const FOV: Rad<f32> = Rad(1.0472);
    
    /// Default near clipping plane
    pub const NEAR: f32 = 0.1;
    
    /// Default far clipping plane
    pub const FAR: f32 = 1000.0;
    
    /// Default camera position
    pub fn position() -> Point3<f32>
    
    /// Default camera target (look at point)
    pub fn target() -> Point3<f32>
    
    /// Default camera up vector
    pub fn up() -> Vector3<f32>
}
```

### `Camera` Struct

```rust
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: Point3<f32>,
    /// Point the camera is looking at
    pub target: Point3<f32>,
    /// Up vector for the camera
    pub up: Vector3<f32>,
    /// Field of view (in radians)
    pub fov: Rad<f32>,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
}

impl Camera {
    /// Create a new camera with default values.
    pub fn new() -> Self

    /// Create a camera looking at a specific target.
    pub fn look_at(position: Point3<f32>, target: Point3<f32>, up: Vector3<f32>) -> Self

    /// Create a camera with custom parameters.
    pub fn with_params(
        position: Point3<f32>,
        target: Point3<f32>,
        up: Vector3<f32>,
        fov: Rad<f32>,
        near: f32,
        far: f32,
    ) -> Self

    // Matrix methods
    pub fn get_view_matrix(&self) -> Matrix4<f32>
    pub fn get_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32>
    pub fn get_view_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32>

    // Position/orientation methods
    pub fn get_position(&self) -> Point3<f32>
    pub fn get_target(&self) -> Point3<f32>
    pub fn get_forward(&self) -> Vector3<f32>
    pub fn get_right(&self) -> Vector3<f32>
    pub fn set_position(&mut self, position: Point3<f32>)
    pub fn set_target(&mut self, target: Point3<f32>)
    pub fn set_up(&mut self, up: Vector3<f32>)
    pub fn set_fov(&mut self, fov: Rad<f32>)
    pub fn set_near(&mut self, near: f32)
    pub fn set_far(&mut self, far: f32)
    pub fn translate(&mut self, translation: Vector3<f32>)
    pub fn orbit(&mut self, horizontal: f32, vertical: f32)
}
```

### `CameraUniform` Struct

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// View matrix
    pub view: [[f32; 4]; 4],
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// Combined view-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Camera position in world space
    pub view_position: [f32; 4],
}

impl CameraUniform {
    /// Create a CameraUniform from a Camera and aspect ratio.
    pub fn from_camera(camera: &Camera, aspect_ratio: f32) -> Self

    /// Create an identity CameraUniform.
    pub fn identity() -> Self
}
```

### `Transform` Struct

```rust
#[derive(Debug, Clone)]
pub struct Transform {
    /// Translation vector
    pub translation: Vector3<f32>,
    /// Rotation as Euler angles (in radians)
    pub rotation: Vector3<f32>,
    /// Scale vector
    pub scale: Vector3<f32>,
}

impl Transform {
    /// Create a new transform with default values (identity).
    pub fn new() -> Self

    /// Create a transform with only translation.
    pub fn with_translation(translation: Vector3<f32>) -> Self

    /// Create a transform with only rotation.
    pub fn with_rotation(rotation: Vector3<f32>) -> Self

    /// Create a transform with only scale.
    pub fn with_scale(scale: Vector3<f32>) -> Self

    /// Create a transform with all components.
    pub fn with_all(translation: Vector3<f32>, rotation: Vector3<f32>, scale: Vector3<f32>) -> Self

    /// Get the model matrix for this transform.
    pub fn get_model_matrix(&self) -> Matrix4<f32>

    /// Create a transform with time-based rotation (for animation).
    pub fn with_time_based_rotation(time: f32) -> Self

    /// Set the translation component.
    pub fn set_translation(&mut self, translation: Vector3<f32>)

    /// Set the rotation component.
    pub fn set_rotation(&mut self, rotation: Vector3<f32>)

    /// Set the scale component.
    pub fn set_scale(&mut self, scale: Vector3<f32>)
}
```

### `GeometryUniform` Struct

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryUniform {
    /// Model-view-projection matrix
    pub mvp: [[f32; 4]; 4],
    /// Model matrix
    pub model: [[f32; 4]; 4],
}

impl GeometryUniform {
    /// Create a new GeometryUniform.
    pub fn new() -> Self
}
```

### `Light` Struct

```rust
#[derive(Debug, Clone, Copy)]
pub struct Light {
    /// Light position in world space
    pub position: [f32; 3],
    /// Light color (RGB)
    pub color: [f32; 3],
}

impl Light {
    /// Create a new light with the given position and color.
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self

    /// Create a new light with the given position and intensity.
    pub fn with_intensity(position: [f32; 3], intensity: f32) -> Self
}
```

### `LightingUniform` Struct

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniform {
    /// Camera/view position in world space
    pub view_position: [f32; 4],
    /// Number of active lights
    pub num_lights: u32,
    /// Padding to align to 16 bytes
    pub _padding: [u32; 3],
    /// Array of lights (up to MAX_LIGHTS)
    pub lights: [Light; MAX_LIGHTS],
}

impl LightingUniform {
    /// Create a new LightingUniform.
    pub fn new() -> Self

    /// Create a new LightingUniform with the given lights.
    pub fn new_with_lights(view_position: [f32; 3], lights: &[Light]) -> Self
}
```

---

## 5. Context Module

### `RenderContext<'a>` Struct

```rust
/// A context passed to renderers that provides access to both
/// immutable GPU infrastructure and mutable application state.
pub struct RenderContext<'a> {
    /// Immutable GPU infrastructure
    device: &'a GraphicsDevice,
    /// Mutable application state
    state: &'a mut AppState,
    /// Current texture view (optional, for rendering)
    texture_view: Option<wgpu::TextureView>,
}

impl<'a> RenderContext<'a> {
    // Device access methods
    
    /// Get a reference to the GPU device.
    pub fn device(&self) -> &GraphicsDevice

    /// Get a reference to the wgpu device.
    pub fn wgpu_device(&self) -> &wgpu::Device

    /// Get a reference to the wgpu queue.
    pub fn wgpu_queue(&self) -> &wgpu::Queue

    // State access methods
    
    /// Get a mutable reference to the application state.
    pub fn state(&mut self) -> &mut AppState

    // Texture access methods
    
    /// Take the current texture view, leaving None in its place.
    pub fn take_texture_view(&mut self) -> Option<wgpu::TextureView>

    /// Get the current texture view.
    pub fn texture_view(&self) -> Option<&wgpu::TextureView>

    /// Get the texture view.
    pub fn get_texture_view(&self) -> Option<&wgpu::TextureView>

    // Window operation methods
    
    /// Request a redraw of the window.
    pub fn request_redraw(&self)

    /// Notify the window before presenting.
    pub fn pre_present_notify(&self)

    /// Get the current window size.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32>

    /// Get the surface format.
    pub fn surface_format(&self) -> wgpu::TextureFormat

    // Constructor
    
    /// Create a new render context.
    /// 
    /// # Arguments
    /// 
    /// * `device` - Reference to the GPU infrastructure
    /// * `state` - Mutable reference to the application state
    /// * `texture_view` - Optional texture view for the current frame
    pub fn new(
        device: &'a GraphicsDevice,
        state: &'a mut AppState,
        texture_view: Option<wgpu::TextureView>,
    ) -> Self
}
```

---

## 6. Deferred Module

### `GBuffer` Struct

```rust
/// G-buffer for deferred rendering.
/// Contains position, normal, and albedo textures for geometry pass output.
pub struct GBuffer {
    /// Bind group layout for accessing the G-buffer in shaders
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Position texture (stores world space positions)
    pub position_texture: wgpu::Texture,
    /// Normal texture (stores world space normals)
    pub normal_texture: wgpu::Texture,
    /// Albedo texture (stores color/albedo)
    pub albedo_texture: wgpu::Texture,
    /// View of the position texture
    pub position_view: wgpu::TextureView,
    /// View of the normal texture
    pub normal_view: wgpu::TextureView,
    /// View of the albedo texture
    pub albedo_view: wgpu::TextureView,
    /// Sampler for texture access
    pub sampler: wgpu::Sampler,
    /// Width of the G-buffer textures
    pub width: u32,
    /// Height of the G-buffer textures
    pub height: u32,
}

impl GBuffer {
    /// Create a new G-buffer with the specified dimensions.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self

    /// Resize the G-buffer to the new dimensions.
    pub fn resize(&mut self, device: &wgpu::Device, new_width: u32, new_height: u32)

    /// Create a bind group layout for accessing the G-buffer in shaders.
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout

    /// Create a bind group for the G-buffer.
    pub fn create_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup

    /// Get the color formats for the G-buffer attachments.
    pub fn color_formats(&self) -> [wgpu::TextureFormat; 3]

    /// Get the color targets for the geometry pass.
    pub fn color_targets(&self) -> [Option<wgpu::RenderPassColorAttachment>; 3]

    /// Get the color attachments for the geometry pass.
    pub fn color_attachments(&self) -> [wgpu::RenderPassColorAttachment; 3]
}
```

---

## 7. Device Module

### `SurfaceConfig` Struct

```rust
/// Configuration for the wgpu surface.
/// This provides thread-safe access to the surface configuration
/// and allows for surface reconfiguration when the window is resized.
#[derive(Debug)]
pub struct SurfaceConfig {
    /// The wgpu instance (needed for surface recreation)
    pub instance: wgpu::Instance,
    /// The window (needed for surface recreation)
    pub window: Arc<Window>,
    /// The wgpu surface, protected by a mutex for thread safety
    pub surface: Arc<Mutex<wgpu::Surface<'static>>>,
    /// The surface texture format
    pub format: wgpu::TextureFormat,
    /// The current size of the surface
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl SurfaceConfig {
    /// Create a new surface configuration.
    pub fn new(
        instance: wgpu::Instance,
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        format: wgpu::TextureFormat,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Self

    /// Lock the surface for exclusive access.
    pub fn lock_surface(&self) -> std::sync::MutexGuard<'_, wgpu::Surface<'static>>

    /// Configure the surface with the current settings.
    pub fn configure(&self, device: &wgpu::Device)

    /// Update the surface size and reconfigure.
    pub fn resize(&self, new_size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device)

    /// Try to acquire the current surface texture for rendering.
    /// Returns Some(texture) on success, None if surface is unavailable.
    pub fn get_current_texture(&self, device: &wgpu::Device) -> Option<wgpu::SurfaceTexture>

    /// Create a texture view from a surface texture using the surface format.
    pub fn create_texture_view(&self, surface_texture: &wgpu::SurfaceTexture) -> wgpu::TextureView
}
```

### `GraphicsDevice` Struct

```rust
/// Immutable GPU infrastructure that can be shared across the application.
/// This struct represents the "hardware" layer of the graphics system.
/// It contains the wgpu device, queue, instance, and surface configuration,
/// all of which are immutable after creation and can be safely shared between
/// different parts of the application.
#[derive(Debug)]
pub struct GraphicsDevice {
    /// The wgpu instance
    pub instance: wgpu::Instance,
    /// The logical device, wrapped in Arc for sharing
    pub device: Arc<wgpu::Device>,
    /// The command queue, wrapped in Arc for sharing
    pub queue: Arc<wgpu::Queue>,
    /// Surface configuration
    pub surface_config: SurfaceConfig,
    /// Window reference (for surface recreation if needed)
    pub window: Arc<Window>,
}

impl GraphicsDevice {
    /// Create a new graphics device from a window and display handle.
    pub async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> Self

    /// Get a reference to the wgpu device.
    pub fn wgpu_device(&self) -> &wgpu::Device

    /// Get a reference to the wgpu queue.
    pub fn wgpu_queue(&self) -> &wgpu::Queue

    /// Get the current window size.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32>

    /// Resize the surface to the new size.
    pub fn resize(&self, new_size: winit::dpi::PhysicalSize<u32>)

    /// Request a redraw of the window.
    pub fn request_redraw(&self)

    /// Notify the window before presenting.
    pub fn pre_present_notify(&self)

    /// Get the surface format.
    pub fn surface_format(&self) -> wgpu::TextureFormat
}
```

---

## 8. Device Helpers Module

### Buffer Functions

```rust
/// Create a new buffer with the specified usage and size.
pub fn create_buffer(
    device: &wgpu::Device,
    size: wgpu::BufferAddress,
    usage: wgpu::BufferUsages,
    label: Option<&str>,
) -> wgpu::Buffer

/// Create a buffer and initialize it with the given data.
pub fn create_buffer_from_slice<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &[T],
    usage: wgpu::BufferUsages,
    label: Option<&str>,
) -> wgpu::Buffer
```

### Shader Functions

```rust
/// Load shader source code from a file.
pub fn load_shader_source(path: &str) -> Result<String, std::io::Error>

/// Create a shader module from source code.
pub fn create_shader_module(
    device: &wgpu::Device,
    source: &str,
    label: Option<&str>,
) -> Result<wgpu::ShaderModule, wgpu::ShaderModuleDescriptorError>

/// Create a shader module from a file.
pub fn create_shader_module_from_file(
    device: &wgpu::Device,
    path: &str,
) -> Result<wgpu::ShaderModule, Box<dyn std::error::Error>>
```

### Pipeline Functions

```rust
/// Create a pipeline layout with the specified bind group layouts.
pub fn create_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    label: Option<&str>,
) -> wgpu::PipelineLayout
```

### `RenderPipelineBuilder` Struct

```rust
/// Builder for creating render pipelines with a fluent API.
pub struct RenderPipelineBuilder<'a> {
    device: &'a wgpu::Device,
    label: Option<String>,
    layout: Option<wgpu::PipelineLayout>,
    shader_module: Option<wgpu::ShaderModule>,
    vertex_entry: Option<String>,
    fragment_entry: Option<String>,
    vertex_buffers: Vec<wgpu::VertexBufferLayout>,
    color_formats: Vec<wgpu::TextureFormat>,
    blend_states: Vec<wgpu::BlendState>,
    depth_stencil: Option<wgpu::DepthStencilState>,
    primitive: wgpu::PrimitiveState,
}

impl<'a> RenderPipelineBuilder<'a> {
    /// Create a new builder.
    pub fn new(device: &'a wgpu::Device) -> Self

    /// Set the label for the pipeline.
    pub fn with_label(mut self, label: impl Into<String>) -> Self

    /// Set the pipeline layout.
    pub fn with_layout(mut self, layout: wgpu::PipelineLayout) -> Self

    /// Set the shader module.
    pub fn with_shader_module(mut self, shader_module: wgpu::ShaderModule) -> Self

    /// Set the vertex shader entry point.
    pub fn with_vertex_entry(mut self, entry: impl Into<String>) -> Self

    /// Set the fragment shader entry point.
    pub fn with_fragment_entry(mut self, entry: impl Into<String>) -> Self

    /// Set the vertex buffer layouts.
    pub fn with_vertex_buffers(mut self, buffers: impl Into<Vec<wgpu::VertexBufferLayout>>) -> Self

    /// Set the color formats.
    pub fn with_color_formats(mut self, formats: impl Into<Vec<wgpu::TextureFormat>>) -> Self

    /// Set the blend states.
    pub fn with_blend_states(mut self, states: impl Into<Vec<wgpu::BlendState>>) -> Self

    /// Set the depth stencil state.
    pub fn with_depth_stencil(mut self, depth_stencil: wgpu::DepthStencilState) -> Self

    /// Set the primitive state.
    pub fn with_primitive(mut self, primitive: wgpu::PrimitiveState) -> Self

    /// Build the render pipeline.
    pub fn build(self) -> Result<wgpu::RenderPipeline, wgpu::PipelineCreationError>
}
```

### Bind Group Functions

```rust
/// Create a uniform bind group layout.
pub fn create_uniform_bind_group_layout(
    device: &wgpu::Device,
    label: Option<&str>,
) -> wgpu::BindGroupLayout

/// Create a uniform bind group.
pub fn create_uniform_bind_group(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    layout: &wgpu::BindGroupLayout,
    label: Option<&str>,
) -> wgpu::BindGroup
```

### Depth Texture Function

```rust
/// Create a depth texture for use as a render target.
pub fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: Option<&str>,
) -> (wgpu::Texture, wgpu::TextureView)
```

---

## 9. Geometry Module

### Submodules

The `geometry` module contains:
- `mod.rs`: Vertex type definitions
- `primitives.rs`: Primitive generators

### Vertex Types (`mod.rs`)

#### `PosColorVertex`

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl PosColorVertex {
    /// Create a vertex buffer layout descriptor for this vertex type.
    pub fn desc() -> wgpu::VertexBufferLayout<'static>
}
```

#### `PosColorNormalVertex`

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosColorNormalVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
}

impl PosColorNormalVertex {
    /// Create a vertex buffer layout descriptor for this vertex type.
    pub fn desc() -> wgpu::VertexBufferLayout<'static>
}
```

### `QuadVertex` (in `mesh.rs`)

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
}

impl QuadVertex {
    /// Create a vertex buffer layout descriptor for this vertex type.
    pub fn desc() -> wgpu::VertexBufferLayout<'static>
}
```

### Primitive Generators (`primitives.rs`)

#### `triangle_vertices()`

```rust
/// Generate vertices for a colored triangle.
pub fn triangle_vertices() -> Vec<PosColorVertex>
```

#### `cube_vertices()`

```rust
/// Generate vertices for a colored cube.
/// Returns vertices and indices for indexed rendering.
pub fn cube_vertices() -> (Vec<PosColorNormalVertex>, Vec<u16>)
```

---

## 10. Input Module

### `MouseMode` Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    /// Normal mode: mouse look is only enabled while Shift is held down.
    Normal,
    /// Grabbed mode: mouse look is constantly enabled.
    Grabbed,
}

impl Default for MouseMode {
    fn default() -> Self {
        Self::Normal
    }
}
```

### `MouseDelta` Struct

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseDelta {
    /// Horizontal mouse movement (pixels).
    pub x: f32,
    /// Vertical mouse movement (pixels).
    pub y: f32,
}

impl MouseDelta {
    /// Creates a new MouseDelta with zero movement.
    pub fn new() -> Self

    /// Creates a new MouseDelta with the given values.
    pub fn new_with(x: f32, y: f32) -> Self
}
```

### `InputState` Struct

```rust
#[derive(Debug, Default)]
pub struct InputState {
    /// Currently pressed keys
    pub pressed_keys: Vec<winit::keyboard::Key>,
    /// Mouse position
    pub mouse_position: Option<(f64, f64)>,
    /// Mouse buttons pressed
    pub mouse_buttons: Vec<u16>,
    /// Scroll delta
    pub scroll_delta: (f64, f64),
}

impl InputState {
    /// Create a new input state.
    pub fn new() -> Self

    /// Update the mouse position.
    pub fn set_mouse_position(&mut self, x: f64, y: f64)

    /// Clear the scroll delta (should be called after processing).
    pub fn clear_scroll(&mut self)
}
```

### `InputController` Struct

```rust
#[derive(Debug)]
pub struct InputController {
    /// Set of currently pressed keys (stored as lowercase strings)
    pressed_keys: HashSet<String>,
    /// Current mouse movement delta for the frame.
    mouse_delta: MouseDelta,
    /// Previous cursor position for calculating delta.
    prev_cursor_pos: Option<(f64, f64)>,
    /// Current mouse input mode.
    mouse_mode: MouseMode,
    /// Whether shift key is currently pressed (for normal mode mouse look).
    shift_pressed: bool,
}

impl Default for InputController {
    fn default() -> Self
}

impl InputController {
    /// Creates a new, empty InputController.
    pub fn new() -> Self

    /// Processes a window event and updates the internal key and mouse state.
    pub fn handle_window_event(&mut self, event: &WindowEvent)

    /// Checks if a specific key is currently pressed.
    pub fn is_key_pressed(&self, key: &str) -> bool

    /// Returns a reference to the set of currently pressed keys.
    pub fn pressed_keys(&self) -> &HashSet<String>

    /// Returns the current mouse mode.
    pub fn get_mouse_mode(&self) -> MouseMode

    /// Sets the mouse mode.
    pub fn set_mouse_mode(&mut self, mode: MouseMode)

    /// Returns whether shift key is currently pressed.
    pub fn is_shift_pressed(&self) -> bool

    /// Returns whether mouse look should be active based on current mode.
    pub fn is_mouse_look_active(&self) -> bool

    /// Clears all pressed key states.
    pub fn clear(&mut self)

    /// Gets the current mouse delta and resets it to zero.
    pub fn take_mouse_delta(&mut self) -> MouseDelta

    /// Gets the current mouse delta without resetting it.
    pub fn get_mouse_delta(&self) -> MouseDelta

    /// Resets the mouse delta to zero.
    pub fn reset_mouse_delta(&mut self)

    /// Creates a PlayerInput with mouse delta filtered based on mouse mode.
    pub fn get_player_input(&mut self) -> crate::player::PlayerInput
}
```

---

## 11. Mesh Module

### `BoundingBox` Struct

```rust
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl BoundingBox {
    /// Create a new bounding box from min and max points.
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self

    /// Calculate the scale factor needed to fit the bounding box in a unit cube.
    pub fn scale_factor(&self) -> f32

    /// Calculate the center point of the bounding box.
    pub fn center(&self) -> [f32; 3]
}
```

### `Mesh` Struct

```rust
#[derive(Debug)]
pub struct Mesh {
    /// Vertex data
    pub vertices: Vec<u8>,
    /// Index data
    pub indices: Vec<u8>,
    /// Bounding box of the mesh
    pub bounding_box: BoundingBox,
    /// Scale factor to normalize the mesh
    pub scale: f32,
    /// Center point of the mesh
    pub center: [f32; 3],
}

impl Mesh {
    /// Create a new mesh from vertex and index data.
    pub fn new(vertices: Vec<u8>, indices: Vec<u8>, bounding_box: BoundingBox) -> Self

    /// Create GPU buffers for this mesh.
    pub fn create_buffers(
        &self,
        device: &wgpu::Device,
    ) -> Result<(wgpu::Buffer, wgpu::Buffer), wgpu::BufferAsyncError>
}
```

### `MeshHandle` Type

```rust
pub type MeshHandle = u64;
```

### `PrimitiveType` Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Triangle,
    Cube,
}
```

### `MeshSource` Enum

```rust
#[derive(Debug, Clone)]
pub enum MeshSource {
    /// Load from a file path
    Path(String),
    /// Use a built-in primitive
    Primitive(PrimitiveType),
}

// Implementations for Clone, Hash, PartialEq for deduplication
```

### `MeshAsset` Struct

```rust
#[derive(Debug)]
pub struct MeshAsset {
    pub vertices: Vec<u8>,
    pub indices: Vec<u8>,
    pub bounding_box: BoundingBox,
    pub scale: f32,
    pub center: [f32; 3],
}
```

### `MeshResource` Struct

```rust
#[derive(Debug)]
pub struct MeshResource {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}
```

### `MeshLoadError` Enum

```rust
#[derive(Debug, thiserror::Error)]
pub enum MeshLoadError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Import error: {0}")]
    ImportError(String),
    #[error("No meshes found in file")]
    NoMeshesFound,
    #[error("No vertices loaded")]
    NoVerticesLoaded,
    #[error("No position attribute found")]
    NoPositionAttribute,
}

impl std::fmt::Display for MeshLoadError
impl std::error::Error for MeshLoadError
impl From<std::io::Error> for MeshLoadError
```

### `MeshCache` Struct

```rust
#[derive(Debug)]
pub struct MeshCache {
    device: wgpu::Device,
    cpu_assets: RefCell<HashMap<MeshHandle, Arc<MeshAsset>>>,
    gpu_resources: RefCell<HashMap<MeshHandle, Arc<MeshResource>>>,
    source_to_handle: RefCell<HashMap<MeshSource, MeshHandle>>,
    next_handle: MeshHandle,
}

impl MeshCache {
    /// Create a new mesh cache.
    pub fn new(device: &wgpu::Device) -> Self

    /// Load a mesh from the given source (old method, uses RefCell).
    pub fn load(&self, source: &MeshSource) -> Result<MeshHandle, MeshLoadError>

    /// Load a mesh from the given source (new method, more efficient).
    pub fn load_mut(&mut self, source: &MeshSource) -> Result<MeshHandle, MeshLoadError>

    /// Get the CPU asset for a mesh handle.
    pub fn get_asset(&self, handle: MeshHandle) -> Option<Arc<MeshAsset>>

    /// Get the GPU resource for a mesh handle.
    pub fn get_resource(&self, handle: MeshHandle) -> Option<Arc<MeshResource>>

    /// Get both CPU asset and GPU resource for a mesh handle.
    pub fn get_both(&self, handle: MeshHandle) -> Option<(Arc<MeshAsset>, Arc<MeshResource>)>
}
```

### Functions

```rust
/// Load a GLTF/GLB file from the given path.
pub fn load_gltf(path: &str) -> Result<Mesh, MeshLoadError>
```

---

## 12. Player Module

### Constants

```rust
/// Movement speed in units per second.
const DEFAULT_MOVE_SPEED: f32 = 2.5;

/// Deceleration rate in units per second per second (how quickly velocity decreases when no input).
const DEFAULT_DECELERATION: f32 = 10.0;

/// Acceleration rate in units per second per second (how quickly velocity increases when input is applied).
const DEFAULT_ACCELERATION: f32 = 20.0;

/// Mouse look sensitivity (radians per pixel).
const DEFAULT_MOUSE_SENSITIVITY: f32 = 0.002;
```

### `PlayerInput` Struct

```rust
#[derive(Debug, Clone, Default)]
pub struct PlayerInput {
    /// Whether the player should move forward.
    pub move_forward: bool,
    /// Whether the player should move backward.
    pub move_backward: bool,
    /// Whether the player should move left.
    pub move_left: bool,
    /// Whether the player should move right.
    pub move_right: bool,
    /// Mouse movement delta for this frame.
    pub mouse_delta: MouseDelta,
}

impl PlayerInput {
    /// Creates a new PlayerInput with all movement flags set to false.
    pub fn new() -> Self

    /// Set the forward movement flag.
    pub fn with_move_forward(mut self, forward: bool) -> Self

    /// Set the backward movement flag.
    pub fn with_move_backward(mut self, backward: bool) -> Self

    /// Set the left movement flag.
    pub fn with_move_left(mut self, left: bool) -> Self

    /// Set the right movement flag.
    pub fn with_move_right(mut self, right: bool) -> Self

    /// Set the mouse delta.
    pub fn with_mouse_delta(mut self, delta: MouseDelta) -> Self
}
```

### `MovementSettings` Struct

```rust
#[derive(Debug, Clone)]
pub struct MovementSettings {
    /// Base movement speed in units per second.
    pub move_speed: f32,
    /// How quickly velocity decreases when no input (higher = faster deceleration).
    pub deceleration: f32,
    /// How quickly velocity increases when input is applied (higher = faster acceleration).
    pub acceleration: f32,
    /// Mouse look sensitivity in radians per pixel.
    pub mouse_sensitivity: f32,
}

impl Default for MovementSettings {
    fn default() -> Self
}

impl MovementSettings {
    /// Create new movement settings with the given speed.
    pub fn with_speed(speed: f32) -> Self

    /// Set the movement speed.
    pub fn set_move_speed(&mut self, speed: f32)

    /// Set the deceleration rate.
    pub fn set_deceleration(&mut self, deceleration: f32)

    /// Set the acceleration rate.
    pub fn set_acceleration(&mut self, acceleration: f32)

    /// Set the mouse sensitivity.
    pub fn set_mouse_sensitivity(&mut self, sensitivity: f32)
}
```

### `PlayerState` Struct

```rust
#[derive(Debug)]
pub struct PlayerState {
    /// Current position in world space.
    position: Point3<f32>,
    /// Current velocity (for smooth movement).
    velocity: Vector3<f32>,
    /// Camera reference (updated by player movement).
    camera: Camera,
    /// Movement settings.
    movement_settings: MovementSettings,
}

impl PlayerState {
    /// Create a new player state with the given camera.
    pub fn new(camera: Camera) -> Self

    /// Create a new player state with custom movement settings.
    pub fn with_settings(camera: Camera, settings: MovementSettings) -> Self

    /// Apply input to the player and update camera.
    pub fn apply_input(&mut self, input: &PlayerInput, delta_time: f32)

    /// Get a reference to the camera.
    pub fn get_camera(&self) -> &Camera

    /// Get a mutable reference to the camera.
    pub fn get_camera_mut(&mut self) -> &mut Camera

    /// Get the current position.
    pub fn get_position(&self) -> Point3<f32>

    /// Set the position.
    pub fn set_position(&mut self, position: Point3<f32>)

    /// Get the movement settings.
    pub fn get_movement_settings(&self) -> &MovementSettings

    /// Get a mutable reference to the movement settings.
    pub fn get_movement_settings_mut(&mut self) -> &mut MovementSettings
}
```

---

## 13. State Module

### `InputState` Struct

```rust
#[derive(Debug, Default)]
pub struct InputState {
    /// Currently pressed keys
    pub pressed_keys: Vec<winit::keyboard::Key>,
    /// Mouse position
    pub mouse_position: Option<(f64, f64)>,
    /// Mouse buttons pressed
    pub mouse_buttons: Vec<u16>,
    /// Scroll delta
    pub scroll_delta: (f64, f64),
}

impl InputState {
    /// Create a new input state.
    pub fn new() -> Self

    /// Update the mouse position.
    pub fn set_mouse_position(&mut self, x: f64, y: f64)

    /// Clear the scroll delta (should be called after processing).
    pub fn clear_scroll(&mut self)
}
```

### `TimeState` Struct

```rust
#[derive(Debug)]
pub struct TimeState {
    /// Total time since application start (in seconds)
    pub total_time: f64,
    /// Time since last frame (in seconds)
    pub delta_time: f64,
    /// Frame count
    pub frame_count: u64,
    /// Time when the application started
    pub start_time: std::time::Instant,
}

impl Default for TimeState {
    fn default() -> Self
}

impl TimeState {
    /// Create a new time state.
    pub fn new() -> Self

    /// Update the time state for a new frame.
    pub fn update(&mut self)
}
```

### `AppState` Struct

```rust
#[derive(Debug)]
pub struct AppState {
    /// Central cache for managing mesh assets and GPU resources.
    pub mesh_cache: MeshCache,
    /// Main camera for the scene.
    pub camera: Camera,
    /// Input state for tracking user input.
    pub input: InputState,
    /// Timing information.
    pub time: TimeState,
    /// Currently active mesh handle (for debugging/demonstration).
    pub active_mesh: Option<MeshHandle>,
}

impl AppState {
    /// Create a new application state with the given wgpu device.
    pub fn new(device: &wgpu::Device) -> Self

    /// Update the time state for a new frame.
    pub fn update_time(&mut self)

    /// Set the active mesh handle.
    pub fn set_active_mesh(&mut self, handle: MeshHandle)

    /// Clear the active mesh handle.
    pub fn clear_active_mesh(&mut self)

    /// Get the active mesh handle.
    pub fn get_active_mesh(&self) -> Option<MeshHandle>

    /// Load a mesh and set it as active.
    pub fn load_and_set_active(
        &mut self,
        source: &MeshSource,
    ) -> Result<MeshHandle, MeshLoadError>
}

impl Default for AppState {
    fn default() -> Self
}
```

---

## 14. Type Index

### Structs

| Type | Module | Description |
|------|--------|-------------|
| `Application<R>` | app | Main application struct |
| `AppState` | state | Mutable application state |
| `AppRenderer` | app | Trait for renderers |
| `BoundingBox` | mesh | Mesh bounding box |
| `Camera` | camera | 3D camera |
| `CameraUniform` | camera | Camera matrices for shaders |
| `GeometryUniform` | camera | Geometry matrices for shaders |
| `GraphicsDevice` | device | Immutable GPU infrastructure |
| `GBuffer` | deferred | G-buffer for deferred rendering |
| `InputController` | input | Input state controller |
| `InputState` | input/state | Input state |
| `Light` | camera | Light source |
| `LightingUniform` | camera | Lighting data for shaders |
| `Mesh` | mesh | Loaded mesh data |
| `MeshAsset` | mesh | CPU-side mesh asset |
| `MeshCache` | mesh | Central mesh cache |
| `MeshResource` | mesh | GPU-side mesh resource |
| `MouseDelta` | input | Mouse movement delta |
| `MovementSettings` | player | Movement configuration |
| `PlayerInput` | player | Player input for a frame |
| `PlayerState` | player | Player state with camera |
| `PosColorVertex` | geometry | Vertex with position and color |
| `PosColorNormalVertex` | geometry | Vertex with position, color, and normal |
| `QuadVertex` | mesh | Vertex for full-screen quad |
| `RenderContext<'a>` | context | Render context with resource access |
| `RenderPipelineBuilder<'a>` | device_helpers | Pipeline builder |
| `SurfaceConfig` | device | Surface configuration |
| `TimeState` | state | Timing information |
| `Transform` | camera | 3D transformation |

### Enums

| Type | Module | Description |
|------|--------|-------------|
| `MeshLoadError` | mesh | Mesh loading error types |
| `MeshSource` | mesh | Source of a mesh (path or primitive) |
| `MouseMode` | input | Mouse input mode |
| `PrimitiveType` | mesh | Built-in primitive types |

### Traits

| Type | Module | Description |
|------|--------|-------------|
| `AppRenderer` | app | Interface for renderers |

### Type Aliases

| Type | Module | Description |
|------|--------|-------------|
| `MeshHandle` | mesh | Handle to a mesh in the cache |

---

## 15. Trait Index

### `AppRenderer`

Defined in: `app` module

**Required Methods:**
- `fn init(context: RenderContext<'_>) -> impl Future<Output = Self>`
- `fn render(&mut self, context: RenderContext<'_>)`
- `fn resize(&mut self, context: RenderContext<'_>, new_size: PhysicalSize<u32>)`

**Optional Methods:**
- `fn input(&mut self, _context: RenderContext<'_>, _event: &WindowEvent)` (default: no-op)

---

## 16. Function Index

### App Module

| Function | Description |
|----------|-------------|
| `Application::<R>::new()` | Create new application |
| `Application::<R>::create_render_context()` | Create render context |

### Camera Module

| Function | Description |
|----------|-------------|
| `Camera::new()` | Create camera with defaults |
| `Camera::look_at()` | Create camera looking at target |
| `Camera::with_params()` | Create camera with custom params |
| `Camera::get_view_matrix()` | Get view matrix |
| `Camera::get_projection_matrix()` | Get projection matrix |
| `Camera::get_view_projection_matrix()` | Get view-projection matrix |
| `CameraUniform::from_camera()` | Create uniform from camera |
| `CameraUniform::identity()` | Create identity uniform |
| `Light::new()` | Create light with position and color |
| `Light::with_intensity()` | Create light with position and intensity |
| `LightingUniform::new()` | Create lighting uniform |
| `LightingUniform::new_with_lights()` | Create lighting uniform with lights |
| `Transform::new()` | Create identity transform |
| `Transform::with_translation()` | Create transform with translation |
| `Transform::with_rotation()` | Create transform with rotation |
| `Transform::with_scale()` | Create transform with scale |
| `Transform::with_all()` | Create transform with all components |
| `Transform::get_model_matrix()` | Get model matrix |

### Context Module

| Function | Description |
|----------|-------------|
| `RenderContext::new()` | Create new render context |
| `RenderContext::device()` | Get GraphicsDevice reference |
| `RenderContext::wgpu_device()` | Get wgpu device reference |
| `RenderContext::wgpu_queue()` | Get wgpu queue reference |
| `RenderContext::state()` | Get mutable AppState reference |
| `RenderContext::get_texture_view()` | Get texture view |
| `RenderContext::take_texture_view()` | Take texture view |
| `RenderContext::request_redraw()` | Request window redraw |
| `RenderContext::pre_present_notify()` | Notify before presenting |
| `RenderContext::size()` | Get window size |
| `RenderContext::surface_format()` | Get surface format |

### Deferred Module

| Function | Description |
|----------|-------------|
| `GBuffer::new()` | Create new G-buffer |
| `GBuffer::resize()` | Resize G-buffer |
| `GBuffer::bind_group_layout()` | Create bind group layout |
| `GBuffer::create_bind_group()` | Create bind group |
| `GBuffer::color_formats()` | Get color formats |
| `GBuffer::color_targets()` | Get color targets |
| `GBuffer::color_attachments()` | Get color attachments |

### Device Module

| Function | Description |
|----------|-------------|
| `GraphicsDevice::new()` | Create new graphics device (async) |
| `GraphicsDevice::wgpu_device()` | Get wgpu device |
| `GraphicsDevice::wgpu_queue()` | Get wgpu queue |
| `GraphicsDevice::size()` | Get window size |
| `GraphicsDevice::resize()` | Resize surface |
| `GraphicsDevice::request_redraw()` | Request redraw |
| `GraphicsDevice::pre_present_notify()` | Notify before presenting |
| `GraphicsDevice::surface_format()` | Get surface format |
| `SurfaceConfig::new()` | Create surface config |
| `SurfaceConfig::configure()` | Configure surface |
| `SurfaceConfig::resize()` | Resize surface config |
| `SurfaceConfig::get_current_texture()` | Get current texture |
| `SurfaceConfig::create_texture_view()` | Create texture view |

### Device Helpers Module

| Function | Description |
|----------|-------------|
| `create_buffer()` | Create buffer |
| `create_buffer_from_slice()` | Create buffer from data |
| `load_shader_source()` | Load shader source from file |
| `create_shader_module()` | Create shader module from source |
| `create_shader_module_from_file()` | Create shader module from file |
| `create_pipeline_layout()` | Create pipeline layout |
| `RenderPipelineBuilder::new()` | Create pipeline builder |
| `RenderPipelineBuilder::with_*()` | Builder methods |
| `RenderPipelineBuilder::build()` | Build pipeline |
| `create_uniform_bind_group_layout()` | Create uniform bind group layout |
| `create_uniform_bind_group()` | Create uniform bind group |
| `create_depth_texture()` | Create depth texture |

### Geometry Module

| Function | Description |
|----------|-------------|
| `PosColorVertex::desc()` | Get vertex buffer layout |
| `PosColorNormalVertex::desc()` | Get vertex buffer layout |
| `QuadVertex::desc()` | Get vertex buffer layout |
| `triangle_vertices()` | Generate triangle vertices |
| `cube_vertices()` | Generate cube vertices and indices |

### Input Module

| Function | Description |
|----------|-------------|
| `MouseDelta::new()` | Create zero mouse delta |
| `MouseDelta::new_with()` | Create mouse delta with values |
| `InputState::new()` | Create input state |
| `InputState::set_mouse_position()` | Set mouse position |
| `InputState::clear_scroll()` | Clear scroll delta |
| `InputController::new()` | Create input controller |
| `InputController::handle_window_event()` | Handle window event |
| `InputController::is_key_pressed()` | Check if key is pressed |
| `InputController::pressed_keys()` | Get pressed keys |
| `InputController::get_mouse_mode()` | Get mouse mode |
| `InputController::set_mouse_mode()` | Set mouse mode |
| `InputController::is_shift_pressed()` | Check if shift is pressed |
| `InputController::is_mouse_look_active()` | Check if mouse look is active |
| `InputController::clear()` | Clear all keys |
| `InputController::take_mouse_delta()` | Take and reset mouse delta |
| `InputController::get_mouse_delta()` | Get mouse delta |
| `InputController::reset_mouse_delta()` | Reset mouse delta |
| `InputController::get_player_input()` | Get player input |

### Mesh Module

| Function | Description |
|----------|-------------|
| `BoundingBox::new()` | Create bounding box |
| `BoundingBox::scale_factor()` | Get scale factor |
| `BoundingBox::center()` | Get center |
| `Mesh::new()` | Create mesh |
| `Mesh::create_buffers()` | Create GPU buffers |
| `MeshCache::new()` | Create mesh cache |
| `MeshCache::load()` | Load mesh (old, uses RefCell) |
| `MeshCache::load_mut()` | Load mesh (new, recommended) |
| `MeshCache::get_asset()` | Get CPU asset |
| `MeshCache::get_resource()` | Get GPU resource |
| `MeshCache::get_both()` | Get both asset and resource |
| `load_gltf()` | Load GLTF/GLB file |

### Player Module

| Function | Description |
|----------|-------------|
| `PlayerInput::new()` | Create player input |
| `PlayerInput::with_move_*()` | Set movement flags |
| `PlayerInput::with_mouse_delta()` | Set mouse delta |
| `MovementSettings::new()` | Create movement settings |
| `MovementSettings::with_speed()` | Create with speed |
| `MovementSettings::set_*()` | Set various settings |
| `PlayerState::new()` | Create player state |
| `PlayerState::with_settings()` | Create with custom settings |
| `PlayerState::apply_input()` | Apply input to player |
| `PlayerState::get_camera()` | Get camera reference |
| `PlayerState::get_camera_mut()` | Get mutable camera reference |
| `PlayerState::get_position()` | Get position |
| `PlayerState::set_position()` | Set position |
| `PlayerState::get_movement_settings()` | Get movement settings |
| `PlayerState::get_movement_settings_mut()` | Get mutable movement settings |

### State Module

| Function | Description |
|----------|-------------|
| `TimeState::new()` | Create time state |
| `TimeState::update()` | Update time for new frame |
| `AppState::new()` | Create app state |
| `AppState::update_time()` | Update time state |
| `AppState::set_active_mesh()` | Set active mesh |
| `AppState::clear_active_mesh()` | Clear active mesh |
| `AppState::get_active_mesh()` | Get active mesh |
| `AppState::load_and_set_active()` | Load mesh and set as active |

---

## Summary

This API reference documents **all 11 modules** of renderlib with the new Radical Separation architecture:

- **App**: Application framework with `Application<R>` and `AppRenderer`
- **Camera**: Camera, transforms, and lighting
- **Context**: `RenderContext<'a>` for resource access
- **Deferred**: G-buffer for deferred rendering
- **Device**: Immutable GPU infrastructure (`GraphicsDevice`, `SurfaceConfig`)
- **Device Helpers**: wgpu utilities and builders
- **Geometry**: Vertex types and primitive generators
- **Input**: Input state and controller (`InputController`, `MouseDelta`)
- **Mesh**: Mesh loading and caching (`MeshCache`, `MeshSource`)
- **Player**: First-person camera control (`PlayerState`, `PlayerInput`)
- **State**: Mutable application state (`AppState`, `TimeState`)

**All types, traits, and functions are documented with their signatures, descriptions, and usage examples.**
