# Renderlib Documentation

**Version:** 0.1.0  
**Renderlib:** A wgpu/winit framework for graphics applications in Rust

## Documentation Overview

This documentation provides comprehensive information about renderlib, including:

- **Architecture**: High-level system design and component interactions
- **Guides**: Step-by-step tutorials and best practices
- **API Reference**: Complete reference to all public types, functions, and methods

## Documentation Structure

```
docs/
├── README.md                    # This file
├── architecture/                # Architecture documentation
│   ├── 01-OVERVIEW.md          # High-level overview and design philosophy
│   ├── 02-MODULES.md           # Detailed module documentation
│   └── 03-COMPONENT_INTERACTIONS.md  # Component interactions and data flow
├── guides/                     # How-to guides and tutorials
│   ├── GETTING_STARTED.md      # Create your first renderlib application
│   └── RENDERING.md            # Rendering pipelines guide
└── api/                        # API reference
    └── REFERENCE.md             # Complete API documentation
```

## Quick Start

### 1. Read the Overview

Start with the [Architecture Overview](architecture/01-OVERVIEW.md) to understand renderlib's design and components.

### 2. Follow the Getting Started Guide

Work through the [Getting Started Guide](guides/GETTING_STARTED.md) to create your first application.

### 3. Explore the Examples

Study the included demos:
- `src/bin/triangle.rs` - Simple triangle rendering
- `src/bin/forward.rs` - Forward rendering with lighting
- `src/bin/deferred.rs` - Deferred rendering with G-buffer

### 4. Dive into the API

Use the [API Reference](api/REFERENCE.md) for detailed information about all public types and functions.

## Documentation by Topic

### Architecture

| Document | Description |
|----------|-------------|
| [01-OVERVIEW](architecture/01-OVERVIEW.md) | Project summary, architecture layers, core components, design philosophy |
| [02-MODULES](architecture/02-MODULES.md) | Detailed documentation for each module |
| [03-COMPONENT_INTERACTIONS](architecture/03-COMPONENT_INTERACTIONS.md) | How components interact during startup, rendering, resize, etc. |

### Guides

| Document | Description |
|----------|-------------|
| [GETTING_STARTED](guides/GETTING_STARTED.md) | Create your first renderlib application, from setup to complete example |
| [RENDERING](guides/RENDERING.md) | Deep dive into rendering pipelines, forward vs. deferred, advanced techniques |

### API Reference

| Document | Description |
|----------|-------------|
| [REFERENCE](api/REFERENCE.md) | Complete API documentation with all types, traits, and functions |

## Renderlib Features

### Core Capabilities

- **Application Framework**: Event loop, window management, renderer trait
- **Graphics Context**: wgpu device, surface, and swap chain management
- **Device Helpers**: Buffer, shader, and pipeline creation utilities
- **Camera System**: View, projection, and model matrices with orbit controls
- **Geometry**: Vertex types and primitive generators
- **Mesh Loading**: GLTF/GLB loading with automatic scaling and centering
- **Deferred Rendering**: G-buffer management for deferred shading

### Rendering Techniques

- **Forward Rendering**: Simple, single-pass rendering with lighting
- **Deferred Rendering**: Multi-pass rendering for efficient lighting
- **Hot Reloading**: Live shader reloading during development
- **Depth Testing**: Proper occlusion handling
- **Multiple Lights**: Support for up to 32 light sources

### Platform Support

- **Windows**: Vulkan, DirectX 12
- **macOS**: Metal
- **Linux**: Vulkan, OpenGL
- **Web**: WebGPU (via wasm)

## Examples

The repository includes three main demos:

| Demo | File | Description |
|------|------|-------------|
| Triangle | `src/bin/triangle.rs` | Simple rotating triangle with shader hot-reload |
| Forward | `src/bin/forward.rs` | Forward rendering with mesh loading and lighting |
| Deferred | `src/bin/deferred.rs` | Deferred rendering with G-buffer and lighting |

