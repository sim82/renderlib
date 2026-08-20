# Getting Started with Renderlib

This guide will walk you through creating your first graphics application using renderlib.

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Project Setup](#2-project-setup)
3. [Creating a Simple Application](#3-creating-a-simple-application)
4. [Understanding the App Structure](#4-understanding-the-app-structure)
5. [Adding a Mesh](#5-adding-a-mesh)
6. [Adding Camera Controls](#6-adding-camera-controls)
7. [Adding Lighting](#7-adding-lighting)
8. [Running Your Application](#8-running-your-application)
9. [Next Steps](#9-next-steps)

---

## 1. Prerequisites

Before you begin, ensure you have the following installed:

### Rust Toolchain

- Rust 1.70 or later (recommended: latest stable)
- Cargo (comes with Rust)

Install Rust using [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Required Tools

- **Vulkan SDK** (for Vulkan backend on Windows/Linux)
  - Windows: Download from [LunarG](https://vulkan.lunarg.com/)
  - Linux: Install via package manager (e.g., `sudo apt install vulkan-tools libvulkan-dev`)
  - macOS: Not needed (uses Metal backend)

- **Git** (for cloning the repository)

### Platform-Specific Notes

| Platform | Backend | Additional Dependencies |
|----------|---------|------------------------|
| Windows | Vulkan, DX12 | Vulkan SDK |
| macOS | Metal | None (Xcode command line tools recommended) |
| Linux | Vulkan | Vulkan drivers, libvulkan-dev |
| Web | WebGPU | Browser with WebGPU support |

---

## 2. Project Setup

### Option A: Using renderlib as a Dependency

Add renderlib to your `Cargo.toml`:

```toml
[dependencies]
renderlib = { git = "https://github.com/sim82/renderlib" }
wgpu = "30"
winit = { version = "0.30", features = ["x11", "rwh_06"], default-features = false }
pollster = "1"
env_logger = "0.11"
```

### Option B: Forking the Repository

Clone the repository and work from there:

```bash
git clone https://github.com/sim82/renderlib.git
cd renderlib
```

Then add your own files in the `examples/` or `src/bin/` directories.

### Verify Your Setup

Try running one of the existing demos:

```bash
cargo run --bin triangle
cargo run --bin forward
cargo run --bin deferred
```

If these run successfully, your environment is properly configured.

---

## 3. Creating a Simple Application

Let's create a simple application that displays a colored triangle.

### Step 1: Create a New Binary

Create a new file `src/bin/my_app.rs`:

```rust
//! My first renderlib application

use winit::event_loop::EventLoop;
use renderlib::app::{App, AppRenderer};
use renderlib::context::GraphicsContext;

// We'll define our renderer next

fn main() {
    // Initialize logger
    env_logger::init();

    // Create event loop
    let event_loop = EventLoop::new().unwrap();
    
    // Use Poll control flow for smooth animation
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // Create and run our application
    let mut app = App::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

### Step 2: Define the Renderer

Add the renderer definition before `main()`:

```rust
use renderlib::geometry::{PosColorVertex, primitives};
use renderlib::device_helpers::*;

/// Our custom renderer
struct MyRenderer {
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    surface_format: wgpu::TextureFormat,
}

impl AppRenderer for MyRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Create vertex buffer from triangle primitive
        let vertex_buffer = create_buffer_from_slice(
            device,
            Some("Triangle Vertex Buffer"),
            primitives::triangle_vertices(),
            wgpu::BufferUsages::VERTEX,
        );

        // Load shader
        let shader_src = r#"
            // Vertex shader
            struct VertexInput {
                @location(0) position: vec3<f32>,
                @location(1) color: vec3<f32>,
            };
            
            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) color: vec3<f32>,
            };
            
            @vertex
            fn vs_main(
                model: VertexInput,
            ) -> VertexOutput {
                var out: VertexOutput;
                out.clip_position = vec4<f32>(model.position, 1.0);
                out.color = model.color;
                return out;
            }
            
            // Fragment shader
            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                return vec4<f32>(in.color, 1.0);
            }
        "#;

        let shader_module = create_shader_module(
            device,
            Some("My Shader"),
            shader_src,
        );

        // Create pipeline layout (no bind groups for this simple example)
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("My Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            },
        );

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("My Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[Some(PosColorVertex::desc())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: context.surface_format.add_srgb_suffix(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            },
        );

        MyRenderer {
            vertex_buffer,
            render_pipeline,
            surface_format: context.surface_format,
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Get current surface texture
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let texture_view = context.create_texture_view(&surface_texture);

        // Create command encoder
        let mut encoder = context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default(),
        );

        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            },
        );

        // Draw the triangle
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
        context.request_redraw();
    }

    fn resize(&mut self, _context: &mut GraphicsContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        // No size-dependent resources in this simple example
    }
}
```

### Step 3: Run Your Application

```bash
cargo run --bin my_app
```

You should see a window with a colorful triangle!

---

## 4. Understanding the App Structure

The basic structure of a renderlib application:

```
┌─────────────────────────────────────────────────────────────┐
│                         main()                                  │
│  1. Initialize logger (env_logger::init)                     │
│  2. Create event loop                                         │
│  3. Set control flow (Poll for games, Wait for apps)            │
│  4. Create App with your renderer                             │
│  5. Run event loop                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      App<YourRenderer>                         │
│  - Implements winit::ApplicationHandler                       │
│  - Manages GraphicsContext and YourRenderer lifecycle          │
│  - Handles window events (create, resize, close, redraw)      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      YourRenderer                               │
│  - Implements AppRenderer trait                                │
│  - Contains your rendering resources (buffers, pipelines)      │
│  - init(): Async initialization                                │
│  - render(): Draw a frame                                       │
│  - resize(): Handle window resize                               │
│  - input(): Handle input events (optional)                     │
└─────────────────────────────────────────────────────────────┘
```

### Key Concepts

1. **GraphicsContext**: Manages wgpu device, surface, and window
2. **AppRenderer**: Trait that your renderer must implement
3. **Resource Initialization**: Happens in `init()`, which is async
4. **Render Loop**: `render()` is called on every redraw request
5. **Event Handling**: Window events are forwarded to your renderer

---

## 5. Adding a Mesh

Let's modify our application to load and display a 3D mesh.

### Update the Renderer

```rust
use renderlib::mesh::{load_gltf, Mesh};
use renderlib::geometry::PosColorNormalVertex;
use cgmath::{Matrix4, Rad, Vector3};
use std::time::Instant;

struct MyRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    render_pipeline: wgpu::RenderPipeline,
    surface_format: wgpu::TextureFormat,
    
    // For animation
    start_time: Instant,
    
    // For mesh positioning
    model_scale: f32,
    mesh_center: Vector3<f32>,
}

impl AppRenderer for MyRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Try to load a mesh, fall back to cube
        let mesh = match load_gltf("assets/duck.glb") {
            Ok(mesh) => mesh,
            Err(_) => {
                // Use built-in cube
                let (vertices, indices) = primitives::cube_vertices();
                Mesh::new(vertices, indices)
            }
        };

        // Create buffers from mesh
        let (vertex_buffer, index_buffer, num_indices) = 
            mesh.create_buffers(device, Some("Mesh"));

        // Load shader (inline for simplicity)
        let shader_src = r#"
            struct CameraUniform {
                view_projection: mat4x4<f32>,
                model: mat4x4<f32>,
            };
            
            @group(0) @binding(0)
            var<uniform> camera: CameraUniform;
            
            struct VertexInput {
                @location(0) position: vec3<f32>,
                @location(1) color: vec3<f32>,
                @location(2) normal: vec3<f32>,
            };
            
            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) color: vec3<f32>,
            };
            
            @vertex
            fn vs_main(
                model: VertexInput,
            ) -> VertexOutput {
                var out: VertexOutput;
                out.clip_position = camera.view_projection * camera.model * vec4<f32>(model.position, 1.0);
                out.color = model.color;
                return out;
            }
            
            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                return vec4<f32>(in.color, 1.0);
            }
        "#;

        let shader_module = create_shader_module(device, Some("Mesh Shader"), shader_src);

        // Create uniform buffer for camera and model matrices
        let camera_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Uniform Buffer"),
                contents: bytemuck::cast_slice(&[CameraUniform {
                    view_projection: Matrix4::identity().into(),
                    model: Matrix4::identity().into(),
                }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            },
        );

        // Create bind group
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Camera Bind Group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                }],
            },
        );

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Mesh Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            },
        );

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Mesh Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[Some(PosColorNormalVertex::desc())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: context.surface_format.add_srgb_suffix(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            },
        );

        MyRenderer {
            vertex_buffer,
            index_buffer,
            num_indices,
            render_pipeline,
            surface_format: context.surface_format,
            start_time: Instant::now(),
            model_scale: mesh.scale,
            mesh_center: mesh.center,
            // We need to store these for later use
            camera_uniform_buffer,
            bind_group,
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // Calculate elapsed time for animation
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Create camera matrices
        let aspect = context.size.width as f32 / context.size.height as f32;
        let view = Matrix4::look_at_rh(
            cgmath::Point3::new(0.0, 0.0, 5.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::new(0.0, 1.0, 0.0),
        );
        let proj = cgmath::perspective(cgmath::Deg(45.0), aspect, 0.1, 100.0);
        let view_proj = proj * view;

        // Create model matrix with rotation and scaling
        let translation = Matrix4::from_translation(-self.mesh_center);
        let scale_matrix = Matrix4::from_scale(self.model_scale);
        let model = Matrix4::from_angle_y(Rad(elapsed * 0.5))
            * scale_matrix
            * translation;

        // Update uniform buffer
        context.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[CameraUniform {
                view_projection: view_proj.into(),
                model: model.into(),
            }]),
        );

        // Get current surface texture
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let texture_view = context.create_texture_view(&surface_texture);

        // Create command encoder
        let mut encoder = context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default(),
        );

        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            },
        );

        // Draw the mesh
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
        context.request_redraw();
    }

    // ... resize() remains the same
}

