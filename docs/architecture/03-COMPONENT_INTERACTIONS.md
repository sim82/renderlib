# Component Interactions and Data Flow

**Version:** 0.2.0  
**Architecture:** Radical Separation (Phases 1-4 Complete)  
**Last Updated:** 2026-08-29

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

The startup sequence uses the new Radical Separation architecture. Here's how it works:

### New Architecture Startup Flow

```
1. Event Loop Created (winit)
   ↓
2. Application::<R> Created
   │
   ├── device: None
   ├── state: None
   ├── renderer: None
   └── window: None
   ↓
3. Event Loop Resumed
   ↓
4. ApplicationHandler::resumed() Called
   │
   ├── 4.1 Create Window
   │   └── window = Arc::new(event_loop.create_window(...))
   │
   ├── 4.2 Initialize GraphicsDevice (Async)
   │   └── device = GraphicsDevice::new(display_handle, window.clone()).await
   │       ├── Create wgpu::Instance
   │       ├── Request adapter
   │       ├── Request device and queue (wrapped in Arc)
   │       ├── Create surface
   │       ├── Get surface capabilities
   │       ├── Create SurfaceConfig
   │       └── Configure surface
   │
   ├── 4.3 Initialize AppState
   │   └── state = AppState::new(device.wgpu_device())
   │       ├── mesh_cache = MeshCache::new(device)
   │       ├── camera = Camera::default()
   │       ├── input = InputState::new()
   │       ├── time = TimeState::new()
   │       └── active_mesh = None
   │
   ├── 4.4 Initialize Renderer (Async)
   │   └── renderer = R::init(RenderContext).await
   │       └── RenderContext::new(&device, &mut state, None)
   │           ├── device: &GraphicsDevice
   │           ├── state: &mut AppState
   │           └── texture_view: None
   │
   └── 4.5 Store References
       ├── self.device = Some(device)
       ├── self.state = Some(state)
       ├── self.renderer = Some(renderer)
       └── self.window = Some(window)
   ↓
5. Window Request Redraw
   ↓
6. Render Loop Begins
```

### Code Flow

```rust
// In Application::<R>::resumed()
fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    // 1. Create window
    let window = Arc::new(
        event_loop
            .create_window(Window::default_attributes())
            .unwrap(),
    );

    // 2. Initialize GPU infrastructure
    let device = pollster::block_on(GraphicsDevice::new(
        event_loop.owned_display_handle(),
        window.clone(),
    ));

    // 3. Initialize application state
    let mut state = AppState::new(device.wgpu_device());

    // 4. Create render context for initialization
    let mut render_context = RenderContext::new(&device, &mut state, None);

    // 5. Initialize renderer
    let renderer = pollster::block_on(R::init(render_context));

    // 6. Store everything
    self.device = Some(device);
    self.state = Some(state);
    self.renderer = Some(renderer);
    self.window = Some(window);

    // 7. Request first redraw
    self.window.as_ref().unwrap().request_redraw();
}
```

---

## 2. Render Loop

The render loop has been **updated** to use the new `RenderContext` for accessing resources.

### Single Frame Rendering

```
1. WindowEvent::RedrawRequested Received
   ↓
2. ApplicationHandler::window_event() → Application::window_event()
   │
   ├── 2.1 Match on WindowEvent::RedrawRequested
   │   ↓
   │
   ├── 2.2 Get Current Surface Texture
   │   └── surface_texture = device.surface_config.get_current_texture(device.wgpu_device())
   │       └── Handles Suboptimal, Outdated, Lost cases automatically
   │
   ├── 2.3 Create Texture View
   │   └── texture_view = device.surface_config.create_texture_view(&surface_texture)
   │
   ├── 2.4 Create RenderContext
   │   └── render_context = self.create_render_context(Some(surface_texture))
   │       ├── device: &self.device
   │       ├── state: &mut self.state
   │       └── texture_view: Some(texture_view)
   │
   ├── 2.5 Call Renderer::render()
   │   └── self.renderer.as_mut().unwrap().render(render_context)
   │       └── Renderer accesses resources via context:
   │           ├── device = context.wgpu_device()
   │           ├── queue = context.wgpu_queue()
   │           ├── state = context.state()
   │           └── texture_view = context.get_texture_view()
   │
   └── 2.6 Present Surface Texture
       └── surface_texture.present()
           └── Handles presentation automatically
   ↓
3. Request Next Redraw (for animation)
   └── context.request_redraw()
```

