# Component Interactions and Data Flow

This document describes how the various components of renderlib interact with each other during different phases of application execution.

## Table of Contents

1. [Startup Sequence](#1-startup-sequence)
2. [Render Loop](#2-render-loop)
3. [Resize Handling](#3-resize-handling)
4. [Shader Hot-Reloading](#4-shader-hot-reloading)
5. [Mesh Loading Pipeline](#5-mesh-loading-pipeline)
6. [Deferred Rendering Pipeline](#6-deferred-rendering-pipeline)
7. [Uniform Buffer Management](#7-uniform-buffer-management)
8. [Bind Group Hierarchy](#8-bind-group-hierarchy)

---

## 1. Startup Sequence

The following sequence diagram shows the complete startup process:

```
┌─────────────────────────────────────────────────────────────────┐
│                              main()                                 │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         env_logger::init()                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      EventLoop::new()                              │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   event_loop.set_control_flow()                    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         App::<R>::new()                             │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   event_loop.run_app(&mut app)                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      EventLoop Resumed                             │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  App::resumed(&mut self, event_loop: &ActiveEventLoop)       │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Window Creation                               │
│  event_loop.create_window(Window::default_attributes())          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    GraphicsContext::new()                          │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  1. Create wgpu::Instance with display handle                │  │
│  │  2. Request adapter (pollster::block_on)                     │  │
│  │  3. Request device and queue                                  │  │
│  │  4. Get window size                                           │  │
│  │  5. Create surface from window                                │  │
│  │  6. Get surface capabilities and format                       │  │
│  │  7. Create GraphicsContext struct                            │  │
│  │  8. Configure surface (first time)                            │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Renderer::init()                              │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  Async initialization:                                       │  │
│  │  - Load shaders (load_shader_source)                         │  │
│  │  - Create shader modules (create_shader_module)             │  │
│  │  - Load meshes (load_gltf or use primitives)                  │  │
│  │  - Create buffers (create_buffer, create_buffer_from_slice)  │  │
│  │  - Create textures (create_depth_texture)                     │  │
│  │  - Create bind group layouts                                 │  │
│  │  - Create bind groups                                         │  │
│  │  - Create pipelines (RenderPipelineBuilder)                  │  │
│  │  - Create cameras (Camera::new)                               │  │
│  │  - Create lights (Light::new)                                 │  │
│  │  - Create uniform buffers                                      │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      window.request_redraw()                       │
└─────────────────────────────────────────────────────────────────┘
```

### Code Flow

```rust
// In main()
let event_loop = EventLoop::new().unwrap();
event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
let mut app = App::<TriangleRenderer>::new();
event_loop.run_app(&mut app).unwrap();

// In App::resumed()
let window = Arc::new(event_loop.create_window(...).unwrap());
let context = pollster::block_on(GraphicsContext::new(...));
let renderer = pollster::block_on(R::init(&context));
self.context = Some(context);
self.renderer = Some(renderer);
window.request_redraw();
```

---

## 2. Render Loop

### Single Frame Rendering

```
┌─────────────────────────────────────────────────────────────────┐
│                     WindowEvent::RedrawRequested                     │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      App::window_event()                            │
│  - Calls renderer.input(&event) for input events                  │
│  - Matches on WindowEvent                                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Renderer::render()                             │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    GraphicsContext::get_current_texture()          │
│  - Tries to acquire surface texture                              │
│  - Handles various states (Success, Suboptimal, Outdated, Lost)    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    GraphicsContext::create_texture_view()          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Command Encoder Creation                       │
│  device.create_command_encoder(&Default::default())             │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Render Pass Setup                            │
│  encoder.begin_render_pass(&wgpu::RenderPassDescriptor {         │
│      color_attachments: [...],                                   │
│      depth_stencil_attachment: Some(...), // if depth enabled    │
│      ...                                                         │
│  })                                                               │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Draw Commands                               │
│  renderpass.set_pipeline(&self.render_pipeline);                  │
│  renderpass.set_bind_group(0, &self.uniform_bind_group, &[]);     │
│  renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); │
│  renderpass.set_index_buffer(...); // if indexed rendering       │
│  renderpass.draw(...) or renderpass.draw_indexed(...);          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                        End Render Pass                              │
│  drop(renderpass); // Ends the render pass                        │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Command Submission                          │
│  queue.submit([encoder.finish()]);                                │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Present Surface                              │
│  context.pre_present_notify();                                    │
│  queue.present(surface_texture);                                  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      context.request_redraw()                        │
│  - Request next frame (continuous rendering)                      │
└─────────────────────────────────────────────────────────────────┘
```

### Forward Rendering Example

```rust
// In ForwardRenderer::render()
fn render(&mut self, context: &mut GraphicsContext) {
    // 1. Handle shader reload
    if self.should_reload {
        self.reload_shader(&context.device).ok();
        self.should_reload = false;
    }
    
    // 2. Update uniforms
    let geometry_uniform = GeometryUniform::new(&self.camera, model, aspect);
    context.queue.write_buffer(&self.geometry_uniform_buffer, 0, 
        bytemuck::cast_slice(&[geometry_uniform]));
    
    // 3. Get surface texture
    let surface_texture = match context.get_current_texture() {
        Some(texture) => texture,
        None => return,
    };
    let texture_view = context.create_texture_view(&surface_texture);
    
    // 4. Create encoder and render pass
    let mut encoder = context.device.create_command_encoder(&Default::default());
    let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &texture_view,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            ..Default::default()
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            ..Default::default()
        }),
        ..Default::default()
    });
    
    // 5. Draw
    renderpass.set_pipeline(&self.render_pipeline);
    renderpass.set_bind_group(0, &self.uniform_bind_group, &[]);
    renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    renderpass.draw_indexed(0..self.num_indices, 0, 0..1);
    
    // 6. Submit and present
    context.queue.submit([encoder.finish()]);
    context.pre_present_notify();
    context.queue.present(surface_texture);
    context.request_redraw();
}
```

---

## 3. Resize Handling

### Resize Sequence

```
┌─────────────────────────────────────────────────────────────────┐
│                      WindowEvent::Resized(new_size)                  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      App::window_event()                            │
│  - Matches on WindowEvent::Resized                               │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    GraphicsContext::resize()                        │
│  - Updates self.size = new_size                                  │
│  - Calls self.configure_surface()                                 │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    GraphicsContext::configure_surface()              │
│  - Creates SurfaceConfiguration with new size                     │
│  - Calls surface.configure(&device, &surface_config)              │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Renderer::resize()                            │
│  - Recreates size-dependent resources                           │
│  - For deferred: resizes G-buffer and depth texture              │
│  - For forward: recreates depth texture                          │
└─────────────────────────────────────────────────────────────────┘
```

### Code Example

```rust
// In ForwardRenderer::resize()
fn resize(&mut self, context: &mut GraphicsContext, new_size: PhysicalSize<u32>) {
    // Recreate depth texture with new size
    let (depth_texture, depth_texture_view) = create_depth_texture(
        &context.device,
        new_size.width,
        new_size.height,
        Some("Depth Texture"),
    );
    self.depth_texture = depth_texture;
    self.depth_texture_view = depth_texture_view;
}

// In DeferredRenderer::resize()
fn resize(&mut self, context: &mut GraphicsContext, new_size: PhysicalSize<u32>) {
    // Resize G-buffer
    self.gbuffer.resize(&context.device, new_size.width, new_size.height);
    
    // Recreate depth texture
    let (depth_texture, depth_texture_view) = create_depth_texture(
        &context.device,
        new_size.width,
        new_size.height,
        Some("Deferred Depth Texture"),
    );
    self.depth_texture = depth_texture;
    self.depth_texture_view = depth_texture_view;
}
```

---

## 4. Shader Hot-Reloading

### Reload Sequence

```
┌─────────────────────────────────────────────────────────────────┐
│                    User Presses 'R' Key                             │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      WindowEvent::KeyboardInput                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Renderer::input()                              │
│  - Checks for 'R' key press                                       │
│  - Sets should_reload flag to true                                │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Renderer::render()                             │
│  - Checks should_reload flag                                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Renderer::reload_shader()                       │
│  - Calls load_shader_source() to read updated shader file         │
│  - Calls create_shader_module() with new source                   │
│  - Creates new pipeline using RenderPipelineBuilder               │
│  - Updates self.render_pipeline                                  │
│  - Clears should_reload flag                                       │
└─────────────────────────────────────────────────────────────────┘
```

### Code Example

```rust
// In TriangleRenderer
fn input(&mut self, event: &WindowEvent) {
    if let WindowEvent::KeyboardInput { event: key_event, .. } = event {
        if let Key::Character(c) = &key_event.logical_key {
            if c.to_ascii_lowercase() == "r" && key_event.state.is_pressed() {
                self.should_reload = true;
            }
        }
    }
}

fn render(&mut self, context: &mut GraphicsContext) {
    if self.should_reload {
        eprintln!("Reloading triangle shader...");
        if let Err(e) = self.reload_shader(&context.device) {
            eprintln!("Shader reload failed: {}", e);
        } else {
            eprintln!("Triangle shader reloaded successfully!");
        }
    }
    // ... rest of rendering
}

fn reload_shader(&mut self, device: &wgpu::Device) -> Result<(), String> {
    let shader_src = load_shader_source(SHADER_PATH)?;
    let shader_module = create_shader_module(device, Some("Triangle Shader"), &shader_src);
    
    let render_pipeline_layout = create_pipeline_layout(
        device,
        Some("Render Pipeline Layout"),
        &[Some(&self.bind_group_layout)],
    );
    
    let pipeline = RenderPipelineBuilder::new(device)
        .with_label(Some("Render Pipeline"))
        .with_layout(Some(&render_pipeline_layout))
        .with_shader_module(&shader_module)
        .with_vertex_entry("vs_main")
        .with_fragment_entry("fs_main")
        .with_vertex_buffers(&[Some(PosColorVertex::desc())])
        .with_color_formats(&[self.surface_format.add_srgb_suffix()])
        .build();
    
    self.render_pipeline = pipeline;
    self.should_reload = false;
    Ok(())
}
```

---

## 5. Mesh Loading Pipeline

### GLTF Loading Sequence

```
┌─────────────────────────────────────────────────────────────────┐
│                      load_gltf(path)                               │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Check File Extension                           │
│  - .glb: Read as bytes, then import_slice()                       │
│  - .gltf: Import directly (handles external buffers)             │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Parse GLTF Document                            │
│  - gltf::import() or gltf::import_slice()                         │
│  - Returns (document, buffers, images)                            │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Get First Mesh                                 │
│  - document.meshes().next()                                      │
│  - Returns Err(MeshLoadError::NoMeshesFound) if none             │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      First Pass: Collect Positions                  │
│  - Iterate through all primitives                                 │
│  - Read positions (required)                                       │
│  - Collect all positions for bounding box calculation            │
│  - Read indices (or generate if missing)                          │
│  - Adjust indices with vertex offset for multi-primitive meshes   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Second Pass: Build Vertices                    │
│  - Iterate through all primitives again                           │
│  - Read positions (required)                                       │
│  - Read normals (optional, default to [0,1,0])                    │
│  - Create PosColorNormalVertex with default color [0.8, 0.8, 0.8] │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Calculate Bounding Box                         │
│  - Find min/max for x, y, z from all positions                    │
│  - Calculate scale factor: 2.0 / max_dimension                     │
│  - Calculate center: (min + max) / 2                             │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Return Mesh Struct                             │
│  - vertices: Vec<PosColorNormalVertex>                           │
│  - indices: Vec<u16>                                               │
│  - bounding_box: BoundingBox                                       │
│  - scale: f32                                                     │
│  - center: Vector3<f32>                                           │
└─────────────────────────────────────────────────────────────────┘
```

### Fallback to Built-in Primitives

```rust
// In ForwardRenderer::init()
let mesh = match load_gltf(DEFAULT_MESH_PATH) {
    Ok(mesh) => mesh,
    Err(_) => {
        let (vertices, indices) = primitives::cube_vertices();
        Mesh::new(vertices, indices)
    }
};
```

### Buffer Creation

```
┌─────────────────────────────────────────────────────────────────┐
│                      Mesh::create_buffers()                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      create_buffer_from_slice()                      │
│  - For vertex buffer: wgpu::BufferUsages::VERTEX                │
│  - For index buffer: wgpu::BufferUsages::INDEX                   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Return (vertex_buffer, index_buffer, count)    │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Deferred Rendering Pipeline

### Two-Pass Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         DeferredRenderer::render()                   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Check and Handle Resize                         │
│  - If G-buffer size != context.size, resize G-buffer and depth     │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Update Uniform Buffers                          │
│  - Update geometry_uniform_buffer with MVP and model matrices     │
│  - Update lighting_uniform_buffer with camera and lights          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Get Surface Texture                            │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Create Command Encoder                          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      GEOMETRY PASS                                  │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  1. Begin render pass with G-buffer color attachments         │  │
│  │     - position_view, normal_view, albedo_view as targets      │  │
│  │     - depth_texture_view as depth attachment                 │  │
│  │  2. Set geometry pipeline                                       │  │
│  │  3. Set geometry bind group (MVP matrices)                     │  │
│  │  4. Set vertex and index buffers                                │  │
│  │  5. Draw indexed (mesh rendering)                               │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      LIGHTING PASS                                  │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  1. Create G-buffer bind group (if not cached)                │  │
│  │  2. Begin render pass with surface view as target              │  │
│  │  3. Set lighting pipeline                                       │  │
│  │  4. Set G-buffer bind group (bindings 0-3)                     │  │
│  │  5. Set lighting uniform bind group (binding 4)                │  │
│  │  6. Set quad vertex buffer                                       │  │
│  │  7. Draw (full-screen quad, 6 vertices)                         │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Submit and Present                             │
└─────────────────────────────────────────────────────────────────┘
```

### G-Buffer Bind Group Layout

```rust
// In GBuffer::bind_group_layout()
pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("GBuffer Bind Group Layout"),
        entries: &[
            // Binding 0: Position texture
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Binding 1: Normal texture
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Binding 2: Albedo texture
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Binding 3: Sampler
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}
```

---

## 7. Uniform Buffer Management

### Uniform Update Pattern

All demos follow a consistent pattern for updating uniform buffers:

```
┌─────────────────────────────────────────────────────────────────┐
│                      Calculate Current State                         │
│  - elapsed time, model matrix, view matrix, etc.                   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Create Uniform Struct                          │
│  - GeometryUniform::new(&camera, model, aspect)                  │
│  - LightingUniform::new_with_lights(&camera, &lights)            │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Write to GPU Buffer                            │
│  queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniform])) │
└─────────────────────────────────────────────────────────────────┘
```

### Uniform Buffer Types

| Uniform Type | Size | Used In | Shader Stage |
|-------------|------|---------|--------------|
| `CameraUniform` | 192 bytes | Camera matrices | Vertex |
| `GeometryUniform` | 128 bytes | MVP and model matrices | Vertex |
| `LightingUniform` | Variable | View position and lights | Fragment |
| `Transform` | N/A (CPU only) | Model matrix generation | N/A |

### Lighting Uniform Layout

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniform {
    pub view_position: [f32; 4],      // 16 bytes
    pub num_lights: u32,               // 4 bytes
    pub _padding: [f32; 3],           // 12 bytes (alignment padding)
    pub lights: [Light; MAX_LIGHTS],  // 32 * MAX_LIGHTS bytes
}
```

Total size: 16 + 4 + 12 + (32 * MAX_LIGHTS) = 32 + (32 * MAX_LIGHTS) bytes

---

## 8. Bind Group Hierarchy

### Bind Group Organization

Each demo uses a different bind group organization based on its needs:

#### Triangle Demo (Simple)

```
Bind Group 0:
├── Binding 0: Uniform Buffer (rotation matrix)
    └── Visibility: Vertex
```

#### Forward Demo

```
Bind Group 0:
├── Binding 0: Geometry Uniform Buffer (MVP, model)
│   └── Visibility: Vertex
└── Binding 1: Lighting Uniform Buffer (view_position, lights)
    └── Visibility: Fragment
```

#### Deferred Demo

```
Geometry Pass:
  Bind Group 0:
  └── Binding 0: Geometry Uniform Buffer
      └── Visibility: Vertex

Lighting Pass:
  Bind Group 0:
  ├── Binding 0: Position Texture (GBuffer)
  │   └── Visibility: Fragment
  ├── Binding 1: Normal Texture (GBuffer)
  │   └── Visibility: Fragment
  ├── Binding 2: Albedo Texture (GBuffer)
  │   └── Visibility: Fragment
  └── Binding 3: Sampler
      └── Visibility: Fragment
  
  Bind Group 1:
  └── Binding 0: Lighting Uniform Buffer
      └── Visibility: Fragment
```

### Bind Group Layout Creation

```rust
// Simple uniform bind group layout
let layout = create_uniform_bind_group_layout(
    device,
    Some("Uniform Bind Group Layout"),
    wgpu::ShaderStages::VERTEX,
);

// Custom bind group layout with multiple entries
let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("Custom Bind Group Layout"),
    entries: &[
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ],
});
```

---

## Summary

The component interactions in renderlib follow consistent patterns:

1. **Initialization**: Async, bottom-up (context first, then renderer)
2. **Rendering**: Top-down (renderer controls the flow)
3. **Resize**: Context first, then renderer for size-dependent resources
4. **Hot-Reloading**: Flag-based, checked at start of render
5. **Mesh Loading**: Two-pass (positions first for bounding box, then full vertices)
6. **Deferred Rendering**: Two-pass (geometry then lighting)
7. **Uniform Updates**: Calculate → Create struct → Write to GPU
8. **Bind Groups**: Organized by frequency of change and shader stage

These patterns ensure consistent behavior across all demos and make it easy to create new renderers.
