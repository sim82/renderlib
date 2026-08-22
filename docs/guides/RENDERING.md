# Rendering Pipelines Guide

This guide provides a comprehensive overview of rendering techniques in renderlib, focusing on forward and deferred rendering pipelines.

## Table of Contents

1. [Rendering Fundamentals](#1-rendering-fundamentals)
2. [Forward Rendering](#2-forward-rendering)
3. [Deferred Rendering](#3-deferred-rendering)
4. [Comparing Forward and Deferred](#4-comparing-forward-and-deferred)
5. [Implementing Custom Pipelines](#5-implementing-custom-pipelines)
6. [Advanced Rendering Techniques](#6-advanced-rendering-techniques)
7. [Performance Considerations](#7-performance-considerations)

---

## 1. Rendering Fundamentals

### The Graphics Pipeline

Modern GPUs implement a configurable graphics pipeline that processes vertices and generates pixels:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Graphics Pipeline                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │  Vertex   │───▶│  Tess-   │───▶│ Geometry │───▶│ Raster-  │  │
│  │  Shader   │    │  ellation│    │  Shader  │    │  ization │  │
│  │           │    │  Shader   │    │           │    │           │  │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘  │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Fragment Shader                            ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Output Merger                              ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Concepts

#### Vertex Processing
- **Vertex Shader**: Transforms each vertex from model space to clip space
- **Input**: Vertex attributes (position, normal, texture coordinates, etc.)
- **Output**: Clip space position and other per-vertex data

#### Rasterization
- Converts primitives (triangles, lines, points) into fragments
- **Fragment**: A potential pixel with interpolated vertex data

#### Fragment Processing
- **Fragment Shader**: Computes final color for each fragment
- **Input**: Interpolated vertex data
- **Output**: Final pixel color

#### Output Merging
- Blends fragment colors with framebuffer
- Handles depth testing, stencil testing, and color blending

### Render Passes

A **render pass** is a sequence of draw commands that write to the same set of attachments (color, depth, stencil buffers).

```rust
let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &texture_view,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
        ..Default::default()
    })],
    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
        view: &depth_view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        ..Default::default()
    }),
    ..Default::default()
});
```

### Load Operations

| LoadOp | Description |
|--------|-------------|
| `Clear(color)` | Fill attachment with specified color |
| `Load` | Keep existing content |
| `DontCare` | Undefined initial content (most efficient) |

### Store Operations

| StoreOp | Description |
|---------|-------------|
| `Store` | Write results to attachment |
| `DontCare` | Results may be discarded |

---

## 2. Forward Rendering

### Overview

Forward rendering (also called direct rendering) is the simplest and most straightforward rendering approach. Each object is processed completely - from vertex shader through fragment shader - in a single pass.

```
┌─────────────────────────────────────────────────────────────────┐
│                      Forward Rendering Pipeline                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  For each object:                                                │
│    ┌─────────────────────────────────────────────────────────┐  │
│    │  1. Set Pipeline                                         │  │
│    │  2. Set Uniforms (MVP matrix, material, etc.)             │  │
│    │  3. Set Vertex Buffers                                    │  │
│    │  4. Set Index Buffer (if indexed)                         │  │
│    │  5. Draw Call (draw or draw_indexed)                      │  │
│    └─────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Result: Final color written to framebuffer                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Forward Rendering in renderlib

The `forward.rs` demo implements forward rendering with:

1. **Mesh Loading**: GLTF or built-in cube
2. **Camera**: Perspective projection with orbit controls
3. **Lighting**: Multiple lights with diffuse and specular components
4. **Depth Testing**: Proper depth buffer for occlusion

### Code Structure

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
    
    let lighting_uniform = LightingUniform::new_with_lights(
        &self.camera, 
        &self.lights[..self.num_lights as usize]
    );
    context.queue.write_buffer(&self.lighting_uniform_buffer, 0,
        bytemuck::cast_slice(&[lighting_uniform]));
    
    // 3. Get surface texture
    let surface_texture = match context.get_current_texture() {
        Some(texture) => texture,
        None => return,
    };
    let texture_view = context.create_texture_view(&surface_texture);
    
    // 4. Create command encoder
    let mut encoder = context.device.create_command_encoder(&Default::default());
    
    // 5. Begin render pass with depth testing
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
    
    // 6. Draw all objects
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    
    // 7. Submit and present
    context.queue.submit([encoder.finish()]);
    context.pre_present_notify();
    context.queue.present(surface_texture);
    context.request_redraw();
}
```

### Forward Rendering Shader

The forward rendering shader (`forward.wgsl`) performs:

1. **Vertex Shader**: Transforms vertices and passes data to fragment shader
2. **Fragment Shader**: Computes lighting for each pixel

```wgsl
// Vertex shader
@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
) -> VertexOutput {
    // Transform position to world space
    let world_pos = vec3<f32>((geometry.model * vec4<f32>(position, 1.0)).xyz);
    
    // Transform to clip space
    let clip_pos = geometry.view_projection * vec4<f32>(world_pos, 1.0);
    
    // Transform normal to world space
    let world_normal = mat3<f32>(geometry.model) * normal;
    
    return VertexOutput(clip_pos, color, world_pos, world_normal);
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let mut color = in.color;
    let normal = normalize(in.world_normal);
    let view_dir = normalize(lighting.view_position.xyz - in.world_position);
    
    // Diffuse lighting
    let mut diffuse = vec3<f32>(0.0);
    for (var i: u32 = 0; i < lighting.num_lights; i = i + 1) {
        let light_pos = lighting.lights[i].position.xyz;
        let light_color = lighting.lights[i].color.xyz;
        let light_dir = normalize(light_pos - in.world_position);
        let diffuse_factor = max(dot(normal, light_dir), 0.0);
        diffuse = diffuse + light_color * diffuse_factor;
    }
    
    // Combine with base color
    color = color * (diffuse + 0.1); // Add ambient
    
    return vec4<f32>(color, 1.0);
}
```

### Depth Testing

Forward rendering uses depth testing to handle occlusion:

```rust
let depth_stencil = wgpu::DepthStencilState {
    format: wgpu::TextureFormat::Depth32Float,
    depth_write_enabled: Some(true),  // Enable depth writes
    depth_compare: Some(wgpu::CompareFunction::Less),  // Nearer objects obscure farther ones
    stencil: wgpu::StencilState::default(),
    bias: wgpu::DepthBiasState::default(),
};
```

### Advantages of Forward Rendering

1. **Simplicity**: Easy to understand and implement
2. **Single Pass**: Each object rendered in one pass
3. **Memory Efficient**: No additional render targets needed
4. **Good for Simple Scenes**: Works well with few objects and lights

### Limitations of Forward Rendering

1. **Light Complexity**: Performance degrades with many lights (O(n*l) where n=objects, l=lights)
2. **No Advanced Effects**: Hard to implement effects like screen-space reflections
3. **Overdraw**: Objects are shaded even if they're obscured

---

## 3. Deferred Rendering

### Overview

Deferred rendering is a multi-pass technique that separates geometry processing from lighting calculation. It's particularly efficient for scenes with many lights.

```
┌─────────────────────────────────────────────────────────────────┐
│                     Deferred Rendering Pipeline                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Phase 1: Geometry Pass                                          │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  For each object:                                            │  │
│  │    - Render to G-buffer (position, normal, albedo)           │  │
│  │    - No lighting calculation                                 │  │
│  │    - Depth testing enabled                                   │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Phase 2: Lighting Pass                                           │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │  - Render full-screen quad                                   │  │
│  │  - Sample G-buffer for position, normal, albedo              │  │
│  │  - Calculate lighting for each pixel                         │  │
│  │  - No depth testing (already handled in geometry pass)        │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Result: Final color written to framebuffer                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Deferred Rendering in renderlib

The `deferred.rs` demo implements deferred rendering with:

1. **G-Buffer**: Three render targets (position, normal, albedo)
2. **Geometry Pass**: Renders all objects to G-buffer
3. **Lighting Pass**: Full-screen quad that reads G-buffer and computes lighting

### G-Buffer Structure

```rust
pub struct GBuffer {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub position_texture: wgpu::Texture,    // World space position
    pub normal_texture: wgpu::Texture,      // World space normal
    pub albedo_texture: wgpu::Texture,      // Surface color
    pub position_view: wgpu::TextureView,
    pub normal_view: wgpu::TextureView,
    pub albedo_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}
```

**Texture Format**: All G-buffer textures use `Rgba16Float` for 16-bit floating point precision.

### Geometry Pass

```rust
// In DeferredRenderer::render()
{
    let mut geometry_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Geometry Pass"),
        color_attachments: &self.gbuffer.color_attachments(),
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });

    // Draw mesh
    geometry_pass.set_pipeline(&self.geometry_pipeline);
    geometry_pass.set_bind_group(0, &self.geometry_bind_group, &[]);
    geometry_pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
    geometry_pass.set_index_buffer(self.mesh_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    geometry_pass.draw_indexed(0..self.num_indices, 0, 0..1);
}
```

### Lighting Pass

```rust
// In DeferredRenderer::render()
{
    let mut lighting_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Lighting Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &surface_view,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            ..Default::default()
        })],
        depth_stencil_attachment: None,
        ..Default::default()
    });

    // Draw full-screen quad
    lighting_pass.set_pipeline(&self.lighting_pipeline);
    lighting_pass.set_bind_group(0, &gbuffer_bind_group, &[]);
    lighting_pass.set_bind_group(1, &self.lighting_uniform_bind_group, &[]);
    lighting_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
    lighting_pass.draw(0..6, 0..1);
}
```

### Geometry Pass Shader

The geometry pass shader (`deferred_geometry.wgsl`) writes to multiple render targets:

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    // Write position to first render target
    var output: FragmentOutput;
    output.position = vec4<f32>(in.world_position, 1.0);
    
    // Write normal to second render target
    output.normal = vec4<f32>(normalize(in.world_normal), 0.0);
    
    // Write albedo to third render target
    output.albedo = vec4<f32>(in.color, 1.0);
    
    return output;
}
```

### Lighting Pass Shader

The lighting pass shader (`deferred_lighting.wgsl`) reads from G-buffer and computes lighting:

```wgsl
@group(0) @binding(0)
var position_texture: texture_2d<f32>;
@group(0) @binding(1)
var normal_texture: texture_2d<f32>;
@group(0) @binding(2)
var albedo_texture: texture_2d<f32>;
@group(0) @binding(3)
var sampler: sampler;

@group(1) @binding(0)
var<uniform> lighting: LightingUniform;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Sample G-buffer
    let position = textureSample(position_texture, sampler, uv).xyz;
    let normal = textureSample(normal_texture, sampler, uv).xyz;
    let albedo = textureSample(albedo_texture, sampler, uv).rgb;
    
    // Calculate lighting
    let n = normalize(normal);
    let view_dir = normalize(lighting.view_position.xyz - position);
    
    let mut color = vec3<f32>(0.0);
    for (var i: u32 = 0; i < lighting.num_lights; i = i + 1) {
        let light_pos = lighting.lights[i].position.xyz;
        let light_color = lighting.lights[i].color.xyz;
        
        let light_dir = normalize(light_pos - position);
        let diffuse = max(dot(n, light_dir), 0.0);
        
        // Simple diffuse lighting
        color = color + albedo * light_color * diffuse;
    }
    
    // Add ambient light
    color = color + albedo * 0.1;
    
    return vec4<f32>(color, 1.0);
}
```

### Bind Group Organization

Deferred rendering uses multiple bind groups:

```
Geometry Pass:
  Bind Group 0:
  └── Binding 0: Geometry Uniform Buffer (MVP, model)
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

### Advantages of Deferred Rendering

1. **Light Efficiency**: Performance independent of number of lights (O(n + s) where n=objects, s=screen pixels)
2. **Complex Lighting**: Easy to implement many lights with complex lighting models
3. **Screen-Space Effects**: Natural fit for screen-space effects (SSR, SSAO, etc.)
4. **Anti-Aliasing**: Can be combined with MSAA more effectively

### Limitations of Deferred Rendering

1. **Memory Usage**: Requires additional render targets (G-buffer)
2. **No Transparency**: Doesn't handle transparent objects well (requires forward rendering pass)
3. **No MSAA**: Multi-sample anti-aliasing doesn't work well with deferred (requires post-process AA)
4. **Material Limitations**: Limited by G-buffer channels (typically 3-4 RTs)

---

## 4. Comparing Forward and Deferred

### Performance Comparison

| Metric | Forward | Deferred |
|--------|---------|----------|
| Few Objects, Few Lights | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Many Objects, Few Lights | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| Few Objects, Many Lights | ⭐ | ⭐⭐⭐⭐⭐ |
| Many Objects, Many Lights | ⭐ | ⭐⭐⭐⭐⭐ |
| Memory Usage | ⭐⭐⭐⭐⭐ | ⭐⭐ |
| Implementation Complexity | ⭐⭐⭐⭐⭐ | ⭐⭐ |
| Transparency Support | ⭐⭐⭐⭐⭐ | ⭐ |
| Screen-Space Effects | ⭐⭐ | ⭐⭐⭐⭐⭐ |

### When to Use Each

**Use Forward Rendering When:**
- You have a simple scene with few objects
- You need transparency support
- You want minimal memory usage
- You're targeting mobile devices with limited memory
- You want the simplest implementation

**Use Deferred Rendering When:**
- You have many dynamic lights
- You want to implement advanced screen-space effects
- You need consistent performance regardless of light count
- You're rendering complex scenes with many objects
- You want to implement post-processing effects

### Hybrid Approach

Many modern engines use a hybrid approach:

1. **Opaque Objects**: Rendered with deferred rendering
2. **Transparent Objects**: Rendered with forward rendering (after deferred lighting pass)
3. **Post-Processing**: Applied after all rendering

---

## 5. Implementing Custom Pipelines

### Creating a New Renderer

To create a custom renderer, implement the `AppRenderer` trait:

```rust
struct MyRenderer {
    // Your rendering resources
}

impl AppRenderer for MyRenderer {
    async fn init(context: &GraphicsContext) -> Self {
        // Initialize resources
        Self { /* ... */ }
    }
    
    fn render(&mut self, context: &mut GraphicsContext) {
        // Render a frame
    }
    
    fn resize(&mut self, context: &mut GraphicsContext, new_size: PhysicalSize<u32>) {
        // Handle resize
    }
    
    fn input(&mut self, event: &WindowEvent) {
        // Handle input (optional)
    }
}
```

### Using RenderPipelineBuilder

The `RenderPipelineBuilder` provides a fluent API for creating pipelines:

```rust
let pipeline = RenderPipelineBuilder::new(&device)
    .with_label(Some("My Pipeline"))
    .with_layout(Some(&pipeline_layout))
    .with_shader_module(&shader_module)
    .with_vertex_entry("vs_main")
    .with_fragment_entry("fs_main")
    .with_vertex_buffers(&[Some(PosColorNormalVertex::desc())])
    .with_color_formats(&[surface_format.add_srgb_suffix()])
    .with_depth_stencil(Some(depth_stencil_state))
    .with_primitive(wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        ..Default::default()
    })
    .build();
