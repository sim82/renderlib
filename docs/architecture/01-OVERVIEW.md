# Renderlib Architecture Overview

**Version:** 0.2.0  
**Language:** Rust  
**Primary Dependencies:** wgpu, winit, cgmath, gltf  
**Architecture:** Radical Separation (Phases 1-4 Complete)

## Table of Contents

1. [Project Summary](#project-summary)
2. [Architecture Layers](#architecture-layers)
3. [Core Components](#core-components)
4. [New Architecture: Radical Separation](#new-architecture-radical-separation)
5. [Rendering Paradigms](#rendering-paradigms)
6. [Data Flow](#data-flow)
7. [Design Philosophy](#design-philosophy)

---

## Project Summary

**Renderlib** is a lightweight, modular graphics framework built on [wgpu](https://github.com/gfx-rs/wgpu) and [winit](https://github.com/rust-windowing/winit). It provides a foundation for building graphics applications with minimal boilerplate while maintaining full access to the underlying WebGPU API.

### Key Features

- **Cross-platform**: Runs on Vulkan, Metal, DirectX 12, OpenGL, and WebGPU backends via wgpu
- **Application Framework**: Built-in event loop and window management
- **Multiple Rendering Paths**: Forward rendering and deferred rendering support
- **Asset Loading**: GLTF/GLB mesh loading with automatic scaling and centering
- **Camera System**: Flexible camera with orbit controls and projection matrices
- **Geometry Utilities**: Pre-built primitives and vertex types
- **Device Helpers**: Ergonomic wrappers for common wgpu operations
- **Hot Reloading**: Live shader reloading during development

### Architecture Evolution

Renderlib has undergone a **major refactoring** implementing the **Radical Separation** architecture:

- **Phase 1 (✅ Completed)**: Created `GraphicsDevice` and `AppState` types
- **Phase 2 (✅ Completed)**: Enhanced `MeshCache` with source deduplication
- **Phase 3 (✅ Completed)**: Created `RenderContext` and updated framework
- **Phase 4 (✅ 50% Completed)**: Framework ready, renderer migration in progress

**See [ARCHITECTURE_MIGRATION.md](../ARCHITECTURE_MIGRATION.md) for migration guide.**

---

## Architecture Layers

The **new Radical Separation architecture** organizes the codebase into clear layers:

```
┌─────────────────────────────────────────────────────────────┐
│                      Application Layer                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │
│  │   triangle   │  │   forward    │  │      deferred         │   │
│  │    demo      │  │   demo       │  │      demo            │   │
│  └─────────────┘  └─────────────┘  └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                   NEW: Framework Layer                          │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Application<R> (src/app.rs)                   │  │
│  │  - Manages GraphicsDevice, AppState, and Renderer          │  │
│  │  - Implements ApplicationHandler for winit                 │  │
│  │  - Creates RenderContext for renderer access               │  │
│  └─────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              RenderContext<'a> (src/context.rs)            │  │
│  │  - Temporary context passed to renderers                   │  │
│  │  - Provides access to both GraphicsDevice and AppState     │  │
│  │  - Manages surface texture lifecycle                        │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                             │
             ┌───────────────────┬───────────────────┐
             ▼                   ▼                   ▼
┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
│  Immutable GPU        │ │   Mutable Application  │ │   Rendering           │
│  Infrastructure       │ │   State               │ │   Techniques          │
│  (src/device.rs)     │ │  (src/state.rs)       │ │  (src/deferred.rs)    │
│                     │ │                      │ │                      │
│  - GraphicsDevice    │ │  - AppState           │ │  - GBuffer            │
│    - instance        │ │    - mesh_cache      │ │  - Bind groups       │
│    - device (Arc)    │ │    - camera           │ │  - Render passes     │
│    - queue (Arc)     │ │    - input            │ │                      │
│    - surface_config  │ │    - time             │ │                      │
│    - window (Arc)    │ │    - active_mesh      │ │                      │
└─────────────────────┘ └─────────────────────┘ └─────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                      Core Systems Layer                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │
│  │  camera.rs   │  │  geometry/   │  │      mesh.rs         │   │
│  │  (Camera,   │  │  (Vertex     │  │  (Mesh loading,      │   │
│  │   Light,    │  │   types,     │  │   BoundingBox)       │   │
│  │   Transform)│  │   primitives)│  │                      │   │
│  └─────────────┘  └─────────────┘  └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                      External Dependencies                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │
│  │   wgpu       │  │   winit      │  │      cgmath         │   │
│  │  (Graphics)  │  │ (Windowing)  │  │   (Math)            │   │
│  └─────────────┘  └─────────────┘  └─────────────────────┘   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │
│  │  pollster    │  │   gltf       │  │    bytemuck         │   │
│  │ (Async)      │  │ (Asset Load) │  │   (Buffer utils)    │   │
│  └─────────────┘  └─────────────┘  └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Layer Descriptions

#### Application Layer
- **Purpose**: Entry points and demo applications
- **Files**: `src/bin/*.rs`
- **Responsibility**: Demonstrate framework usage

#### Framework Layer (NEW - Radical Separation)
- **Purpose**: Application management and resource access
- **Key Types**: `Application<R>`, `RenderContext<'a>`
- **Responsibility**: Manage lifecycle, provide clean access to resources

#### Infrastructure Layer (NEW - Separated)
- **GraphicsDevice** (`src/device.rs`): Immutable GPU resources
  - `instance: wgpu::Instance`
  - `device: Arc<wgpu::Device>`
  - `queue: Arc<wgpu::Queue>`
  - `surface_config: SurfaceConfig`
  - `window: Arc<Window>`
- **AppState** (`src/state.rs`): Mutable application state
  - `mesh_cache: MeshCache`
  - `camera: Camera`
  - `input: InputState`
  - `time: TimeState`
  - `active_mesh: Option<MeshHandle>`

#### Core Systems Layer
- **Camera** (`src/camera.rs`): View, projection, transforms
- **Geometry** (`src/geometry/`): Vertex types and primitives
- **Mesh** (`src/mesh.rs`): Mesh loading and caching

#### External Dependencies
- **wgpu**: WebGPU implementation
- **winit**: Windowing and event loop
- **cgmath**: Math library
- **gltf**: Asset loading
- **pollster**: Async runtime
- **bytemuck**: Buffer utilities

---

## Core Components

### 1. Application Framework (`app.rs`)

The **Application** struct (new) and **App** struct (old) implement `winit::ApplicationHandler` and provide:

- Event loop management
- Window creation and lifecycle
- Graphics context initialization
- Renderer trait integration

**Key Trait:** `AppRenderer`

#### Methods
- `init(context: RenderContext<'_>)`: Async initialization
- `render(&mut self, context: RenderContext<'_>)`: Called on redraw requests
- `resize(&mut self, context: RenderContext<'_>, size: ...)`: Called on window resize
- `input(&mut self, context: RenderContext<'_>, event: &WindowEvent)`: Called on input events (default: no-op)
- `init_new(context: RenderContext<'_>)`: Async initialization (new architecture)
- `render_new(&mut self, context: RenderContext<'_>)`: Called on redraw requests
- `resize_new(&mut self, context: RenderContext<'_>, size: ...)`: Called on window resize
- `input_new(&mut self, context: RenderContext<'_>, event: &WindowEvent)`: Called on input events

**Note:** All methods use the new `RenderContext<'_>` parameter for accessing both GPU infrastructure and application state.

### 2. Graphics Device (`device.rs`) - NEW

The **GraphicsDevice** struct represents **immutable GPU infrastructure**:

```rust
pub struct GraphicsDevice {
    pub instance: wgpu::Instance,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface_config: SurfaceConfig,
    pub window: Arc<Window>,
}
```

**Key Features:**
- All fields are immutable after creation
- Can be shared across threads via `Arc`
- Provides convenience methods: `wgpu_device()`, `wgpu_queue()`, `surface_format()`, `size()`
- Handles surface recreation and configuration

### 3. Application State (`state.rs`) - NEW

The **AppState** struct represents **mutable application state**:

```rust
pub struct AppState {
    pub mesh_cache: MeshCache,
    pub camera: Camera,
    pub input: InputState,
    pub time: TimeState,
    pub active_mesh: Option<MeshHandle>,
}
```

**Key Features:**
- Contains all mutable data for the application
- `mesh_cache`: Central cache for mesh assets and GPU resources
- `camera`: Main camera for the scene
- `input`: Input state for tracking user input
- `time`: Timing information
- Provides convenience methods for loading and accessing meshes

### 4. Render Context (`context.rs`) - ENHANCED

The **RenderContext** struct provides **temporary access** to both infrastructure and state:

```rust
pub struct RenderContext<'a> {
    device: &'a GraphicsDevice,      // Immutable infrastructure
    state: &'a mut AppState,         // Mutable state
    texture_view: Option<wgpu::TextureView>, // Current frame texture
}
```

**Key Methods:**
- `device()` / `wgpu_device()`: Access GPU device
- `wgpu_queue()`: Access GPU queue
- `state()`: Access mutable application state
- `texture_view()` / `get_texture_view()`: Get current texture view
- `take_texture_view()`: Take ownership of texture view
- `request_redraw()`: Request window redraw
- `size()`: Get current window size
- `surface_format()`: Get surface format



### 6. Device Helpers (`device_helpers.rs`)

Utility functions and builders for common wgpu operations:

- **Buffer Creation**: `create_buffer()`, `create_buffer_from_slice()`
- **Shader Management**: `load_shader_source()`, `create_shader_module()`
- **Pipeline Building**: `RenderPipelineBuilder` with fluent API
- **Bind Group Helpers**: `create_uniform_bind_group_layout()`, `create_uniform_bind_group()`
- **Depth Textures**: `create_depth_texture()`

### 7. Camera System (`camera.rs`)

Comprehensive camera and lighting support:

- **Camera**: Position, target, up vector, FOV, near/far planes
- **CameraUniform**: View, projection, and view-projection matrices for shaders
- **Transform**: Translation, rotation, scale with model matrix generation
- **GeometryUniform**: MVP and model matrices for vertex shading
- **Light**: Position and color for light sources
- **LightingUniform**: View position and array of lights for fragment shading

### 8. Geometry Module (`geometry/`)

Vertex types and primitive generators:

- **Vertex Types**: `PosColorVertex`, `PosColorNormalVertex`, `QuadVertex`
- **Primitive Generators**: `triangle_vertices()`, `cube_vertices()`
- **Vertex Buffer Layouts**: Automatic `desc()` methods for each vertex type

### 9. Mesh Loading (`mesh.rs`)

GLTF/GLB mesh loading and management:

- **Mesh**: Vertices, indices, bounding box, scale, center
- **BoundingBox**: Min/max calculations, scale factor, center point
- **MeshCache**: Central cache with source deduplication (NEW in Phase 2)
- **Mesh Loading**: `load()` (old), `load_mut()` (new, recommended)
- **Buffer Creation**: `create_buffers()` for GPU upload
- **Full-screen Quad**: `QuadVertex`, `quad_vertices_2d()`, `create_quad_buffer()`

### 10. Input Module (`input.rs`) - NEW

Input state tracking for frame-rate independent movement and controls:

- **InputState**: Currently pressed keys, mouse position, mouse buttons, scroll delta
- **InputController**: Tracks key states, handles window events, provides key queries
- **MouseDelta**: Mouse movement delta for a frame
- **MouseMode**: Normal (mouse look with Shift) or Grabbed (always on)

### 11. Player Module (`player.rs`) - NEW

First-person camera control system:

- **PlayerState**: Position, velocity, camera reference, movement settings
- **PlayerInput**: Movement directions and mouse delta for a frame
- **MovementSettings**: Speed, acceleration, deceleration, mouse sensitivity
- Provides smooth, frame-rate independent movement

### 12. Deferred Rendering (`deferred.rs`)

G-buffer management for deferred shading:

- **GBuffer**: Position, normal, and albedo textures with views
- **Bind Group Layout**: For accessing G-buffer in shaders
- **Color Formats**: Standard RGBA16Float for all G-buffer attachments
- **Resize Support**: Dynamic resizing of all G-buffer textures
- **Render Pass Helpers**: Color attachments and targets for geometry pass

---

## New Architecture: Radical Separation

### The Problem

The old architecture mixed **immutable GPU infrastructure** (device, queue) with **mutable application state** (mesh cache, camera) in the same `GraphicsContext` struct. This led to:

1. **Unclear ownership**: What should be mutable vs. immutable?
2. **Thread safety issues**: Hard to share GPU resources across threads
3. **Interior mutability**: Required `RefCell` for mutable access to state
4. **Confusing API**: Mixed concerns in method signatures

### The Solution

**Radical Separation** cleanly divides the architecture into:

```
┌─────────────────────────────────────────────────────────────┐
│                    IMMUTABLE (Never Changes)                    │
├─────────────────────────────────────────────────────────────┤
│  GraphicsDevice                                                    │
│  ├── instance: wgpu::Instance                                    │
│  ├── device: Arc<wgpu::Device>                                   │
│  ├── queue: Arc<wgpu::Queue>                                     │
│  ├── surface_config: SurfaceConfig                              │
│  └── window: Arc<Window>                                          │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    MUTABLE (Changes Every Frame)                 │
├─────────────────────────────────────────────────────────────┤
│  AppState                                                         │
│  ├── mesh_cache: MeshCache                                       │
│  ├── camera: Camera                                              │
│  ├── input: InputState                                           │
│  ├── time: TimeState                                             │
│  └── active_mesh: Option<MeshHandle>                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    ACCESS PATTERN                                 │
├─────────────────────────────────────────────────────────────┤
│  RenderContext<'a>                                                │
│  ├── device: &'a GraphicsDevice  (immutable borrow)             │
│  └── state: &'a mut AppState     (mutable borrow)               │
└─────────────────────────────────────────────────────────────┘
```

### Benefits

1. **Clear Separation**: Easy to understand what's mutable vs. immutable
2. **Type Safety**: No interior mutability in core architecture
3. **Thread Safety**: GraphicsDevice can be shared via Arc
4. **Better Performance**: Direct access without RefCell overhead
5. **Maintainability**: Clearer code structure, easier to modify

---

## Rendering Paradigms

### Forward Rendering

```
Mesh Data → Vertex Shader → Fragment Shader → Framebuffer
         (MVP Matrix)   (Lighting)      (Final Color)
```

**Used in:** `forward.rs` demo  
**Pros:** Simple, single pass, good for small number of lights  
**Cons:** O(n*l) complexity where n=objects, l=lights

### Deferred Rendering

```
Phase 1 (Geometry Pass):
Mesh Data → Vertex Shader → Fragment Shader → G-Buffer
         (MVP Matrix)   (Output Position, Normal, Albedo)

Phase 2 (Lighting Pass):
Full-screen Quad → G-Buffer Sampling → Lighting Calculation → Framebuffer
                     (Read Position, Normal, Albedo)    (Final Color)
```

**Used in:** `deferred.rs` demo  
**Pros:** O(n + s) complexity where n=objects, s=screen pixels, efficient for many lights  
**Cons:** More memory usage (G-buffer storage), two passes required

---

## Data Flow

### Initialization Flow (New Architecture)

```
1. Event Loop Created (winit)
   ↓
2. Application::<R> Created
   ↓
3. Event Loop Resumed
   ↓
4. Window Created
   ↓
5. GraphicsDevice::new() - Creates immutable GPU infrastructure
   ↓
6. AppState::new() - Creates mutable application state
   ↓
7. Renderer::init_new(RenderContext) - Creates application-specific resources
   ↓
8. Window Request Redraw
   ↓
9. Render Loop Begins
```

### Render Frame Flow (New Architecture)

```
1. WindowEvent::RedrawRequested
   ↓
2. Application::window_event() → Renderer::render_new(RenderContext)
   ↓
3. RenderContext created with:
   - Borrow of GraphicsDevice
   - Mutable borrow of AppState
   - Current surface texture view
   ↓
4. Renderer accesses resources via context:
   - device = context.wgpu_device()
   - queue = context.wgpu_queue()
   - state = context.state()
   - texture_view = context.get_texture_view()
   ↓
5. Create Command Encoder
   ↓
6. Begin Render Pass
   ↓
7. Set Pipeline, Bind Groups, Buffers
   ↓
8. Draw Commands (draw, draw_indexed)
   ↓
9. End Render Pass
   ↓
10. Submit Command Buffer
    ↓
11. Present Surface Texture
```

### Resize Flow (New Architecture)

```
1. WindowEvent::Resized(new_size)
   ↓
2. GraphicsDevice::resize() - Updates surface config
   ↓
3. Renderer::resize_new(RenderContext, new_size) - Recreates size-dependent resources
   ↓
4. Request Redraw
```

---

## Design Philosophy

### 1. Minimal Abstraction

Renderlib provides thin wrappers over wgpu, exposing the underlying API when needed. This allows:
- Full access to WebGPU features
- Easy migration to raw wgpu
- No hidden performance overhead

### 2. Composition Over Inheritance

- Use Rust traits for polymorphism (`AppRenderer`)
- Prefer struct composition over trait inheritance
- Generic types for flexible integration

### 3. Framework, Not Engine

Renderlib is a **framework** that helps you build graphics applications, not a **game engine** that dictates architecture:
- You control the rendering pipeline
- You manage your resources
- You define your scene structure

### 4. Clear Separation of Concerns (NEW)

The Radical Separation architecture ensures:
- **Immutable infrastructure** is separate from **mutable state**
- **No interior mutability** in core types
- **Clear ownership** and lifetimes
- **Type safety** through Rust's type system

### 5. Development Experience

- **Hot Reloading**: Press 'R' to reload shaders without restarting
- **Clear Error Messages**: Panics with context, not cryptic wgpu errors
- **Sensible Defaults**: Cameras, transforms, and materials have reasonable defaults
- **Flexible Configuration**: Easy to customize, easy to replace

### 6. Performance Considerations

- **Buffer Management**: Uses `bytemuck` for safe buffer casting
- **Zero-cost Abstractions**: Helper functions compile away
- **Resource Reuse**: Bind group layouts and pipelines are created once
- **Async Initialization**: Supports async resource loading
- **No RefCell Overhead**: New architecture avoids interior mutability

---

## Next Steps

- [Module Documentation](./02-MODULES.md) - Detailed documentation for each of the 11 modules
- [Getting Started Guide](../guides/GETTING_STARTED.md) - Create your first renderlib application
- [API Reference](../api/REFERENCE.md) - Complete API documentation
- [Rendering Pipelines Guide](../guides/RENDERING.md) - Deep dive into rendering techniques
- [Architecture Migration Guide](../ARCHITECTURE_MIGRATION.md) - Migrate from old to new architecture
