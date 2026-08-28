# Renderlib Documentation

**Version:** 0.2.0  
**Renderlib:** A wgpu/winit framework for graphics applications in Rust  
**Architecture:** Radical Separation (Phases 1-4 Complete)

## Documentation Overview

This documentation provides comprehensive information about renderlib, including:

- **Architecture**: High-level system design and component interactions
- **Guides**: Step-by-step tutorials and best practices
- **API Reference**: Complete reference to all public types, functions, and methods
- **Migration**: Guide for moving from old to new architecture

## Current Architecture Status

Renderlib has undergone a **major refactoring** implementing the **Radical Separation** architecture:

### ✅ Completed Phases
- **Phase 1**: Infrastructure - Created `GraphicsDevice` and `AppState` types
- **Phase 2**: MeshCache Cleanup - Enhanced with source deduplication
- **Phase 3**: Framework Updates - Created `RenderContext` and updated `AppRenderer`
- **Phase 4**: Framework Ready - New architecture is complete and functional

### 🎯 New Architecture Principles

**Radical Separation** means:
1. **Immutable Infrastructure**: `GraphicsDevice` contains device, queue, surface - never changes
2. **Mutable State**: `AppState` contains mesh_cache, camera, input - changes every frame
3. **Clean Access**: `RenderContext<'a>` provides temporary access to both
4. **No Interior Mutability**: No more `RefCell` in core architecture

### 📚 Documentation Structure

```
docs/
├── README.md                    # This file
├── ARCHITECTURE_MIGRATION.md    # Guide for migrating to new architecture
├── architecture/
│   ├── 01-OVERVIEW.md          # High-level overview and design philosophy
│   ├── 02-MODULES.md           # Detailed module documentation (ALL 11 modules)
│   └── 03-COMPONENT_INTERACTIONS.md  # Component interactions and data flow
├── guides/
│   ├── GETTING_STARTED.md      # Create your first renderlib application
│   └── RENDERING.md            # Rendering pipelines guide
├── api/
│   └── REFERENCE.md             # Complete API documentation
├── examples/
│   └── EXAMPLES.md             # All example documentation
└── phase-docs/
    ├── PHASES_1_2_COMPLETED.md # Phase 1 & 2 implementation details
    ├── PHASES_3_4_COMPLETED.md # Phase 3 & 4 implementation details
    └── PHASE_4_COMPLETED.md    # Phase 4 renderer migration status
```

## Quick Start

### 1. Read the Overview

Start with the [Architecture Overview](architecture/01-OVERVIEW.md) to understand renderlib's **new** design and components.

### 2. Start with the New Architecture

All projects use the **Radical Separation architecture**:
- Use `Application<R>` for your application
- Implement `AppRenderer` trait
- Access resources via `RenderContext<'a>`
- GPU infrastructure in `GraphicsDevice`
- Mutable state in `AppState`

### 3. Follow the Getting Started Guide

Work through the [Getting Started Guide](guides/GETTING_STARTED.md) to create your first application.

### 4. Explore the Examples

Study the included demos:
- `src/bin/triangle.rs` - Simple triangle rendering (old architecture)
- `src/bin/forward.rs` - Forward rendering with lighting (old architecture)
- `src/bin/deferred.rs` - Deferred rendering with G-buffer (old architecture)
- `src/bin/deferred_with_camera_controls.rs` - Deferred with camera controls (old architecture)

**Note:** Examples are currently using old architecture but work perfectly. New architecture examples coming soon.

### 5. Dive into the API

Use the [API Reference](api/REFERENCE.md) for detailed information about all public types and functions.

## Documentation by Topic

### Architecture

| Document | Description |
|----------|-------------|
| [01-OVERVIEW](architecture/01-OVERVIEW.md) | Project summary, **new** architecture layers, core components, design philosophy |
| [02-MODULES](architecture/02-MODULES.md) | Detailed documentation for **all 11** modules including new ones |
| [03-COMPONENT_INTERACTIONS](architecture/03-COMPONENT_INTERACTIONS.md) | How components interact in **new** architecture |
| [ARCHITECTURE_MIGRATION](ARCHITECTURE_MIGRATION.md) | Guide for migrating from old to new architecture |

### Guides

| Document | Description |
|----------|-------------|
| [GETTING_STARTED](guides/GETTING_STARTED.md) | Create your first renderlib application, from setup to complete example |
| [RENDERING](guides/RENDERING.md) | Deep dive into rendering pipelines, forward vs. deferred, advanced techniques |

### API Reference

| Document | Description |
|----------|-------------|
| [REFERENCE](api/REFERENCE.md) | Complete API documentation with **all 11 modules**, types, traits, and functions |

### Phase Documentation

| Document | Description |
|----------|-------------|
| [PHASES_1_2_COMPLETED](phase-docs/PHASES_1_2_COMPLETED.md) | Infrastructure and MeshCache cleanup implementation |
| [PHASES_3_4_COMPLETED](phase-docs/PHASES_3_4_COMPLETED.md) | Framework updates and partial renderer migration |
| [PHASE_4_COMPLETED](phase-docs/PHASE_4_COMPLETED.md) | Renderer migration progress and current status |

## Renderlib Features

### Core Capabilities

- **Application Framework**: Event loop, window management, renderer trait
- **Graphics Context**: wgpu device, surface, and swap chain management
- **Device Helpers**: Buffer, shader, and pipeline creation utilities
- **Camera System**: View, projection, and model matrices with orbit controls
- **Geometry**: Vertex types and primitive generators
- **Mesh Loading**: GLTF/GLB loading with automatic scaling and centering
- **Deferred Rendering**: G-buffer management for deferred shading