```

### Multiple Render Passes

For complex pipelines with multiple passes:

```rust
fn render(&mut self, context: &mut GraphicsContext) {
    let surface_texture = match context.get_current_texture() {
        Some(texture) => texture,
        None => return,
    };
    let surface_view = context.create_texture_view(&surface_texture);
    
    let mut encoder = context.device.create_command_encoder(&Default::default());
    
    // Pass 1: Geometry
    {
        let mut pass = encoder.begin_render_pass(&pass1_desc);
        // Draw geometry
    }
    
    // Pass 2: Lighting
    {
        let mut pass = encoder.begin_render_pass(&pass2_desc);
        // Draw lighting
    }
    
    // Pass 3: Post-processing
    {
        let mut pass = encoder.begin_render_pass(&pass3_desc);
        // Apply post-processing
    }
    
    context.queue.submit([encoder.finish()]);
    context.queue.present(surface_texture);
}
```

### Sharing Resources Between Passes

Textures created in one pass can be used as inputs in subsequent passes:

```rust
// Create a texture for intermediate results
let intermediate_texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("Intermediate Texture"),
    size: wgpu::Extent3d {
        width: context.size.width,
        height: context.size.height,
        depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba16Float,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    view_formats: &[],
});

// Create a view for rendering to this texture
let intermediate_view = intermediate_texture.create_view(&Default::default());