// Add CameraUniform for the shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}
```

### Using the Camera Module

Instead of manually creating camera matrices, you can use renderlib's `Camera` type:

```rust
use renderlib::camera::{Camera, CameraUniform as CameraUniformLib};

// In your renderer struct
struct MyRenderer {
    // ...
    camera: Camera,
}

// In init()
let camera = Camera::new();

// In render()
aspect = context.size.width as f32 / context.size.height as f32;
let camera_uniform = CameraUniformLib::from_camera(&self.camera, aspect);

context.queue.write_buffer(
    &self.camera_uniform_buffer,
    0,
    bytemuck::cast_slice(&[camera_uniform]),
);
```

---

## 6. Adding Camera Controls

Let's add keyboard controls to move the camera around.

### Update the Renderer

```rust
use winit::event::WindowEvent;
use winit::keyboard::Key;

struct MyRenderer {
    // ... existing fields
    camera: Camera,
    camera_speed: f32,
}

impl AppRenderer for MyRenderer {
    // ... existing methods

    fn input(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput { event: key_event, .. } = event {
            // Handle 'R' key for shader reload
            if let Key::Character(c) = &key_event.logical_key {
                if c.to_ascii_lowercase() == "r" && key_event.state.is_pressed() {
                    // You could add shader reload here
                }
            }
            
            // Handle WASD keys for camera movement
            if key_event.state.is_pressed() {
                let move_speed = self.camera_speed;
                let forward = self.camera.get_forward();
                let right = self.camera.get_right();
                
                match &key_event.logical_key {
                    Key::Character(c) if c.to_ascii_lowercase() == "w" => {
                        self.camera.translate(forward * move_speed);
                    }
                    Key::Character(c) if c.to_ascii_lowercase() == "s" => {
                        self.camera.translate(-forward * move_speed);
                    }
                    Key::Character(c) if c.to_ascii_lowercase() == "a" => {
                        self.camera.translate(-right * move_speed);
                    }
                    Key::Character(c) if c.to_ascii_lowercase() == "d" => {
                        self.camera.translate(right * move_speed);
                    }
                    _ => {}
                }
            }
        }
    }
}

