# Renderlib Documentation

**Version:** 0.2.0  
**Renderlib:** A wgpu/winit framework for graphics applications in Rust

## Overview

Renderlib provides a foundation for building graphics applications with wgpu and winit. It handles window management, GPU resource initialization, rendering loops, and common graphics utilities.

**For API documentation, run `cargo doc --open`.**

## Documentation Structure

```
docs/
├── README.md                    # This file
├── architecture/
│   ├── 01-OVERVIEW.md          # Architecture and component overview
│   ├── 02-MODULES.md           # Module descriptions
│   └── 03-COMPONENT_INTERACTIONS.md  # Data flow and interactions
└── examples/
    └── EXAMPLES.md             # Example programs
```

## Quick Start

1. **Read** [Architecture Overview](architecture/01-OVERVIEW.md) for high-level design
2. **Follow** [Getting Started Guide](guides/GETTING_STARTED.md) to create your first app
3. **Browse** API docs with `cargo doc --open`
4. **Run** examples with `cargo run --bin triangle`

## Modules

| Module | Purpose |
|--------|---------|
| `app` | Application framework and renderer trait |
| `camera` | Camera, transforms, and lighting |
| `context` | Render context for resource access |
| `deferred` | G-buffer management for deferred rendering |
| `device` | GPU infrastructure (device, queue, surface) |
| `device_helpers` | Utilities for buffer, shader, and pipeline creation |
| `geometry` | Vertex types and primitive generators |
| `input` | Input state and event handling |
| `mesh` | Mesh loading and caching |
| `player` | First-person camera control |
| `state` | Application state management |

## Examples

| Example | Description |
|---------|-------------|
| `triangle` | Simple rotating triangle with shader hot-reload |
| `forward` | Forward rendering with mesh loading and lighting |
| `deferred` | Deferred rendering with G-buffer |
| `deferred_with_camera_controls` | Deferred rendering with camera controls |

## Project Structure

```
renderlib/
├── src/
│   ├── app.rs              # Application framework
│   ├── camera.rs           # Camera and lighting
│   ├── context.rs          # Render context
│   ├── deferred.rs         # Deferred rendering
│   ├── device.rs           # GPU infrastructure
│   ├── device_helpers.rs   # wgpu utilities
│   ├── geometry/          # Vertex types and primitives
│   ├── input.rs            # Input handling
│   ├── mesh.rs             # Mesh loading
│   ├── player.rs           # Camera control
│   ├── state.rs            # Application state
│   └── bin/               # Example programs
└── docs/                   # High-level documentation
```

## Dependencies

- `wgpu` 30 - WebGPU implementation
- `winit` 0.30 - Windowing and event loop
- `pollster` 1 - Async runtime
- `bytemuck` 1.16 - Buffer utilities
- `cgmath` 0.18 - Math library
- `gltf` 1.4.1 - GLTF/GLB loading

## Resources

- [wgpu Documentation](https://wgpu.rs/)
- [Learn WGPU](https://sotrh.github.io/learn-wgpu/)
- [Rust Graphics](https://rust-gpu.dev/)

## License

MIT License. See [LICENSE](../LICENSE) for details.