// Create a sampler for reading from this texture
let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
    address_mode_u: wgpu::AddressMode::ClampToEdge,
    address_mode_v: wgpu::AddressMode::ClampToEdge,
    mag_filter: wgpu::FilterMode::Linear,
    min_filter: wgpu::FilterMode::Linear,
    ..Default::default()
});

// Pass 1: Render to intermediate texture
let mut pass1 = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &intermediate_view,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
        ..Default::default()
    })],
    ..Default::default()
});
// Draw to intermediate texture

// Pass 2: Read from intermediate texture
let mut pass2 = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &surface_view,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
        ..Default::default()
    })],
    ..Default::default()
});

// Set bind group with intermediate texture
pass2.set_bind_group(0, &intermediate_bind_group, &[]);
// Draw full-screen quad
```

---

## 6. Advanced Rendering Techniques

### Shadow Mapping

Shadow mapping uses a depth texture to determine visibility from a light's perspective:

1. **Shadow Pass**: Render scene from light's perspective to depth texture
2. **Main Pass**: Render scene normally, using shadow map to determine shadowed areas

```rust
// Create shadow map texture
let shadow_map_size = 1024;
let (shadow_map_texture, shadow_map_view) = create_depth_texture(
    &device,
    shadow_map_size,
    shadow_map_size,
    Some("Shadow Map"),
);