// In init()
MyRenderer {
    // ...
    camera: Camera::new(),
    camera_speed: 0.1,
}
```

### Adding Mouse Look

For mouse look, you'll need to track mouse position and calculate rotation:

```rust
use winit::event::MouseButton;

struct MyRenderer {
    // ...
    last_mouse_pos: Option<(f64, f64)>,
    mouse_sensitivity: f32,
}

impl AppRenderer for MyRenderer {
    fn input(&mut self, event: &WindowEvent) {
        // ... keyboard handling

        if let WindowEvent::MouseInput { button, state, .. } = event {
            if *button == MouseButton::Left {
                if *state.is_pressed() {
                    // Capture mouse on left click
                    // You might want to hide the cursor here
                } else {
                    // Release mouse
                    self.last_mouse_pos = None;
                }
            }
        }

        if let WindowEvent::CursorMoved { position, .. } = event {
            if let Some(last_pos) = self.last_mouse_pos {
                let delta_x = position.x - last_pos.0;
                let delta_y = position.y - last_pos.1;
                
                // Calculate rotation
                let yaw = delta_x as f32 * self.mouse_sensitivity;
                let pitch = delta_y as f32 * self.mouse_sensitivity;
                
                // Orbit camera around target
                self.camera.orbit(yaw, pitch);
            }
            self.last_mouse_pos = Some((position.x, position.y));
        }
    }
}
```

---

## 7. Adding Lighting

Let's add simple lighting to our mesh using renderlib's lighting system.

### Update the Shader

```rust
let shader_src = r#"
    struct CameraUniform {
        view_projection: mat4x4<f32>,
        model: mat4x4<f32>,
    };
    
    struct LightingUniform {
        view_position: vec4<f32>,
        num_lights: u32,
        _padding: vec3<f32>,
        lights: array<Light, 32>,
    };
    
    struct Light {
        position: vec4<f32>,
        color: vec4<f32>,
    };
    
    @group(0) @binding(0)
    var<uniform> camera: CameraUniform;
    
    @group(0) @binding(1)
    var<uniform> lighting: LightingUniform;
    
    struct VertexInput {
        @location(0) position: vec3<f32>,
        @location(1) color: vec3<f32>,
        @location(2) normal: vec3<f32>,
    };
    
    struct VertexOutput {
        @builtin(position) clip_position: vec4<f32>,
        @location(0) color: vec3<f32>,
        @location(1) world_position: vec3<f32>,
        @location(2) world_normal: vec3<f32>,
    };
    
    @vertex
    fn vs_main(
        model: VertexInput,
    ) -> VertexOutput {
        var out: VertexOutput;
        let world_pos = vec3<f32>((camera.model * vec4<f32>(model.position, 1.0)).xyz);
        out.clip_position = camera.view_projection * vec4<f32>(world_pos, 1.0);
        out.color = model.color;
        out.world_position = world_pos;
        out.world_normal = mat3<f32>(camera.model) * model.normal;
        return out;
    }
    
    @fragment
    fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
        let mut color = in.color;
        
        // Simple diffuse lighting
        let normal = normalize(in.world_normal);
        let view_dir = normalize(lighting.view_position.xyz - in.world_position);
        
        let mut diffuse = vec3<f32>(0.0);
        for (var i: u32 = 0; i < lighting.num_lights; i = i + 1) {
            let light_pos = lighting.lights[i].position.xyz;
            let light_color = lighting.lights[i].color.xyz;
            
            let light_dir = normalize(light_pos - in.world_position);
            let diffuse_factor = max(dot(normal, light_dir), 0.0);
            diffuse = diffuse + light_color * diffuse_factor;
        }
        
        // Combine with base color
        color = color * (diffuse + 0.1); // Add ambient light
        
        return vec4<f32>(color, 1.0);
    }
