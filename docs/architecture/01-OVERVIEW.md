# Architecture Overview

## Design Philosophy

Renderlib is a **framework**, not an engine. It provides the foundation for building graphics applications while giving you full control over the rendering pipeline, resource management, and scene structure.

## Architecture Layers

```
Application Layer
    ↓
Framework Layer (Application, RenderContext)
    ↓
Infrastructure Layer (GraphicsDevice, AppState)
    ↓
Core Systems Layer (Camera, Geometry, Mesh)
    ↓
External Dependencies (wgpu, winit, cgmath)
```

## Core Components

### Framework
- **Application** - Manages the event loop, window, and renderer lifecycle
- **RenderContext** - Provides access to GPU resources and application state during rendering

### Infrastructure
- **GraphicsDevice** - Immutable GPU resources (device, queue, surface, window)
- **AppState** - Mutable application state (mesh cache, camera, input, time)

### Core Systems
- **Camera** - View and projection matrices, transforms
- **Geometry** - Vertex types and primitive generators
- **Mesh** - Mesh loading and caching
- **Deferred** - G-buffer for deferred rendering
- **Input** - Input state tracking
- **Player** - First-person camera control

### Utilities
- **Device Helpers** - Buffer, shader, and pipeline creation utilities

## Data Flow

### Initialization
```
Event Loop Resumed
    ↓
Window Created
    ↓
GraphicsDevice Initialized (device, queue, surface)
    ↓
AppState Created (mesh cache, camera, input)
    ↓
Renderer Initialized
    ↓
Request Redraw
```

### Render Frame
```
Redraw Requested
    ↓
Acquire Surface Texture
    ↓
Create RenderContext
    ↓
Renderer::render(RenderContext)
    ↓
Submit Commands
    ↓
Present
    ↓
Request Next Redraw (for animation)
```

### Resize
```
Window Resized
    ↓
GraphicsDevice::resize()
    ↓
Renderer::resize(RenderContext, new_size)
    ↓
Request Redraw
```

## Rendering Techniques

### Forward Rendering
Single-pass rendering: mesh data → vertex shader → fragment shader → framebuffer.

Good for: simple scenes, few light sources.

### Deferred Rendering
Two-pass rendering:
1. Geometry pass: render mesh to G-buffer (position, normal, albedo)
2. Lighting pass: apply lighting to full-screen quad

Good for: complex scenes, many light sources.

## Key Features

- **Hot Reloading** - Press 'R' to reload shaders without restarting
- **Thread Safety** - GPU resources can be shared across threads
- **Type Safety** - Clear separation between mutable and immutable state
- **Minimal Abstraction** - Thin wrappers over wgpu with full access to underlying API
