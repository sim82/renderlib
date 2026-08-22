# Renderlib Architecture Overview

**Version:** 0.1.0  
**Language:** Rust  
**Primary Dependencies:** wgpu, winit, cgmath, gltf  

## Table of Contents

1. [Project Summary](#project-summary)
2. [Architecture Layers](#architecture-layers)
3. [Core Components](#core-components)
4. [Rendering Paradigms](#rendering-paradigms)
5. [Data Flow](#data-flow)
6. [Design Philosophy](#design-philosophy)

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

---

## Architecture Layers

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
│                      Framework Layer                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │
│  │    app.rs    │  │  context.rs  │  │   device_helpers.rs  │   │
│  │  (App<R>)    │  │ (Graphics-   │  │   (Buffer/Shader/P-  │   │
│  │              │  │   Context)   │  │    ipeline helpers)   │   │
│  └─────────────┘  └─────────────┘  └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
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
│                      Rendering Layer                            │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                    deferred.rs                             │  │
│  │              (GBuffer management)                          │  │
│  └─────────────────────────────────────────────────────────┘  │
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

---

## Core Components

### 1. Application Framework (`app.rs`)

The **App** struct implements `winit::ApplicationHandler` and provides:

- Event loop management
- Window creation and lifecycle
- Graphics context initialization
- Renderer trait integration

**Key Trait:** `AppRenderer`
- `init()`: Async initialization of rendering resources
- `render()`: Called on redraw requests
- `resize()`: Called on window resize
- `input()`: Called on input events

### 2. Graphics Context (`context.rs`)

The **GraphicsContext** struct encapsulates all wgpu resources needed for rendering:

- `wgpu::Instance`, `Adapter`, `Device`, `Queue`
- `Surface` and `SurfaceFormat` management
- Window reference and size tracking
- Surface configuration and resize handling
- Texture acquisition for rendering

### 3. Device Helpers (`device_helpers.rs`)

Utility functions and builders for common wgpu operations:

- **Buffer Creation**: `create_buffer()`, `create_buffer_from_slice()`
- **Shader Management**: `load_shader_source()`, `create_shader_module()`
- **Pipeline Building**: `RenderPipelineBuilder` with fluent API
- **Bind Group Helpers**: `create_uniform_bind_group_layout()`, `create_uniform_bind_group()`
- **Depth Textures**: `create_depth_texture()`

### 4. Camera System (`camera.rs`)

Comprehensive camera and lighting support:

- **Camera**: Position, target, up vector, FOV, near/far planes
- **CameraUniform**: View, projection, and view-projection matrices for shaders
- **Transform**: Translation, rotation, scale with model matrix generation
- **GeometryUniform**: MVP and model matrices for vertex shading
- **Light**: Position and color for light sources
- **LightingUniform**: View position and array of lights for fragment shading

### 5. Geometry Module (`geometry/`)

Vertex types and primitive generators:

- **Vertex Types**: `PosColorVertex`, `PosColorNormalVertex`, `QuadVertex`
- **Primitive Generators**: `triangle_vertices()`, `cube_vertices()`
- **Vertex Buffer Layouts**: Automatic `desc()` methods for each vertex type

### 6. Mesh Loading (`mesh.rs`)

GLTF/GLB mesh loading and management:

- **Mesh**: Vertices, indices, bounding box, scale, center
- **BoundingBox**: Min/max calculations, scale factor, center point
- **Mesh Loading**: `load_gltf()` with fallback to built-in primitives
- **Buffer Creation**: `create_buffers()` for GPU upload
- **Full-screen Quad**: `QuadVertex`, `quad_vertices_2d()`, `create_quad_buffer()`

### 7. Deferred Rendering (`deferred.rs`)

G-buffer management for deferred shading:

- **GBuffer**: Position, normal, and albedo textures with views
- **Bind Group Layout**: For accessing G-buffer in shaders
- **Color Formats**: Standard RGBA16Float for all G-buffer attachments
- **Resize Support**: Dynamic resizing of all G-buffer textures
- **Render Pass Helpers**: Color attachments and targets for geometry pass

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

### Initialization Flow

```
1. Event Loop Created (winit)
   ↓
2. App<Renderer> Created
   ↓
3. Event Loop Resumed
   ↓
4. Window Created
   ↓
5. GraphicsContext::new() - Creates wgpu resources
   ↓
6. Renderer::init() - Creates application-specific resources
   ↓
7. Window Request Redraw
   ↓
8. Render Loop Begins
```

### Render Frame Flow

```
1. WindowEvent::RedrawRequested
   ↓
2. App::window_event() → Renderer::render()
   ↓
3. Get Current Surface Texture
   ↓
4. Create Texture View
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

### Resize Flow

```
1. WindowEvent::Resized(new_size)
   ↓
2. GraphicsContext::resize() - Updates surface config
   ↓
3. Renderer::resize() - Recreates size-dependent resources
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

### 4. Development Experience

- **Hot Reloading**: Press 'R' to reload shaders without restarting
- **Clear Error Messages**: Panics with context, not cryptic wgpu errors
- **Sensible Defaults**: Cameras, transforms, and materials have reasonable defaults
- **Flexible Configuration**: Easy to customize, easy to replace

### 5. Performance Considerations

- **Buffer Management**: Uses `bytemuck` for safe buffer casting
- **Zero-cost Abstractions**: Helper functions compile away
- **Resource Reuse**: Bind group layouts and pipelines are created once
- **Async Initialization**: Supports async resource loading

---

## Next Steps

- [Module Documentation](./02-MODULES.md) - Detailed documentation for each module
- [Getting Started Guide](../guides/GETTING_STARTED.md) - Create your first renderlib application
- [API Reference](../api/REFERENCE.md) - Complete API documentation
- [Rendering Pipelines Guide](../guides/RENDERING.md) - Deep dive into rendering techniques