### Renderer render() Method Example

```rust
fn render(&mut self, mut context: RenderContext<'_>) {
    // 1. Access resources
    let device = context.wgpu_device();
    let queue = context.wgpu_queue();
    let state = context.state();
    let texture_view = context.get_texture_view().expect("No texture view");
    
    // 2. Update time
    state.update_time();
    let delta_time = state.time.delta_time;
    
    // 3. Handle input (if needed)
    // Note: Input events are handled separately in input() method
    
    // 4. Update camera (if needed)
    // Camera updates would typically be in input() method
    
    // 5. Create command encoder
    let mut encoder = device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor::default()
    );
    
    // 6. Begin render pass
    let mut render_pass = encoder.begin_render_pass(
        &wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        }
    );
    
    // 7. Set pipeline and buffers
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    
    // 8. Draw
    render_pass.draw(0..3, 0..1);
    
    // 9. End render pass
    drop(render_pass);
    
    // 10. Submit command buffer
    queue.submit(std::iter::once(encoder.finish()));
    
    // 11. Request next redraw for animation
    context.request_redraw();
}
```

### Forward Rendering Example

Here's a more complete example showing forward rendering with the new architecture:

```rust
struct ForwardRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    geometry_uniform_buffer: wgpu::Buffer,
    geometry_bind_group: wgpu::BindGroup,
    lighting_uniform_buffer: wgpu::Buffer,
    lighting_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
}

impl AppRenderer for ForwardRenderer {
    async fn init(mut context: RenderContext<'_>) -> Self {
        let device = context.wgpu_device();
        let surface_format = context.device().surface_format();
        
        // Load mesh
        let mesh_handle = context.state().mesh_cache.load_mut(
            &MeshSource::Path("mesh.gltf".to_string())
        ).expect("Failed to load mesh");
        
        let (mesh_asset, mesh_resource) = context.state().mesh_cache.get_both(mesh_handle)
            .expect("Failed to get mesh");
        
        // Create depth texture
        let (depth_texture, depth_texture_view) = renderlib::device_helpers::create_depth_texture(
            device, context.size().width, context.size().height, Some("Depth Texture")
        );
        
        // Create render pipeline
        let shader_module = renderlib::device_helpers::create_shader_module_from_file(
            device, "forward.wgsl"
        ).expect("Failed to load shader");
        
        let render_pipeline = RenderPipelineBuilder::new(device)
            .with_label("Forward Pipeline")
            .with_shader_module(shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(vec![PosColorNormalVertex::desc()])
            .with_color_formats(vec![surface_format])
            .with_depth_stencil(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
            .build()
            .expect("Failed to create pipeline");
        
        // Create uniform buffers
        let geometry_uniform_buffer = renderlib::device_helpers::create_buffer(
            device,
            std::mem::size_of::<GeometryUniform>() as wgpu::BufferAddress,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            Some("Geometry Uniform Buffer")
        );
        
        let lighting_uniform_buffer = renderlib::device_helpers::create_buffer(
            device,
            std::mem::size_of::<LightingUniform>() as wgpu::BufferAddress,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            Some("Lighting Uniform Buffer")
        );
        
        // Create bind groups
        let geometry_bind_group_layout = renderlib::device_helpers::create_uniform_bind_group_layout(
            device, Some("Geometry Bind Group Layout")
        );
        let geometry_bind_group = renderlib::device_helpers::create_uniform_bind_group(
            device, &geometry_uniform_buffer, &geometry_bind_group_layout, Some("Geometry Bind Group")
        );
        
        let lighting_bind_group_layout = renderlib::device_helpers::create_uniform_bind_group_layout(
            device, Some("Lighting Bind Group Layout")
        );
        let lighting_bind_group = renderlib::device_helpers::create_uniform_bind_group(
            device, &lighting_uniform_buffer, &lighting_bind_group_layout, Some("Lighting Bind Group")
        );
        
        Self {
            render_pipeline,
            vertex_buffer: mesh_resource.vertex_buffer,
            index_buffer: mesh_resource.index_buffer,
            num_indices: mesh_resource.num_indices,
            geometry_uniform_buffer,
            geometry_bind_group,
            lighting_uniform_buffer,
            lighting_bind_group,
            depth_texture,
            depth_texture_view,
        }
    }

    fn render(&mut self, mut context: RenderContext<'_>) {
        let device = context.wgpu_device();
        let queue = context.wgpu_queue();
        let state = context.state();
        let texture_view = context.get_texture_view().expect("No texture view");
        
        // Update time
        state.update_time();
        let delta_time = state.time.delta_time;
        
        // Update geometry uniform
        let aspect_ratio = context.size().width as f32 / context.size().height as f32;
        let camera_uniform = CameraUniform::from_camera(&state.camera, aspect_ratio);
        let model_matrix = Matrix4::from_scale(0.5);
        let geometry_uniform = GeometryUniform {
            mvp: (camera_uniform.view_projection * model_matrix).into(),
            model: model_matrix.into(),
        };
        
        queue.write_buffer(
            &self.geometry_uniform_buffer,
            0,
            bytemuck::bytes_of(&geometry_uniform)
        );
        
        // Update lighting uniform
        let lighting_uniform = LightingUniform::new_with_lights(
            [state.camera.position.x, state.camera.position.y, state.camera.position.z],
            &[] // No lights for now
        );
        
        queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::bytes_of(&lighting_uniform)
        );
        
        // Create command encoder
        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default()
        );
        
        // Begin render pass with depth
        let mut render_pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Forward Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            }
        );
        
        // Set pipeline and bind groups
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.geometry_bind_group, &[]);
        render_pass.set_bind_group(1, &self.lighting_bind_group, &[]);
        
        // Set vertex and index buffers
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        
        // Draw
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        
        // End render pass
        drop(render_pass);
        
        // Submit command buffer
        queue.submit(std::iter::once(encoder.finish()));
        
        // Request next redraw
        context.request_redraw();
    }

    fn resize(&mut self, context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>) {
        let device = context.wgpu_device();
        
        // Recreate depth texture with new size
        let (new_depth_texture, new_depth_texture_view) = renderlib::device_helpers::create_depth_texture(
            device, new_size.width, new_size.height, Some("Depth Texture")
        );
        
        self.depth_texture = new_depth_texture;
        self.depth_texture_view = new_depth_texture_view;
    }

    fn input(&mut self, mut context: RenderContext<'_>, event: &WindowEvent) {
        let state = context.state();
        
        // Handle input events
        match event {
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if let Key::Character(c) = &key_event.logical_key {
                    if c.to_ascii_lowercase() == "r" && key_event.state.is_pressed() {
                        // Reload shaders
                        self.reload_shader(context);
                    }
                }
            }
            _ => {}
        }
        
        // Update input state
        state.input.handle_window_event(event);
        
        // Apply player input to camera
        let player_input = state.input.get_player_input();
        let mut player = renderlib::player::PlayerState::new(state.camera.clone());
        player.apply_input(&player_input, state.time.delta_time);
        state.camera = player.get_camera().clone();
    }
}
```