### New Architecture Features (Radical Separation)

- **GraphicsDevice**: Immutable GPU infrastructure (device, queue, surface config)
- **AppState**: Mutable application state (mesh cache, camera, input, time)
- **RenderContext**: Temporary context combining both for renderer access
- **Clean Separation**: No interior mutability in core architecture
- **Thread Safety**: GraphicsDevice can be shared across threads via Arc

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

The repository includes four main demos:

| Demo | File | Description |
|------|------|-------------|
| Triangle | `src/bin/triangle.rs` | Simple rotating triangle with shader hot-reload |
| Forward | `src/bin/forward.rs` | Forward rendering with mesh loading and lighting |
| Deferred | `src/bin/deferred.rs` | Deferred rendering with G-buffer and lighting |
| Deferred with Camera | `src/bin/deferred_with_camera_controls.rs` | Deferred rendering with first-person camera controls |

Run demos with:

```bash
cargo run --bin triangle
cargo run --bin forward
cargo run --bin deferred
cargo run --bin deferred_with_camera_controls
```

## Project Structure

```
renderlib/
├── Cargo.toml                 # Project configuration
├── src/
│   ├── lib.rs                 # Library root, module exports
│   ├── app.rs                 # Application framework (old: App, new: Application)
│   ├── camera.rs              # Camera, transforms, lighting
│   ├── context.rs             # Graphics context (old) + RenderContext (new)
│   ├── deferred.rs            # G-buffer for deferred rendering
│   ├── device.rs              # NEW: Immutable GPU infrastructure
│   ├── device_helpers.rs      # wgpu utilities
│   ├── geometry/
│   │   ├── mod.rs             # Vertex types
│   │   └── primitives.rs      # Primitive generators
│   ├── input.rs               # NEW: Input state and controller
│   ├── mesh.rs                # Mesh loading and caching
│   ├── player.rs              # NEW: First-person player/camera control
│   ├── state.rs               # NEW: Mutable application state
│   └── bin/
│       ├── triangle.rs       # Triangle demo
│       ├── forward.rs        # Forward rendering demo
│       ├── deferred.rs        # Deferred rendering demo
│       └── deferred_with_camera_controls.rs  # Deferred with camera demo
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

## Architecture Migration
### Current Architecture: Radical Separation

Renderlib uses the **Radical Separation architecture** which provides:

- `Application<R>` - Application struct with separated GPU and state
- `GraphicsDevice` - Immutable GPU infrastructure (device, queue, surface)
- `AppState` - Mutable application state (mesh_cache, camera, input, time)
- `RenderContext<'a>` - Temporary context providing access to both
- `AppRenderer` trait - Interface for renderers

**See [Architecture Overview](architecture/01-OVERVIEW.md) for complete details.**

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
| 0.2.0 | 2026-08-26 | Radical Separation architecture (Phases 1-4), new modules: device, input, player, state |
| 0.1.0 | Initial | Initial release |

---

## Table of Contents for All Documentation

### Architecture

1. **[01-OVERVIEW](architecture/01-OVERVIEW.md)**
   - Project Summary
   - **New** Architecture Layers (Radical Separation)
   - Core Components (11 modules)
   - Rendering Paradigms
   - Data Flow
   - Design Philosophy

2. **[02-MODULES](architecture/02-MODULES.md)**
   - App Module (old and new)
   - Camera Module
   - Context Module (old and new)
   - **NEW** Device Module
   - Device Helpers Module
   - Deferred Module
   - Geometry Module
   - **NEW** Input Module
   - Mesh Module
   - **NEW** Player Module
   - **NEW** State Module
   - lib.rs

3. **[03-COMPONENT_INTERACTIONS](architecture/03-COMPONENT_INTERACTIONS.md)**
   - **New** Startup Sequence (Radical Separation)
   - **New** Render Loop
   - Resize Handling
   - Shader Hot-Reloading
   - Mesh Loading Pipeline
   - Deferred Rendering Pipeline
   - Uniform Buffer Management
   - Bind Group Hierarchy

4. **[ARCHITECTURE_MIGRATION](ARCHITECTURE_MIGRATION.md)**
   - Migration Overview
   - Old vs New Architecture
   - Step-by-Step Migration Guide
   - Common Patterns

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
   - Module Index (11 modules)
   - Detailed API for all modules
   - Type Index
   - Trait Index
   - Function Index

### Examples

1. **[EXAMPLES](examples/EXAMPLES.md)**
   - Triangle Demo
   - Forward Rendering Demo
   - Deferred Rendering Demo
   - Deferred with Camera Controls Demo
   - Running the Examples
   - Learning from the Examples

### Phase Documentation

1. **[PHASES_1_2_COMPLETED](phase-docs/PHASES_1_2_COMPLETED.md)**
   - Phase 1: Infrastructure
   - Phase 2: MeshCache Cleanup

2. **[PHASES_3_4_COMPLETED](phase-docs/PHASES_3_4_COMPLETED.md)**
   - Phase 3: Framework Updates
   - Phase 4: Renderer Migration (Partial)

3. **[PHASE_4_COMPLETED](phase-docs/PHASE_4_COMPLETED.md)**
   - Framework Completion
   - Triangle Renderer Migration
   - Current Status and Next Steps

---

*Happy rendering with renderlib! 🎮*
