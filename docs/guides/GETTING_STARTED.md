# Getting Started with Renderlib

This guide walks you through creating your first graphics application using renderlib.

## Prerequisites

- Rust 1.70+ (latest stable recommended)
- Cargo (comes with Rust)
- Vulkan SDK (Windows/Linux only)
- Git

Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Project Setup

### Using renderlib as a Dependency

Add to your `Cargo.toml`:
```toml
[dependencies]
renderlib = { git = "https://github.com/your-repo/renderlib" }
wgpu = "30"
winit = { version = "0.30", features = ["x11", "rwh_06"], default-features = false }
pollster = "1"
```

### Forking the Repository

```bash
git clone https://github.com/your-repo/renderlib.git
cd renderlib
```

## Creating a Simple Application

Create a renderer struct and implement `AppRenderer`:

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
        
        // Create vertex buffer
        let vertices = vec![/* your vertex data */];
        let vertex_buffer = renderlib::device_helpers::create_buffer_from_slice(
            device, &vertices, wgpu::BufferUsages::VERTEX, None
        );
        
        // Create render pipeline
        let shader = renderlib::device_helpers::create_shader_module_from_file(
            device, "shader.wgsl"
        ).unwrap();
        
        let render_pipeline = renderlib::device_helpers::RenderPipelineBuilder::new(device)
            .with_shader_module(shader)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(vec![MyVertex::desc()])
            .with_color_formats(vec![context.device().surface_format()])
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
    
    fn resize(&mut self, _context: RenderContext<'_>, _size: winit::dpi::PhysicalSize<u32>) {}
    fn input(&mut self, _context: RenderContext<'_>, _event: &winit::event::WindowEvent) {}
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = Application::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

## Adding a Mesh

Use `MeshCache` to load meshes:

```rust
async fn init(mut context: RenderContext<'_>) -> Self {
    let device = context.wgpu_device();
    
    // Load mesh
    use renderlib::mesh::MeshSource;
    let mesh_handle = context.state().mesh_cache.load_mut(
        &MeshSource::Path("mesh.gltf".to_string())
    ).unwrap();
    
    let (asset, resource) = context.state().mesh_cache.get_both(mesh_handle).unwrap();
    
    Self {
        vertex_buffer: resource.vertex_buffer,
        index_buffer: resource.index_buffer,
        num_indices: resource.num_indices,
        // ...
    }
}

fn render(&mut self, mut context: RenderContext<'_>) {
    // ...
    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
}
```

## Adding Camera Controls

Use `PlayerState` for first-person camera:

```rust
use renderlib::player::{PlayerState, PlayerInput};

struct MyRenderer {
    // ...
    player: PlayerState,
}

impl AppRenderer for MyRenderer {
    async fn init(mut context: RenderContext<'_>) -> Self {
        let camera = context.state().camera.clone();
        let player = PlayerState::new(camera);
        
        Self { player, /* ... */ }
    }
    
    fn input(&mut self, mut context: RenderContext<'_>, event: &winit::event::WindowEvent) {
        // Update input controller
        context.state().input.handle_window_event(event);
        
        // Get player input and apply
        let player_input = context.state().input.get_player_input();
        self.player.apply_input(&player_input, context.state().time.delta_time);
        
        // Update camera in state
        context.state().camera = self.player.get_camera().clone();
    }
    
    fn render(&mut self, mut context: RenderContext<'_>) {
        // Use updated camera
        let camera = &context.state().camera;
        // ...
    }
}
```

## Running Your Application

```bash
# Debug mode
cargo run

# Release mode (better performance)
cargo run --release

# With logging
RUST_LOG=debug cargo run
```

## Next Steps

- Run the examples: `cargo run --bin triangle`
- Read the [Architecture Overview](../architecture/01-OVERVIEW.md)
- Browse API docs: `cargo doc --open`
- Check out [Rendering Guide](RENDERING.md)