"#;
```

### Update the Renderer

```rust
use renderlib::camera::{Light, LightingUniform};

struct MyRenderer {
    // ... existing fields
    lighting_uniform_buffer: wgpu::Buffer,
    lights: [Light; renderlib::camera::MAX_LIGHTS],
    num_lights: u32,
}

impl AppRenderer for MyRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        // ... existing initialization

        // Create lights
        let mut lights = [Light::default(); renderlib::camera::MAX_LIGHTS];
        lights[0] = Light::new([2.0, 3.0, 4.0], [1.0, 1.0, 1.0]); // White light
        lights[1] = Light::new([-3.0, 2.0, 2.0], [1.0, 0.0, 0.0]); // Red light
        let num_lights = 2;

        // Create lighting uniform buffer
        let camera_for_lighting = Camera::new(); // Temporary camera
        let lighting_uniform = LightingUniform::new_with_lights(
            &camera_for_lighting,
            &lights[..num_lights as usize]
        );
        let lighting_uniform_buffer = create_buffer(
            device,
            Some("Lighting Uniform Buffer"),
            &lighting_uniform,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Update bind group layout to include lighting
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[
                    // Camera uniforms (vertex stage)
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
                    // Lighting uniforms (fragment stage)
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
            },
        );

        // Update bind group
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Uniform Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: lighting_uniform_buffer.as_entire_binding(),
                    },
                ],
            },
        );

        MyRenderer {
            // ... existing fields
            lighting_uniform_buffer,
            lights,
            num_lights,
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        // ... existing camera update code

        // Update lighting uniform
        let lighting_uniform = LightingUniform::new_with_lights(
            &self.camera,
            &self.lights[..self.num_lights as usize]
        );
        context.queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting_uniform]),
        );

        // ... rest of render()
    }
}
```

---

## 8. Running Your Application

### Running the Application

```bash
cargo run --bin my_app
```

### Build Modes

- **Debug**: `cargo run` - Slower but with better error messages
- **Release**: `cargo run --release` - Optimized for performance

### Troubleshooting

| Issue | Solution |
|-------|----------|
| Window doesn't open | Check Vulkan/Metal drivers |
| Black screen | Check shader compilation errors |
| Mesh not loading | Verify GLTF file path, try built-in cube |
| Shader errors | Check WGSL syntax, use `cargo run --bin triangle` to verify basic rendering works |

### Logging

Renderlib uses `env_logger` for logging. Set the log level:

```rust
// In main(), before running the app
std::env::set_var("RUST_LOG", "debug");
env_logger::init();
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