---

## 3. Resize Handling

Resize handling has been **updated** to work with the new architecture.

### Resize Sequence

```
1. WindowEvent::Resized(new_size) Received
   ↓
2. ApplicationHandler::window_event() → Application::window_event()
   │
   ├── 2.1 Match on WindowEvent::Resized(new_size)
   │   ↓
   │
   ├── 2.2 Resize GraphicsDevice Surface
   │   └── device.resize(new_size)
   │       └── surface_config.resize(new_size, device.wgpu_device())
   │           └── Reconfigures surface with new dimensions
   │
   ├── 2.3 Create RenderContext
   │   └── render_context = self.create_render_context(None)
   │       ├── device: &self.device
   │       ├── state: &mut self.state
   │       └── texture_view: None (no texture yet)
   │
   ├── 2.4 Call Renderer::resize()
   │   └── self.renderer.as_mut().unwrap().resize(render_context, new_size)
   │       └── Renderer recreates size-dependent resources
   │
   └── 2.5 Request Redraw
       └── self.window.as_ref().unwrap().request_redraw()
```

### Code Example

```rust
fn resize(&mut self, mut context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>) {
    let device = context.wgpu_device();
    let state = context.state();
    
    // Update camera aspect ratio
    let aspect_ratio = new_size.width as f32 / new_size.height as f32;
    state.camera.set_fov(cgmath::Rad(std::f32::consts::PI / 3.0));
    // Note: Camera aspect ratio is handled in the projection matrix calculation
    
    // Recreate depth texture with new size
    let (new_depth_texture, new_depth_texture_view) = renderlib::device_helpers::create_depth_texture(
        device, new_size.width, new_size.height, Some("Depth Texture")
    );
    
    self.depth_texture = new_depth_texture;
    self.depth_texture_view = new_depth_texture_view;
    
    // Recreate G-buffer if using deferred rendering
    if let Some(gbuffer) = &mut self.gbuffer {
        gbuffer.resize(device, new_size.width, new_size.height);
    }
}
```