Run demos with:

```bash
cargo run --bin triangle
cargo run --bin forward
cargo run --bin deferred
```

## Project Structure

```
renderlib/
├── Cargo.toml                 # Project configuration
├── src/
│   ├── lib.rs                 # Library root, module exports
│   ├── app.rs                 # Application framework
│   ├── camera.rs              # Camera, transforms, lighting
│   ├── context.rs             # Graphics context
│   ├── deferred.rs            # G-buffer for deferred rendering
│   ├── device_helpers.rs      # wgpu utilities
│   ├── geometry/
│   │   ├── mod.rs             # Vertex types
│   │   └── primitives.rs      # Primitive generators
│   ├── mesh.rs                # Mesh loading
│   └── bin/
│       ├── triangle.rs       # Triangle demo
│       ├── forward.rs        # Forward rendering demo
│       └── deferred.rs        # Deferred rendering demo
├── assets/
│   └── README.md              # Asset directory documentation
└── docs/
    └── ...                    # This documentation
```

## Dependencies

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

## Contributing

### Documentation Contributions

To contribute to the documentation:

1. Fork the repository
2. Make changes to files in the `docs/` directory
3. Ensure links between documents are correct
4. Check that examples in documentation still work
5. Submit a pull request

### Code Contributions

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo test` to ensure nothing breaks
5. Update documentation if needed
6. Submit a pull request

### Documentation Standards

- Use consistent Markdown formatting
- Include code examples where helpful
- Link to related documentation
- Keep examples simple and focused
- Document both the "what" and the "why"

## Resources

### Learning Resources

- [wgpu Documentation](https://wgpu.rs/)
- [Rust Graphics](https://rust-gpu.dev/)
- [Learn WGPU](https://sotrh.github.io/learn-wgpu/)
- [cgmath Documentation](https://docs.rs/cgmath/latest/cgmath/)

### Community

- GitHub Issues: Report bugs and request features
- GitHub Discussions: Ask questions and share projects
- Rust Graphics Discord: Join the Rust graphics community

## License

Renderlib is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | Current | Initial release |

---

## Table of Contents for All Documentation

### Architecture

1. **[01-OVERVIEW](architecture/01-OVERVIEW.md)**
   - Project Summary
   - Architecture Layers
   - Core Components
   - Rendering Paradigms
   - Data Flow
   - Design Philosophy

2. **[02-MODULES](architecture/02-MODULES.md)**
   - App Module
   - Camera Module
   - Context Module
   - Device Helpers Module
   - Deferred Module
   - Geometry Module
   - Mesh Module
   - lib.rs

3. **[03-COMPONENT_INTERACTIONS](architecture/03-COMPONENT_INTERACTIONS.md)**
   - Startup Sequence
   - Render Loop
   - Resize Handling
   - Shader Hot-Reloading
   - Mesh Loading Pipeline
   - Deferred Rendering Pipeline
   - Uniform Buffer Management
   - Bind Group Hierarchy

### Guides

1. **[GETTING_STARTED](guides/GETTING_STARTED.md)**
   - Prerequisites
   - Project Setup
   - Creating a Simple Application
   - Understanding the App Structure
   - Adding a Mesh
   - Adding Camera Controls
   - Adding Lighting
   - Running Your Application
   - Next Steps

2. **[RENDERING](guides/RENDERING.md)**
   - Rendering Fundamentals
   - Forward Rendering
   - Deferred Rendering
   - Comparing Forward and Deferred
   - Implementing Custom Pipelines
   - Advanced Rendering Techniques
   - Performance Considerations

### API Reference

1. **[REFERENCE](api/REFERENCE.md)**
   - Crate Documentation
   - Module Index
   - Detailed API for all modules
   - Type Index
   - Trait Index
   - Function Index

---

*Happy rendering with renderlib! 🎮*