---

## 9. Next Steps

Now that you've created your first renderlib application, here are some next steps:

### Explore the Examples

- `triangle.rs`: Simple triangle rendering
- `forward.rs`: Forward rendering with mesh loading and lighting
- `deferred.rs`: Deferred rendering with G-buffer

### Learn More

- [Architecture Documentation](../architecture/01-OVERVIEW.md): Understand renderlib's design
- [Module Documentation](../architecture/02-MODULES.md): Detailed module reference
- [Component Interactions](../architecture/03-COMPONENT_INTERACTIONS.md): How components work together

### Advanced Topics

1. **Deferred Rendering**: Implement your own deferred renderer
2. **Post-processing**: Add bloom, SSR, or other effects
3. **Shadow Mapping**: Add real-time shadows
4. **PBR Materials**: Implement physically-based rendering
5. **Animation**: Load and play animated GLTF models
6. **Instanced Rendering**: Render many instances efficiently
7. **Compute Shaders**: Use GPU compute for particle systems, etc.

### Contributing

If you find bugs or want to add features:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo test` to ensure nothing breaks
5. Submit a pull request

### Community

- GitHub Issues: Report bugs and request features
- GitHub Discussions: Ask questions and share projects

---

## Complete Example

Here's the complete code for a simple mesh viewer with camera controls:

```rust
//! Simple mesh viewer with camera controls

use std::time::Instant;

use cgmath::{Matrix4, Rad, Vector3};
use winit::{
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::Key,
};

use renderlib::{
    app::{App, AppRenderer},
    camera::{Camera, CameraUniform, Light, LightingUniform, MAX_LIGHTS},
    context::GraphicsContext,
    device_helpers::*,
    geometry::{primitives, PosColorNormalVertex},
    mesh::{load_gltf, Mesh},
};

const MESH_PATH: &str = "assets/duck.glb";

struct MeshViewer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    camera_uniform_buffer: wgpu::Buffer,
    lighting_uniform_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    surface_format: wgpu::TextureFormat,
    
    camera: Camera,
    lights: [Light; MAX_LIGHTS],
    num_lights: u32,
    
    start_time: Instant,
    model_scale: f32,
    mesh_center: Vector3<f32>,
    camera_speed: f32,
}