---

## 4. Shader Hot-Reloading

Shader hot-reloading allows you to **modify shaders without restarting** the application. This feature works with the new architecture.

### Reload Sequence

```
1. User Presses 'R' Key
   ↓
2. WindowEvent::KeyboardInput Received
   ↓
3. Application::input() → Renderer::input()
   │
   ├── 3.1 Check for 'R' key press
   │   └── if key == 'r' && pressed: reload_shader()
   │
   └── 3.2 Call reload_shader()
       │
       ├── 3.2.1 Load New Shader Source
       │   └── source = std::fs::read_to_string(shader_path)?
       │
       ├── 3.2.2 Create New Shader Module
       │   └── new_module = device.create_shader_module(&desc)?
       │
       ├── 3.2.3 Create New Pipeline
       │   └── new_pipeline = create_pipeline(device, new_module, ...)
       │
       └── 3.2.4 Replace Old Pipeline
           └── self.render_pipeline = new_pipeline
```

### Code Example

```rust
fn reload_shader(&mut self, mut context: RenderContext<'_>) {
    let device = context.wgpu_device();
    let surface_format = context.device().surface_format();
    
    // Try to reload shader
    match renderlib::device_helpers::create_shader_module_from_file(
        device,
        "forward.wgsl"
    ) {
        Ok(shader_module) => {
            // Create new pipeline with updated shader
            let new_pipeline = RenderPipelineBuilder::new(device)
                .with_label("Forward Pipeline (Reloaded)")
                .with_shader_module(shader_module)
                .with_vertex_entry("vs_main")
                .with_fragment_entry("fs_main")
                .with_vertex_buffers(vec![PosColorNormalVertex::desc()])
                .with_color_formats(vec![surface_format])
                .with_depth_stencil(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                })
                .build()
                .expect("Failed to create reloaded pipeline");
            
            // Replace old pipeline
            self.render_pipeline = new_pipeline;
            
            println!("Shader reloaded successfully!");
        }
        Err(e) => {
            eprintln!("Failed to reload shader: {:?}", e);
        }
    }
}

fn input(&mut self, mut context: RenderContext<'_>, event: &WindowEvent) {
    match event {
        WindowEvent::KeyboardInput { event: key_event, .. } => {
            if let Key::Character(c) = &key_event.logical_key {
                if c.to_ascii_lowercase() == "r" && key_event.state.is_pressed() {
                    self.reload_shader(context);
                }
            }
        }
        _ => {}
    }
    
    // Forward to input state
    context.state().input.handle_window_event(event);
}
```