// Shadow pass
let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &[],
    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
        view: &shadow_map_view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        ..Default::default()
    }),
    ..Default::default()
});

// Use light's view-projection matrix
let light_vp = light_camera.get_view_projection_matrix(aspect);
let light_uniform = CameraUniform::from_camera(&light_camera, aspect);

// Draw all objects with light's MVP matrix
```

### Screen-Space Ambient Occlusion (SSAO)

SSAO approximates ambient occlusion by sampling the depth buffer:

1. **Depth Pass**: Render scene to depth texture
2. **SSAO Pass**: Sample depth buffer to compute occlusion factor
3. **Lighting Pass**: Apply SSAO to lighting calculation

### Bloom

Bloom creates a glowing effect for bright areas:

1. **Bright Pass**: Extract bright pixels to a texture
2. **Blur Pass**: Apply Gaussian blur to bright texture
3. **Combine Pass**: Add blurred bright texture to original image

### Tone Mapping and Gamma Correction

Apply tone mapping to HDR colors before display:

```wgsl
// In fragment shader
let mapped = pow(color.rgb, vec3<f32>(1.0 / 2.2)); // Gamma correction
let final_color = vec4<f32>(mapped, color.a);
```

---

## 7. Performance Considerations

### Optimization Techniques

#### 1. Minimize Draw Calls

- **Instanced Rendering**: Render many similar objects with one draw call
- **Batching**: Combine multiple objects into one mesh
- **Culling**: Skip rendering objects that aren't visible

```rust
// Instanced rendering
let instance_count = 1000;
render_pass.draw_indexed(0..index_count, 0, 0..instance_count);
```

#### 2. Efficient Buffer Usage

- **Uniform Buffers**: Use for frequently changed data
- **Storage Buffers**: Use for large, read-only data
- **Dynamic Offsets**: Update portions of buffers instead of recreating

```rust
// Update only a portion of a buffer
queue.write_buffer(&buffer, offset, bytemuck::cast_slice(&[new_data]));
```

#### 3. Pipeline State Management

- **Pipeline Cache**: Reuse pipeline layouts and bind group layouts
- **Minimize State Changes**: Sort draw calls by pipeline and state
- **Specialization Constants**: Use for shader variants

#### 4. Texture Optimization

- **Compression**: Use compressed texture formats (BCn, ASTC, ETC2)
- **Mipmapping**: Generate mipmaps for textures viewed from a distance
- **Anisotropic Filtering**: Improve texture quality at oblique angles

```rust
let texture = device.create_texture(&wgpu::TextureDescriptor {
    format: wgpu::TextureFormat::Bc7RgbaUnormSrgb, // Compressed format
    mip_level_count: 10, // Mipmaps
    ..Default::default()
});
```

#### 5. Depth Testing Optimization

- **Early Depth Testing**: Enable to avoid fragment shader execution for obscured fragments
- **Depth Clamping**: Clamp depth values instead of discarding

```rust
let depth_stencil = wgpu::DepthStencilState {
    depth_write_enabled: Some(true),
    depth_compare: Some(wgpu::CompareFunction::Less),
    depth_bias: wgpu::DepthBiasState {
        constant: 0,
        slope_scale: 0.0,
        clamp: 0.0,
    },
    stencil: wgpu::StencilState::default(),
};
```

### Profiling Tools

- **wgpu Profiler**: Built-in profiling for wgpu applications
- **Renderdoc**: Graphics debugger for frame capture and analysis
- **PIX**: Microsoft's graphics debugger (Windows)
- **Xcode Graphics Tools**: For Metal on macOS
- **NSight**: For Vulkan and OpenGL on Linux

### Common Performance Pitfalls

1. **Too Many Draw Calls**: Each draw call has overhead
2. **Frequent Buffer Updates**: Recreating buffers every frame
3. **Unnecessary Computation**: Performing calculations in shaders that could be precomputed
4. **Memory Bandwidth**: Large textures or many render targets
5. **Synchronization**: Waiting for GPU to finish between frames

---

## Summary

This guide has covered:

1. **Rendering Fundamentals**: The graphics pipeline and render passes
2. **Forward Rendering**: Simple, single-pass rendering with lighting
3. **Deferred Rendering**: Multi-pass rendering for efficient lighting
4. **Comparison**: When to use each rendering approach
5. **Custom Pipelines**: How to implement your own rendering techniques
6. **Advanced Techniques**: Shadow mapping, SSAO, bloom, etc.
7. **Performance**: Optimization techniques and considerations

For more information, see:

- [Architecture Overview](../architecture/01-OVERVIEW.md): High-level system design
- [Module Documentation](../architecture/02-MODULES.md): Detailed module reference
- [Component Interactions](../architecture/03-COMPONENT_INTERACTIONS.md): How components work together
- [Getting Started Guide](GETTING_STARTED.md): Create your first application

Happy rendering!