impl AppRenderer for MeshViewer {
    async fn init(context: &GraphicsContext) -> Self {
        let device = &context.device;

        // Load mesh
        let mesh = match load_gltf(MESH_PATH) {
            Ok(mesh) => mesh,
            Err(_) => {
                let (vertices, indices) = primitives::cube_vertices();
                Mesh::new(vertices, indices)
            }
        };

        // Create buffers
        let (vertex_buffer, index_buffer, num_indices) = 
            mesh.create_buffers(device, Some("Mesh"));

        // Create camera
        let camera = Camera::new();

        // Create lights
        let mut lights = [Light::default(); MAX_LIGHTS];
        lights[0] = Light::new([2.0, 3.0, 4.0], [1.0, 1.0, 1.0]);
        lights[1] = Light::new([-3.0, 2.0, 2.0], [1.0, 0.0, 0.0]);
        let num_lights = 2;

        // Create uniform buffers
        let aspect = context.size.width as f32 / context.size.height as f32;
        let camera_uniform = CameraUniform::from_camera(&camera, aspect);
        let camera_uniform_buffer = create_buffer(
            device,
            Some("Camera Uniform Buffer"),
            &camera_uniform,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let lighting_uniform = LightingUniform::new_with_lights(&camera, &lights[..num_lights as usize]);
        let lighting_uniform_buffer = create_buffer(
            device,
            Some("Lighting Uniform Buffer"),
            &lighting_uniform,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        // Load shader
        let shader_src = include_str!("../../src/shaders/forward.wgsl");
        let shader_module = create_shader_module(device, Some("Mesh Shader"), shader_src);

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
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
            },
        );

        // Create bind group
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Uniform Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: lighting_uniform_buffer.as_entire_binding(),
                    },
                ],
            },
        );

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Mesh Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            },
        );

        // Create render pipeline
        let render_pipeline = RenderPipelineBuilder::new(device)
            .with_label(Some("Mesh Render Pipeline"))
            .with_layout(Some(&pipeline_layout))
            .with_shader_module(&shader_module)
            .with_vertex_entry("vs_main")
            .with_fragment_entry("fs_main")
            .with_vertex_buffers(&[Some(PosColorNormalVertex::desc())])
            .with_color_formats(&[context.surface_format.add_srgb_suffix()])
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            })
            .build();

        MeshViewer {
            vertex_buffer,
            index_buffer,
            num_indices,
            camera_uniform_buffer,
            lighting_uniform_buffer,
            render_pipeline,
            bind_group,
            surface_format: context.surface_format,
            camera,
            lights,
            num_lights,
            start_time: Instant::now(),
            model_scale: mesh.scale,
            mesh_center: mesh.center,
            camera_speed: 0.1,
        }
    }

    fn render(&mut self, context: &mut GraphicsContext) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let aspect = context.size.width as f32 / context.size.height as f32;

        // Create model matrix with rotation
        let translation = Matrix4::from_translation(-self.mesh_center);
        let scale_matrix = Matrix4::from_scale(self.model_scale);
        let model = Matrix4::from_angle_y(Rad(elapsed * 0.5))
            * scale_matrix
            * translation;

        // Update camera uniform
        let camera_uniform = CameraUniform::from_camera(&self.camera, aspect);
        context.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        // Update lighting uniform
        let lighting_uniform = LightingUniform::new_with_lights(
            &self.camera,
            &self.lights[..self.num_lights as usize]
        );
        context.queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lighting_uniform]),
        );

        // Get surface texture
        let surface_texture = match context.get_current_texture() {
            Some(texture) => texture,
            None => return,
        };
        let texture_view = context.create_texture_view(&surface_texture);

        // Create encoder and render pass
        let mut encoder = context.device.create_command_encoder(&Default::default());
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // Draw
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        // Submit and present
        context.queue.submit([encoder.finish()]);
        context.pre_present_notify();
        context.queue.present(surface_texture);
        context.request_redraw();
    }

    fn resize(&mut self, _context: &mut GraphicsContext, _new_size: winit::dpi::PhysicalSize<u32>) {
        // No size-dependent resources in this example
    }

    fn input(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput { event: key_event, .. } = event {
            if key_event.state.is_pressed() {
                let move_speed = self.camera_speed;
                let forward = self.camera.get_forward();
                let right = self.camera.get_right();
                
                if let Key::Character(c) = &key_event.logical_key {
                    match c.to_ascii_lowercase().as_str() {
                        "w" => self.camera.translate(forward * move_speed),
                        "s" => self.camera.translate(-forward * move_speed),
                        "a" => self.camera.translate(-right * move_speed),
                        "d" => self.camera.translate(right * move_speed),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn main() {
    env_logger::init();
    
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::<MeshViewer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

This complete example demonstrates:
- Mesh loading with fallback to built-in primitives
- Camera with WASD controls
- Multiple lights with diffuse lighting
- Smooth rotation animation
- Proper resource management

Happy coding!