---

## 5. Mesh Loading Pipeline

The mesh loading pipeline has been **enhanced** with source deduplication in Phase 2.

### GLTF Loading Sequence

```
1. Renderer Calls mesh_cache.load_mut()
   │
   ├── 1.1 Check Source-to-Handle Map
   │   └── if source_to_handle.contains(&source): return existing handle
   │
   ├── 1.2 Load Mesh from Source
   │   │
   │   ├── 1.2.1 Match on MeshSource
   │   │   ├── Path: Load from GLTF/GLB file
   │   │   │   └── load_gltf(path)
   │   │   │       ├── Open file
   │   │   │       ├── Parse GLTF
   │   │   │       ├── Extract meshes
   │   │   │       ├── Calculate bounding box
   │   │   │       ├── Calculate scale and center
   │   │   │       └── Return Mesh
   │   │   └── Primitive: Use built-in generator
   │   │       ├── Match on PrimitiveType
   │   │       │   ├── Triangle: triangle_vertices()
   │   │       │   └── Cube: cube_vertices()
   │   │       └── Create Mesh from vertices
   │   │
   │   └── 1.2.2 Create MeshAsset
   │       └── cpu_assets.insert(next_handle, Arc::new(mesh_asset))
   │
   ├── 1.3 Create GPU Resources
   │   └── gpu_resources.insert(next_handle, Arc::new(mesh_resource))
   │       └── create_buffers(device, &mesh.vertices, &mesh.indices)
   │
   ├── 1.4 Update Maps
   │   ├── source_to_handle.insert(source.clone(), next_handle)
   │   └── next_handle += 1
   │
   └── 1.5 Return Handle
       └── return Ok(next_handle)
```

### Fallback to Built-in Primitives

If GLTF loading fails or a primitive source is specified, the system falls back to built-in generators:

```rust
// In load_gltf or load method:
match source {
    MeshSource::Path(path) => {
        // Try to load from file
        match load_gltf(path) {
            Ok(mesh) => Ok(mesh),
            Err(_) => {
                // Fallback to built-in primitive based on path
                if path.contains("triangle") {
                    Ok(create_triangle_mesh())
                } else if path.contains("cube") {
                    Ok(create_cube_mesh())
                } else {
                    Err(MeshLoadError::ImportError(format!("Failed to load mesh from {}", path)))
                }
            }
        }
    }
    MeshSource::Primitive(primitive_type) => {
        match primitive_type {
            PrimitiveType::Triangle => Ok(create_triangle_mesh()),
            PrimitiveType::Cube => Ok(create_cube_mesh()),
        }
    }
}
```

### Buffer Creation

Once a mesh is loaded, its GPU buffers are created:

```rust
// In Mesh::create_buffers()
pub fn create_buffers(
    &self,
    device: &wgpu::Device,
) -> Result<(wgpu::Buffer, wgpu::Buffer), wgpu::BufferAsyncError> {
    // Create vertex buffer
    let vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: &self.vertices,
            usage: wgpu::BufferUsages::VERTEX,
        }
    );
    
    // Create index buffer
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: &self.indices,
            usage: wgpu::BufferUsages::INDEX,
        }
    );
    
    Ok((vertex_buffer, index_buffer))
}
```

---

## 6. Deferred Rendering Pipeline

Deferred rendering uses a **two-pass architecture** with the new `GBuffer` system.

