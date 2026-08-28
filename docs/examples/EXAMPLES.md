# Examples

## Overview

Renderlib includes four example programs demonstrating different aspects of the framework.

| Example | File | Description |
|---------|------|-------------|
| Triangle | `src/bin/triangle.rs` | Rotating triangle with shader hot-reload |
| Forward | `src/bin/forward.rs` | Forward rendering with mesh loading and lighting |
| Deferred | `src/bin/deferred.rs` | Deferred rendering with G-buffer |
| Deferred with Camera | `src/bin/deferred_with_camera_controls.rs` | Deferred rendering with first-person camera |

## Running Examples

```bash
# Run a specific example
cargo run --bin triangle
cargo run --bin forward
cargo run --bin deferred
cargo run --bin deferred_with_camera_controls

# Release mode for better performance
cargo run --release --bin forward

# With logging
RUST_LOG=debug cargo run --bin forward
```

## Triangle Demo

**Purpose:** Simplest example demonstrating basic rendering.

**Features:**
- Single colored triangle
- Rotation over time
- Shader hot-reload (press R)

**Key Concepts:**
- Application setup
- Vertex buffer creation
- Uniform buffers
- Render pipeline
- Shader usage

## Forward Rendering Demo

**Purpose:** Demonstrates mesh loading and forward rendering.

**Features:**
- Loads 3D mesh (GLTF/GLB or built-in cube)
- Forward rendering with lighting
- Depth testing
- Rotation animation
- Shader hot-reload

**Key Concepts:**
- Mesh loading and caching
- Forward rendering pipeline
- Depth buffer
- Camera system
- Lighting

## Deferred Rendering Demo

**Purpose:** Demonstrates deferred rendering technique.

**Features:**
- G-buffer (position, normal, albedo)
- Two-pass rendering
- Geometry pass to G-buffer
- Lighting pass with full-screen quad
- Shader hot-reload

**Key Concepts:**
- Deferred rendering
- G-buffer management
- Multi-pass rendering
- Texture sampling in shaders

## Deferred with Camera Controls Demo

**Purpose:** Extends deferred demo with camera controls.

**Features:**
- All deferred rendering features
- First-person camera
- WASD movement
- Mouse look
- Toggle mouse mode with tilde key (`)

**Key Concepts:**
- Camera control
- Input handling
- Frame-rate independent movement
- Mouse capture

## Keyboard Controls

| Key | Action | Examples |
|-----|--------|----------|
| R | Reload shaders | All |
| ` (tilde) | Toggle mouse mode | Deferred with Camera |
| W | Move forward | Deferred with Camera |
| A | Move left | Deferred with Camera |
| S | Move backward | Deferred with Camera |
| D | Move right | Deferred with Camera |
| Shift | Enable mouse look (Normal mode) | Deferred with Camera |

## Asset Files

Examples look for files in `assets/`:
```
assets/
├── meshes/     # GLTF/GLB files
└── shaders/    # WGSL shader files
```

If no mesh file is found, examples fall back to built-in primitives.

## Creating Your Own Example

1. Create file in `src/bin/`:
```bash
touch src/bin/my_example.rs
```

2. Add to `Cargo.toml`:
```toml
[[bin]]
name = "my_example"
path = "src/bin/my_example.rs"
```

3. Implement `AppRenderer`:
```rust
use renderlib::app::{AppRenderer, Application};
use renderlib::context::RenderContext;

struct MyRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
}

impl AppRenderer for MyRenderer {
    async fn init(mut context: RenderContext<'_>) -> Self {
        let device = context.wgpu_device();
        let surface_format = context.device().surface_format();
        
        // Create resources
        let vertex_buffer = renderlib::device_helpers::create_buffer_from_slice(
            device, &vertices, wgpu::BufferUsages::VERTEX, None
        );
        
        let shader = renderlib::device_helpers::create_shader_module_from_file(
            device, "my_shader.wgsl"
        ).unwrap();
        
        let render_pipeline = renderlib::device_helpers::RenderPipelineBuilder::new(device)
            .with_shader_module(shader)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(vec![MyVertex::desc()])
            .with_color_formats(vec![surface_format])
            .build()
            .unwrap();
        
        Self { render_pipeline, vertex_buffer }
    }
    
    fn render(&mut self, mut context: RenderContext<'_>) {
        let texture_view = context.get_texture_view().unwrap();
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
    
    fn resize(&mut self, _context: RenderContext<'_>, _size: PhysicalSize<u32>) {}
    fn input(&mut self, _context: RenderContext<'_>, _event: &WindowEvent) {}
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = Application::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

4. Run:
```bash
cargo run --bin my_example
```