### Two-Pass Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    DEFERRED RENDERING PIPELINE                   │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Phase 1: Geometry Pass                                        │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Input: Mesh vertices, MVP matrix                          │  │
│  │  Output: Position, Normal, Albedo to G-Buffer             │  │
│  │                                                         │  │
│  │  for each mesh:                                         │  │
│  │    Set geometry pipeline                                 │  │
│  │    Set vertex/index buffers                              │  │
│  │    Set geometry uniform (MVP matrix)                     │  │
│  │    Draw mesh → G-Buffer (3 textures)                      │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                               │
│  Phase 2: Lighting Pass                                       │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Input: G-Buffer textures, Light positions/colors         │  │
│  │  Output: Final color to framebuffer                       │  │
│  │                                                         │  │
│  │    Set lighting pipeline                                 │  │
│  │    Set full-screen quad vertex buffer                    │  │
│  │    Set G-Buffer bind group                               │  │
│  │    Set lighting uniform (view position, lights)           │  │
│  │    Draw full-screen quad → Framebuffer                    │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### G-Buffer Bind Group Layout

The G-Buffer uses a specific bind group layout for shader access:

```rust
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

### Geometry Pass Shader (deferred_geometry.wgsl)

```wgsl
// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) albedo: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> geometry: GeometryUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = geometry.mvp * vec4<f32>(in.position, 1.0);
    out.world_position = (geometry.model * vec4<f32>(in.position, 1.0)).xyz;
    out.world_normal = normalize((geometry.model * vec4<f32>(in.normal, 0.0)).xyz);
    out.albedo = in.color;
    return out;
}

// Fragment shader
@group(0) @binding(0)
var position_texture: texture_2d<f32>;
@group(0) @binding(1)
var normal_texture: texture_2d<f32>;
@group(0) @binding(2)
var albedo_texture: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Write to multiple render targets
    return vec4<f32>(in.world_position, 1.0);
    // Note: In WGSL, you need to use @location directives for each output
}
```

### Lighting Pass Shader (deferred_lighting.wgsl)

```wgsl
@group(0) @binding(0)
var gbuffer_position: texture_2d<f32>;
@group(0) @binding(1)
var gbuffer_normal: texture_2d<f32>;
@group(0) @binding(2)
var gbuffer_albedo: texture_2d<f32>;
@group(0) @binding(3)
var gbuffer_sampler: sampler;

@group(1) @binding(0)
var<uniform> lighting: LightingUniform;

@fragment
fn fs_main(@builtin(position) pixel_coord: vec2<u32>) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(f32(pixel_coord.x), f32(pixel_coord.y)) / 
             vec2<f32>(textureDimensions(gbuffer_position));
    
    // Sample G-buffer
    let position = textureSample(gbuffer_position, gbuffer_sampler, uv).xyz;
    let normal = textureSample(gbuffer_normal, gbuffer_sampler, uv).xyz;
    let albedo = textureSample(gbuffer_albedo, gbuffer_sampler, uv).xyz;
    
    // Calculate lighting
    let view_dir = normalize(lighting.view_position.xyz - position);
    let normal = normalize(normal);
    
    // Simple directional light (for demonstration)
    let light_dir = normalize(vec3<f32>(1.0, -1.0, -1.0));
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    // Combine with albedo
    let color = albedo * diffuse * vec3<f32>(1.0, 0.9, 0.8); // Light color
    
    return vec4<f32>(color, 1.0);
}
```

---

## 7. Uniform Buffer Management

Uniform buffers are used to pass data from CPU to GPU that changes frequently (e.g., every frame).

### Uniform Update Pattern

The typical pattern for updating uniform buffers:

```
1. Create Uniform Buffer (Once)
   │
   ├── Allocate buffer with UNIFORM | COPY_DST usage
   └── Store in renderer struct
   ↓
2. Update Uniform Data (Every Frame)
   │
   ├── Calculate new uniform values (matrices, positions, etc.)
   └── Write to buffer using queue.write_buffer()
   ↓
3. Use in Rendering
   │
   └── Buffer is bound to bind group, which is set in render pass
```

### Uniform Buffer Types

Renderlib provides several uniform types:

#### CameraUniform

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
    pub view_projection: [[f32; 4]; 4],
    pub view_position: [f32; 4],
}
```

#### GeometryUniform

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryUniform {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
}
```

#### LightingUniform

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniform {
    pub view_position: [f32; 4],
    pub num_lights: u32,
    pub _padding: [u32; 3],
    pub lights: [Light; MAX_LIGHTS],
}
```

### Lighting Uniform Layout

The lighting uniform is designed to work with the shader:

```wgsl
// In shader:
@group(1) @binding(0)
var<uniform> lighting: LightingUniform;

// Access in fragment shader:
let view_pos = lighting.view_position.xyz;
let num_lights = lighting.num_lights;
for (var i: u32 = 0; i < num_lights; i++) {
    let light = lighting.lights[i];
    // Use light.position and light.color
}
```

---

## 8. Bind Group Hierarchy

Bind groups organize resources (buffers, textures, samplers) for access in shaders. Each bind group corresponds to a bind group layout.

### Bind Group Organization

Different renderers use different bind group organizations:

#### Triangle Demo (Simple)

```
Bind Group 0:
├── Uniform Buffer: GeometryUniform (MVP matrix)
└── Usage: Vertex shader

Shader:
@group(0) @binding(0)
var<uniform> uniforms: GeometryUniform;
```

#### Forward Demo

```
Bind Group 0:
├── Uniform Buffer: GeometryUniform (MVP + Model matrices)
└── Usage: Vertex shader

Bind Group 1:
├── Uniform Buffer: LightingUniform (view position, lights)
└── Usage: Fragment shader

Shader:
@group(0) @binding(0)
var<uniform> geometry: GeometryUniform;

@group(1) @binding(0)
var<uniform> lighting: LightingUniform;
```

#### Deferred Demo

```
Geometry Pass:
├── Bind Group 0:
│   └── Uniform Buffer: GeometryUniform (MVP + Model matrices)
│
Deferred Lighting Pass:
├── Bind Group 0:
│   ├── Texture: Position (binding 0)
│   ├── Texture: Normal (binding 1)
│   ├── Texture: Albedo (binding 2)
│   └── Sampler: G-buffer sampler (binding 3)
│
└── Bind Group 1:
    └── Uniform Buffer: LightingUniform

Shader (Lighting Pass):
@group(0) @binding(0)
var gbuffer_position: texture_2d<f32>;
@group(0) @binding(1)
var gbuffer_normal: texture_2d<f32>;
@group(0) @binding(2)
var gbuffer_albedo: texture_2d<f32>;
@group(0) @binding(3)
var gbuffer_sampler: sampler;

@group(1) @binding(0)
var<uniform> lighting: LightingUniform;
```

### Bind Group Layout Creation

Bind group layouts define how resources are accessed in shaders:

```rust
// Create a bind group layout for a uniform buffer
let bind_group_layout = device.create_bind_group_layout(
    &wgpu::BindGroupLayoutDescriptor {
        label: Some("Uniform Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(std::mem::size_of::<CameraUniform>() as wgpu::BufferAddress),
            },
            count: None,
        }],
    }
);

// Create a bind group for a texture and sampler
let texture_bind_group_layout = device.create_bind_group_layout(
    &wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture Bind Group Layout"),
        entries: &[
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
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    }
);
```

---

## Summary

The Radical Separation architecture provides **clean, type-safe access** to all resources through the `RenderContext`:

| Resource | Old Architecture | New Architecture |
|----------|-----------------|------------------|
| Device | `context.device` | `context.wgpu_device()` |
| Queue | `context.queue` | `context.wgpu_queue()` |
| Mesh Cache | `context.mesh_cache` | `context.state().mesh_cache` |
| Camera | `context.camera` | `context.state().camera` |
| Input | `context.input` | `context.state().input` |
| Time | N/A | `context.state().time` |
| Surface Format | `context.surface_format` | `context.device().surface_format()` |
| Window Size | `context.size` | `context.device().size()` |

**All component interactions now flow through the `RenderContext`, providing a consistent, clean interface for renderers to access all necessary resources.**
