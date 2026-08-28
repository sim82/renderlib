# Renderlib Module Documentation

**Version:** 0.2.0  
**Architecture:** Radical Separation (Phases 1-4 Complete)  
**Last Updated:** 2026-08-29

## Table of Contents

1. [app Module](#1-app-module)
2. [camera Module](#2-camera-module)
3. [context Module](#3-context-module)
4. [deferred Module](#4-deferred-module)
5. [device Module](#5-device-module)
6. [device_helpers Module](#6-device_helpers-module)
7. [geometry Module](#7-geometry-module)
8. [input Module](#8-input-module)
9. [mesh Module](#9-mesh-module)
10. [player Module](#10-player-module)
11. [state Module](#11-state-module)
12. [lib.rs](#12-librs)

---

## 1. app Module

### Overview

The `app` module provides the **application framework** for renderlib. It implements the new Radical Separation architecture with clean separation between GPU infrastructure and application state.

**Key Concept:** The `Application<R>` struct manages the application lifecycle, while the `AppRenderer` trait defines the interface that renderers must implement.

### Key Types

#### `AppRenderer` Trait

The `AppRenderer` trait defines the interface that all renderers must implement. It provides a clean separation between application management and rendering logic.

```rust
pub trait AppRenderer: Sized {
    /// Initialize rendering resources asynchronously.
    /// Receives a RenderContext which provides access to both
    /// GPU infrastructure (via device) and application state (via state).
    fn init(context: RenderContext<'_>) -> impl std::future::Future<Output = Self>;

    /// Called when the window needs to be redrawn.
    /// Receives a RenderContext for accessing resources.
    fn render(&mut self, context: RenderContext<'_>);

    /// Called on window resize (after the surface has been reconfigured).
    /// Receives the new size and a RenderContext for accessing resources.
    fn resize(&mut self, context: RenderContext<'_>, new_size: winit::dpi::PhysicalSize<u32>);

    /// Called when an input event occurs (e.g., key press).
    /// Default implementation does nothing.
    fn input(&mut self, _context: RenderContext<'_>, _event: &WindowEvent) {}
}
```

**Example Implementation:**

```rust
use renderlib::app::AppRenderer;
use renderlib::context::RenderContext;

struct MyRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
}

impl AppRenderer for MyRenderer {
    async fn init(mut context: RenderContext<'_>) -> Self {
        let device = context.wgpu_device();
        
        // Create render pipeline
        let render_pipeline = create_pipeline(device);
        
        // Create vertex buffer
        let vertex_buffer = renderlib::device_helpers::create_buffer_from_slice(
            device,
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );
        
        Self { render_pipeline, vertex_buffer }
    }

    fn render(&mut self, mut context: RenderContext<'_>) {
        let device = context.wgpu_device();
        let queue = context.wgpu_queue();
        let texture_view = context.get_texture_view().expect("No texture view");
        
        // Render logic here
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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
            });
            
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..3, 0..1);
        }
        
        queue.submit(std::iter::once(encoder.finish()));
    }

    fn resize(&mut self, _context: RenderContext<'_>, _new_size: winit::dpi::PhysicalSize<u32>) {
        // Recreate size-dependent resources
    }
}
```

#### `Application<R>` Struct

The `Application` struct is the main application type that implements `winit::ApplicationHandler`. It manages the GPU infrastructure, application state, and renderer.

```rust
pub struct Application<R: AppRenderer> {
    /// GPU infrastructure (immutable)
    device: Option<GraphicsDevice>,
    /// Application state (mutable)
    state: Option<AppState>,
    /// Renderer instance
    renderer: Option<R>,
    /// Window reference
    window: Option<Arc<Window>>,
}
```

**Key Methods:**

- `new()`: Create a new application instance
- `create_render_context(surface_texture)`: Create a RenderContext for the current frame
- Implements `ApplicationHandler` methods: `resumed()`, `window_event()`, etc.

**Example Usage:**

```rust
use renderlib::app::{AppRenderer, Application};

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = Application::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

### Dependencies

- `winit`: For `ApplicationHandler`, `Window`, `WindowEvent`
- `context`: For `RenderContext`
- `device`: For `GraphicsDevice`
- `state`: For `AppState`

---

## 2. camera Module

### Overview

The `camera` module provides comprehensive camera and lighting support for 3D rendering. It includes types for camera configuration, view/projection matrices, transforms, and lighting uniforms.

### Constants

```rust
/// Maximum number of lights supported by the rendering system.
pub const MAX_LIGHTS: usize = 32;
```

### Default Configuration (`defaults` module)

The `defaults` module provides sensible default values for camera configuration:

```rust
pub mod defaults {
    /// Default field of view (in radians)
    pub const FOV: Rad<f32> = Rad(1.0472); // ~60 degrees
    
    /// Default near clipping plane
    pub const NEAR: f32 = 0.1;
    
    /// Default far clipping plane
    pub const FAR: f32 = 1000.0;
    
    /// Default camera position
    pub fn position() -> Point3<f32> {
        Point3::new(0.0, 0.0, 5.0)
    }
    
    /// Default camera target (look at point)
    pub fn target() -> Point3<f32> {
        Point3::new(0.0, 0.0, 0.0)
    }
    
    /// Default camera up vector
    pub fn up() -> Vector3<f32> {
        Vector3::new(0.0, 1.0, 0.0)
    }
}
```

### Key Types

#### `Camera` Struct

The `Camera` struct represents a 3D camera with position, orientation, and projection settings.

```rust
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: Point3<f32>,
    /// Point the camera is looking at
    pub target: Point3<f32>,
    /// Up vector for the camera
    pub up: Vector3<f32>,
    /// Field of view (in radians)
    pub fov: Rad<f32>,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
}
```

**Constructors:**

```rust
impl Camera {
    /// Create a new camera with default values
    pub fn new() -> Self {
        Self {
            position: defaults::position(),
            target: defaults::target(),
            up: defaults::up(),
            fov: defaults::FOV,
            near: defaults::NEAR,
            far: defaults::FAR,
        }
    }

    /// Create a camera looking at a specific target
    pub fn look_at(position: Point3<f32>, target: Point3<f32>, up: Vector3<f32>) -> Self {
        Self {
            position,
            target,
            up,
            fov: defaults::FOV,
            near: defaults::NEAR,
            far: defaults::FAR,
        }
    }

    /// Create a camera with custom parameters
    pub fn with_params(
        position: Point3<f32>,
        target: Point3<f32>,
        up: Vector3<f32>,
        fov: Rad<f32>,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position,
            target,
            up,
            fov,
            near,
            far,
        }
    }
}
```

**Matrix Methods:**

```rust
impl Camera {
    /// Get the view matrix for this camera
    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(
            self.position,
            (self.target - self.position).normalize(),
            self.up,
        )
    }

    /// Get the projection matrix for this camera
    pub fn get_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32> {
        cgmath::perspective(self.fov, aspect_ratio, self.near, self.far)
    }

    /// Get the combined view-projection matrix
    pub fn get_view_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32> {
        let view = self.get_view_matrix();
        let proj = self.get_projection_matrix(aspect_ratio);
        proj * view
    }
}
```

**Position/Orientation Methods:**

```rust
impl Camera {
    /// Get the current camera position
    pub fn get_position(&self) -> Point3<f32> { self.position }

    /// Get the current camera target
    pub fn get_target(&self) -> Point3<f32> { self.target }

    /// Get the forward vector (direction camera is facing)
    pub fn get_forward(&self) -> Vector3<f32> {
        (self.target - self.position).normalize()
    }

    /// Get the right vector (perpendicular to forward and up)
    pub fn get_right(&self) -> Vector3<f32> {
        let forward = self.get_forward();
        forward.cross(self.up).normalize()
    }

    /// Set the camera position
    pub fn set_position(&mut self, position: Point3<f32>) { self.position = position; }

    /// Set the camera target
    pub fn set_target(&mut self, target: Point3<f32>) { self.target = target; }

    /// Set the camera up vector
    pub fn set_up(&mut self, up: Vector3<f32>) { self.up = up; }

    /// Set the field of view
    pub fn set_fov(&mut self, fov: Rad<f32>) { self.fov = fov; }

    /// Set the near clipping plane
    pub fn set_near(&mut self, near: f32) { self.near = near; }

    /// Set the far clipping plane
    pub fn set_far(&mut self, far: f32) { self.far = far; }

    /// Translate the camera by the given vector
    pub fn translate(&mut self, translation: Vector3<f32>) {
        self.position += translation;
        self.target += translation;
    }

    /// Orbit the camera around the target
    pub fn orbit(&mut self, horizontal: f32, vertical: f32) {
        // Implementation rotates camera around target
    }
}
```

#### `CameraUniform` Struct

The `CameraUniform` struct contains the camera matrices in a format suitable for shader uniforms.

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// View matrix
    pub view: [[f32; 4]; 4],
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// Combined view-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Camera position in world space
    pub view_position: [f32; 4],
}
```

**Methods:**

```rust
impl CameraUniform {
    /// Create a CameraUniform from a Camera and aspect ratio
    pub fn from_camera(camera: &Camera, aspect_ratio: f32) -> Self {
        Self {
            view: camera.get_view_matrix().into(),
            projection: camera.get_projection_matrix(aspect_ratio).into(),
            view_projection: camera.get_view_projection_matrix(aspect_ratio).into(),
            view_position: [camera.position.x, camera.position.y, camera.position.z, 1.0],
        }
    }

    /// Create an identity CameraUniform
    pub fn identity() -> Self {
        Self {
            view: Matrix4::identity().into(),
            projection: Matrix4::identity().into(),
            view_projection: Matrix4::identity().into(),
            view_position: [0.0, 0.0, 0.0, 1.0],
        }
    }
}
```

#### `Transform` Struct

The `Transform` struct represents a 3D transformation (translation, rotation, scale).

```rust
#[derive(Debug, Clone)]
pub struct Transform {
    /// Translation vector
    pub translation: Vector3<f32>,
    /// Rotation as Euler angles (in radians)
    pub rotation: Vector3<f32>,
    /// Scale vector
    pub scale: Vector3<f32>,
}
```

**Constructors:**

```rust
impl Transform {
    /// Create a new transform with default values (identity)
    pub fn new() -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    /// Create a transform with only translation
    pub fn with_translation(translation: Vector3<f32>) -> Self {
        Self {
            translation,
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    /// Create a transform with only rotation
    pub fn with_rotation(rotation: Vector3<f32>) -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation,
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    /// Create a transform with only scale
    pub fn with_scale(scale: Vector3<f32>) -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale,
        }
    }

    /// Create a transform with all components
    pub fn with_all(translation: Vector3<f32>, rotation: Vector3<f32>, scale: Vector3<f32>) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }
}
```

**Matrix Methods:**

```rust
impl Transform {
    /// Get the model matrix for this transform
    pub fn get_model_matrix(&self) -> Matrix4<f32> {
        let translation_matrix = Matrix4::from_translation(self.translation);
        let rotation_matrix = Matrix4::from_angle_x(self.rotation.x)
            * Matrix4::from_angle_y(self.rotation.y)
            * Matrix4::from_angle_z(self.rotation.z);
        let scale_matrix = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        
        translation_matrix * rotation_matrix * scale_matrix
    }

    /// Create a transform with time-based rotation (for animation)
    pub fn with_time_based_rotation(time: f32) -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, time, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    /// Set the translation component
    pub fn set_translation(&mut self, translation: Vector3<f32>) {
        self.translation = translation;
    }

    /// Set the rotation component
    pub fn set_rotation(&mut self, rotation: Vector3<f32>) {
        self.rotation = rotation;
    }

    /// Set the scale component
    pub fn set_scale(&mut self, scale: Vector3<f32>) {
        self.scale = scale;
    }
}
```

#### `GeometryUniform` Struct

The `GeometryUniform` struct contains the model and MVP matrices for vertex shading.

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryUniform {
    /// Model-view-projection matrix
    pub mvp: [[f32; 4]; 4],
    /// Model matrix
    pub model: [[f32; 4]; 4],
}

impl GeometryUniform {
    /// Create a new GeometryUniform
    pub fn new() -> Self {
        Self {
            mvp: Matrix4::identity().into(),
            model: Matrix4::identity().into(),
        }
    }
}
```

#### `Light` Struct

The `Light` struct represents a light source in the scene.

```rust
#[derive(Debug, Clone, Copy)]
pub struct Light {
    /// Light position in world space
    pub position: [f32; 3],
    /// Light color (RGB)
    pub color: [f32; 3],
}

impl Light {
    /// Create a new light with the given position and color
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        Self { position, color }
    }

    /// Create a new light with the given position and intensity
    pub fn with_intensity(position: [f32; 3], intensity: f32) -> Self {
        Self {
            position,
            color: [intensity, intensity, intensity],
        }
    }
}
```

#### `LightingUniform` Struct

The `LightingUniform` struct contains lighting information for fragment shading.

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniform {
    /// Camera/view position in world space
    pub view_position: [f32; 4],
    /// Number of active lights
    pub num_lights: u32,
    /// Padding to align to 16 bytes
    pub _padding: [u32; 3],
    /// Array of lights (up to MAX_LIGHTS)
    pub lights: [Light; MAX_LIGHTS],
}

impl LightingUniform {
    /// Create a new LightingUniform
    pub fn new() -> Self {
        Self {
            view_position: [0.0, 0.0, 0.0, 1.0],
            num_lights: 0,
            _padding: [0; 3],
            lights: [Light::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]); MAX_LIGHTS],
        }
    }

    /// Create a new LightingUniform with the given lights
    pub fn new_with_lights(view_position: [f32; 3], lights: &[Light]) -> Self {
        let mut uniform = Self::new();
        uniform.view_position = [view_position[0], view_position[1], view_position[2], 1.0];
        uniform.num_lights = lights.len() as u32;
        
        for (i, &light) in lights.iter().enumerate().take(MAX_LIGHTS) {
            uniform.lights[i] = light;
        }
        
        uniform
    }
}
```

---

## 3. context Module

### Overview

The `context` module provides the **RenderContext** struct, which is the **cornerstone of the Radical Separation architecture**. It provides clean, temporary access to both immutable GPU infrastructure and mutable application state.

### Key Type

#### `RenderContext<'a>` Struct

The `RenderContext` struct is passed to renderer methods and provides access to all necessary resources:

```rust
/// A context passed to renderers that provides access to both
/// immutable GPU infrastructure and mutable application state.
pub struct RenderContext<'a> {
    /// Immutable GPU infrastructure
    device: &'a GraphicsDevice,
    /// Mutable application state
    state: &'a mut AppState,
    /// Current texture view (optional, for rendering)
    texture_view: Option<wgpu::TextureView>,
}
```

**Key Methods:**

**Device Access:**

```rust
impl<'a> RenderContext<'a> {
    /// Get a reference to the GPU device.
    pub fn device(&self) -> &GraphicsDevice {
        self.device
    }

    /// Get a reference to the wgpu device.
    pub fn wgpu_device(&self) -> &wgpu::Device {
        &self.device.device
    }

    /// Get a reference to the wgpu queue.
    pub fn wgpu_queue(&self) -> &wgpu::Queue {
        &self.device.queue
    }
}
```

**State Access:**

```rust
impl<'a> RenderContext<'a> {
    /// Get a mutable reference to the application state.
    pub fn state(&mut self) -> &mut AppState {
        self.state
    }
}
```

**Texture Access:**

```rust
impl<'a> RenderContext<'a> {
    /// Take the current texture view, leaving None in its place.
    pub fn take_texture_view(&mut self) -> Option<wgpu::TextureView> {
        self.texture_view.take()
    }

    /// Get the current texture view.
    pub fn texture_view(&self) -> Option<&wgpu::TextureView> {
        self.texture_view.as_ref()
    }

    /// Get the texture view.
    pub fn get_texture_view(&self) -> Option<&wgpu::TextureView> {
        self.texture_view.as_ref()
    }
}
```

**Window Operations:**

```rust
impl<'a> RenderContext<'a> {
    /// Request a redraw of the window.
    pub fn request_redraw(&self) {
        self.device.request_redraw();
    }

    /// Notify the window before presenting.
    pub fn pre_present_notify(&self) {
        self.device.pre_present_notify();
    }

    /// Get the current window size.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.device.size()
    }

    /// Get the surface format.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.device.surface_format()
    }
}
```

**Constructor:**

```rust
impl<'a> RenderContext<'a> {
    /// Create a new render context.
    pub fn new(
        device: &'a GraphicsDevice,
        state: &'a mut AppState,
        texture_view: Option<wgpu::TextureView>,
    ) -> Self {
        Self {
            device,
            state,
            texture_view,
        }
    }
}
```

### Usage Example

```rust
use renderlib::context::RenderContext;

fn render(&mut self, mut context: RenderContext<'_>) {
    // Access GPU infrastructure
    let device = context.wgpu_device();
    let queue = context.wgpu_queue();
    
    // Access mutable state
    let state = context.state();
    let mesh_cache = &state.mesh_cache;
    let camera = &state.camera;
    
    // Get texture view for rendering
    let texture_view = context.get_texture_view()?;
    
    // Request redraw for next frame (e.g., for animation)
    context.request_redraw();
    
    // Get window size
    let size = context.size();
    
    // Get surface format
    let format = context.surface_format();
    
    // ... rendering logic
}
```

### Dependencies

- `device`: For `GraphicsDevice`
- `state`: For `AppState`

---

## 4. deferred Module

### Overview

The `deferred` module provides **G-buffer management** for deferred rendering. Deferred rendering is a technique that separates the geometry processing from the lighting calculation, allowing for more efficient handling of many light sources.

### Key Type

#### `GBuffer` Struct

The `GBuffer` struct manages the multiple render targets used in deferred rendering:

```rust
/// G-buffer for deferred rendering.
/// Contains position, normal, and albedo textures for geometry pass output.
pub struct GBuffer {
    /// Bind group layout for accessing the G-buffer in shaders
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Position texture (stores world space positions)
    pub position_texture: wgpu::Texture,
    /// Normal texture (stores world space normals)
    pub normal_texture: wgpu::Texture,
    /// Albedo texture (stores color/albedo)
    pub albedo_texture: wgpu::Texture,
    /// View of the position texture
    pub position_view: wgpu::TextureView,
    /// View of the normal texture
    pub normal_view: wgpu::TextureView,
    /// View of the albedo texture
    pub albedo_view: wgpu::TextureView,
    /// Sampler for texture access
    pub sampler: wgpu::Sampler,
    /// Width of the G-buffer textures
    pub width: u32,
    /// Height of the G-buffer textures
    pub height: u32,
}
```

**Constructor:**

```rust
impl GBuffer {
    /// Create a new G-buffer with the specified dimensions.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        // Create textures for position, normal, and albedo
        let position_texture = create_gbuffer_texture(device, width, height, "Position");
        let normal_texture = create_gbuffer_texture(device, width, height, "Normal");
        let albedo_texture = create_gbuffer_texture(device, width, height, "Albedo");
        
        // Create texture views
        let position_view = position_texture.create_view(&Default::default());
        let normal_view = normal_texture.create_view(&Default::default());
        let albedo_view = albedo_texture.create_view(&Default::default());
        
        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("GBuffer Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        // Create bind group layout
        let bind_group_layout = create_gbuffer_bind_group_layout(device);
        
        Self {
            bind_group_layout,
            position_texture,
            normal_texture,
            albedo_texture,
            position_view,
            normal_view,
            albedo_view,
            sampler,
            width,
            height,
        }
    }
}
```

**Resize Method:**

```rust
impl GBuffer {
    /// Resize the G-buffer to the new dimensions.
    pub fn resize(&mut self, device: &wgpu::Device, new_width: u32, new_height: u32) {
        self.width = new_width;
        self.height = new_height;
        
        // Recreate textures with new dimensions
        self.position_texture = create_gbuffer_texture(device, new_width, new_height, "Position");
        self.normal_texture = create_gbuffer_texture(device, new_width, new_height, "Normal");
        self.albedo_texture = create_gbuffer_texture(device, new_width, new_height, "Albedo");
        
        // Recreate texture views
        self.position_view = self.position_texture.create_view(&Default::default());
        self.normal_view = self.normal_texture.create_view(&Default::default());
        self.albedo_view = self.albedo_texture.create_view(&Default::default());
    }
}
```

**Bind Group Methods:**

```rust
impl GBuffer {
    /// Create a bind group for accessing the G-buffer in shaders.
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GBuffer Bind Group Layout"),
            entries: &[
                // Position texture
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
                // Normal texture
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
                // Albedo texture
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
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    /// Create a bind group for the G-buffer.
    pub fn create_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GBuffer Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
}
```

**Render Pass Helpers:**

```rust
impl GBuffer {
    /// Get the color formats for the G-buffer attachments.
    pub fn color_formats(&self) -> [wgpu::TextureFormat; 3] {
        [
            self.position_texture.format(),
            self.normal_texture.format(),
            self.albedo_texture.format(),
        ]
    }

    /// Get the color targets for the geometry pass.
    pub fn color_targets(&self) -> [Option<wgpu::RenderPassColorAttachment>; 3] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: &self.position_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.normal_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.albedo_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
        ]
    }

    /// Get the color attachments for the geometry pass.
    pub fn color_attachments(&self) -> [wgpu::RenderPassColorAttachment; 3] {
        [
            wgpu::RenderPassColorAttachment {
                view: &self.position_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            },
            wgpu::RenderPassColorAttachment {
                view: &self.normal_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            },
            wgpu::RenderPassColorAttachment {
                view: &self.albedo_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            },
        ]
    }
}
```

### Usage Example

```rust
// Create G-buffer
let mut gbuffer = GBuffer::new(device, width, height);

// In geometry pass
let color_attachments = gbuffer.color_attachments();
let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &color_attachments,
    ..Default::default()
});
// Draw geometry...

// In lighting pass
let bind_group = gbuffer.create_bind_group(device);
let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &output_texture_view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
    })],
    ..Default::default()
});
render_pass.set_bind_group(0, &bind_group, &[]);
// Draw full-screen quad with lighting shader...
```

---

## 5. device Module

### Overview

The `device` module provides **immutable GPU infrastructure** that can be shared across the application. This is a **core component of the Radical Separation architecture**.

**Key Concept:** Unlike the application state which changes every frame, the GPU infrastructure (device, queue, surface) is immutable after creation and can be safely shared between different parts of the application.

### Key Types

#### `SurfaceConfig` Struct

The `SurfaceConfig` struct provides thread-safe access to the wgpu surface configuration:

```rust
/// Configuration for the wgpu surface.
/// This provides thread-safe access to the surface configuration
/// and allows for surface reconfiguration when the window is resized.
#[derive(Debug)]
pub struct SurfaceConfig {
    /// The wgpu instance (needed for surface recreation)
    pub instance: wgpu::Instance,
    /// The window (needed for surface recreation)
    pub window: Arc<Window>,
    /// The wgpu surface, protected by a mutex for thread safety
    pub surface: Arc<Mutex<wgpu::Surface<'static>>>,
    /// The surface texture format
    pub format: wgpu::TextureFormat,
    /// The current size of the surface
    pub size: winit::dpi::PhysicalSize<u32>,
}
```

**Methods:**

```rust
impl SurfaceConfig {
    /// Create a new surface configuration.
    pub fn new(
        instance: wgpu::Instance,
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        format: wgpu::TextureFormat,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Self

    /// Lock the surface for exclusive access.
    pub fn lock_surface(&self) -> std::sync::MutexGuard<'_, wgpu::Surface<'static>>

    /// Configure the surface with the current settings.
    pub fn configure(&self, device: &wgpu::Device)

    /// Update the surface size and reconfigure.
    pub fn resize(&self, new_size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device)

    /// Try to acquire the current surface texture for rendering.
    pub fn get_current_texture(&self, device: &wgpu::Device) -> Option<wgpu::SurfaceTexture>

    /// Create a texture view from a surface texture using the surface format.
    pub fn create_texture_view(&self, surface_texture: &wgpu::SurfaceTexture) -> wgpu::TextureView
}
```

#### `GraphicsDevice` Struct

The `GraphicsDevice` struct represents the **immutable GPU infrastructure**:

```rust
/// Immutable GPU infrastructure that can be shared across the application.
/// This struct represents the "hardware" layer of the graphics system.
/// It contains the wgpu device, queue, instance, and surface configuration,
/// all of which are immutable after creation and can be safely shared between
/// different parts of the application.
#[derive(Debug)]
pub struct GraphicsDevice {
    /// The wgpu instance
    pub instance: wgpu::Instance,
    /// The logical device, wrapped in Arc for sharing
    pub device: Arc<wgpu::Device>,
    /// The command queue, wrapped in Arc for sharing
    pub queue: Arc<wgpu::Queue>,
    /// Surface configuration
    pub surface_config: SurfaceConfig,
    /// Window reference (for surface recreation if needed)
    pub window: Arc<Window>,
}
```

**Methods:**

```rust
impl GraphicsDevice {
    /// Create a new graphics device from a window and display handle.
    pub async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> Self

    /// Get a reference to the wgpu device.
    pub fn wgpu_device(&self) -> &wgpu::Device

    /// Get a reference to the wgpu queue.
    pub fn wgpu_queue(&self) -> &wgpu::Queue

    /// Get the current window size.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32>

    /// Resize the surface to the new size.
    pub fn resize(&self, new_size: winit::dpi::PhysicalSize<u32>)

    /// Request a redraw of the window.
    pub fn request_redraw(&self)

    /// Notify the window before presenting.
    pub fn pre_present_notify(&self)

    /// Get the surface format.
    pub fn surface_format(&self) -> wgpu::TextureFormat
}
```

### Usage Example

```rust
use renderlib::device::GraphicsDevice;

// Create a new graphics device
let display_handle = event_loop.owned_display_handle();
let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
let device = GraphicsDevice::new(display_handle, window.clone()).await;

// Access wgpu types
let wgpu_device = device.wgpu_device();
let wgpu_queue = device.wgpu_queue();
let surface_format = device.surface_format();
let size = device.size();

// Request redraw
device.request_redraw();

// Resize
device.resize(new_size);

// Share across threads
let device_arc = Arc::new(device);
```

### Thread Safety

The `GraphicsDevice` is designed to be shared across threads:

- `device` and `queue` are wrapped in `Arc` for thread-safe reference counting
- `surface` is protected by a `Mutex` for thread-safe access
- All methods are thread-safe

```rust
use std::sync::Arc;

let device = Arc::new(GraphicsDevice::new(display_handle, window).await);

// Share with multiple threads
let device_clone = Arc::clone(&device);
std::thread::spawn(move || {
    // Use device_clone in another thread
    let wgpu_device = device_clone.wgpu_device();
    // ...
});
```

---

## 6. device_helpers Module

### Overview

The `device_helpers` module provides **utility functions and builders** for common wgpu operations. These helpers make it easier to work with wgpu by providing ergonomic wrappers around common patterns.

### Functions

#### Buffer Creation

```rust
/// Create a new buffer with the specified usage and size.
pub fn create_buffer(
    device: &wgpu::Device,
    size: wgpu::BufferAddress,
    usage: wgpu::BufferUsages,
    label: Option<&str>,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label,
        size,
        usage,
        mapped_at_creation: false,
    })
}

/// Create a buffer and initialize it with the given data.
pub fn create_buffer_from_slice<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &[T],
    usage: wgpu::BufferUsages,
    label: Option<&str>,
) -> wgpu::Buffer {
    let size = (std::mem::size_of::<T>() * data.len()) as wgpu::BufferAddress;
    let buffer = create_buffer(device, size, usage | wgpu::BufferUsages::COPY_DST, label);
    
    // Write data to buffer
    let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Staging Buffer"),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::COPY_SRC,
    });
    
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(&staging_buffer, 0, &buffer, 0, size);
    
    let command_buffer = encoder.finish();
    device.queue().submit(std::iter::once(command_buffer));
    
    buffer
}
```

#### Shader Management

```rust
/// Load shader source code from a file.
pub fn load_shader_source(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

/// Create a shader module from source code.
pub fn create_shader_module(
    device: &wgpu::Device,
    source: &str,
    label: Option<&str>,
) -> Result<wgpu::ShaderModule, wgpu::ShaderModuleDescriptorError> {
    let desc = wgpu::ShaderModuleDescriptor {
        label,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    };
    Ok(device.create_shader_module(&desc)?)
}

/// Create a shader module from a file.
pub fn create_shader_module_from_file(
    device: &wgpu::Device,
    path: &str,
) -> Result<wgpu::ShaderModule, Box<dyn std::error::Error>> {
    let source = load_shader_source(path)?;
    Ok(create_shader_module(device, &source, Some(path))?)
}
```

#### Pipeline Building

```rust
/// Create a pipeline layout with the specified bind group layouts.
pub fn create_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    label: Option<&str>,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label,
        bind_group_layouts,
        push_constant_ranges: &[],
    })
}

/// Builder for creating render pipelines with a fluent API.
pub struct RenderPipelineBuilder<'a> {
    device: &'a wgpu::Device,
    label: Option<String>,
    layout: Option<wgpu::PipelineLayout>,
    shader_module: Option<wgpu::ShaderModule>,
    vertex_entry: Option<String>,
    fragment_entry: Option<String>,
    vertex_buffers: Vec<wgpu::VertexBufferLayout>,
    color_formats: Vec<wgpu::TextureFormat>,
    blend_states: Vec<wgpu::BlendState>,
    depth_stencil: Option<wgpu::DepthStencilState>,
    primitive: wgpu::PrimitiveState,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self
    pub fn with_label(mut self, label: impl Into<String>) -> Self
    pub fn with_layout(mut self, layout: wgpu::PipelineLayout) -> Self
    pub fn with_shader_module(mut self, shader_module: wgpu::ShaderModule) -> Self
    pub fn with_vertex_entry(mut self, entry: impl Into<String>) -> Self
    pub fn with_fragment_entry(mut self, entry: impl Into<String>) -> Self
    pub fn with_vertex_buffers(mut self, buffers: impl Into<Vec<wgpu::VertexBufferLayout>>) -> Self
    pub fn with_color_formats(mut self, formats: impl Into<Vec<wgpu::TextureFormat>>) -> Self
    pub fn with_blend_states(mut self, states: impl Into<Vec<wgpu::BlendState>>) -> Self
    pub fn with_depth_stencil(mut self, depth_stencil: wgpu::DepthStencilState) -> Self
    pub fn with_primitive(mut self, primitive: wgpu::PrimitiveState) -> Self
    pub fn build(self) -> Result<wgpu::RenderPipeline, wgpu::PipelineCreationError>
}
```

**Example:**

```rust
use renderlib::device_helpers::RenderPipelineBuilder;

let pipeline = RenderPipelineBuilder::new(device)
    .with_label("My Pipeline")
    .with_shader_module(shader_module)
    .with_vertex_entry("vs_main")
    .with_fragment_entry("fs_main")
    .with_vertex_buffers(vec![vertex_buffer_layout])
    .with_color_formats(vec![surface_format])
    .with_primitive(wgpu::PrimitiveState::default())
    .build()?;
```

#### Bind Group Helpers

```rust
/// Create a uniform bind group layout.
pub fn create_uniform_bind_group_layout(
    device: &wgpu::Device,
    label: Option<&str>,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// Create a uniform bind group.
pub fn create_uniform_bind_group(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    layout: &wgpu::BindGroupLayout,
    label: Option<&str>,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: 0,
                size: None,
            }),
        }],
    })
}
```

#### Depth Texture

```rust
/// Create a depth texture for use as a render target.
pub fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: Option<&str>,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    
    (texture, view)
}
```

---

## 7. geometry Module

### Submodules

The `geometry` module contains two submodules:
- `mod.rs`: Vertex type definitions
- `primitives.rs`: Primitive generators

### Vertex Types (`mod.rs`)

#### `PosColorVertex`

A vertex with position and color attributes:

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl PosColorVertex {
    /// Create a vertex buffer layout descriptor for this vertex type.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PosColorVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
```

#### `PosColorNormalVertex`

A vertex with position, color, and normal attributes:

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosColorNormalVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
}

impl PosColorNormalVertex {
    /// Create a vertex buffer layout descriptor for this vertex type.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PosColorNormalVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
```

### Primitive Generators (`primitives.rs`)

#### `triangle_vertices()`

Generates vertices for a simple triangle:

```rust
/// Generate vertices for a colored triangle.
pub fn triangle_vertices() -> Vec<PosColorVertex> {
    vec![
        PosColorVertex {
            position: [0.0, 0.5, 0.0],
            color: [1.0, 0.0, 0.0], // Red
        },
        PosColorVertex {
            position: [-0.5, -0.5, 0.0],
            color: [0.0, 1.0, 0.0], // Green
        },
        PosColorVertex {
            position: [0.5, -0.5, 0.0],
            color: [0.0, 0.0, 1.0], // Blue
        },
    ]
}
```

#### `cube_vertices()`

Generates vertices for a colored cube:

```rust
/// Generate vertices for a colored cube.
/// Returns vertices and indices for indexed rendering.
pub fn cube_vertices() -> (Vec<PosColorNormalVertex>, Vec<u16>) {
    // Define cube vertices with positions, colors, and normals
    let vertices = vec![
        // Front face
        PosColorNormalVertex { position: [-1.0, -1.0,  1.0], color: [1.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] },
        PosColorNormalVertex { position: [ 1.0, -1.0,  1.0], color: [0.0, 1.0, 0.0], normal: [0.0, 0.0, 1.0] },
        PosColorNormalVertex { position: [ 1.0,  1.0,  1.0], color: [0.0, 0.0, 1.0], normal: [0.0, 0.0, 1.0] },
        PosColorNormalVertex { position: [-1.0,  1.0,  1.0], color: [1.0, 1.0, 0.0], normal: [0.0, 0.0, 1.0] },
        // Back face, etc...
    ];
    
    // Define indices for cube (12 triangles = 36 indices)
    let indices: Vec<u16> = vec![
        // Front face
        0, 1, 2, 0, 2, 3,
        // Back face, etc...
    ];
    
    (vertices, indices)
}
```

---

## 8. input Module

### Overview

The `input` module provides **input state tracking** for frame-rate independent movement and controls. This is a **new module** added as part of the Radical Separation architecture.

### Key Types

#### `MouseMode` Enum

Controls how mouse look is handled:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    /// Normal mode: mouse look is only enabled while Shift is held down.
    Normal,
    /// Grabbed mode: mouse look is constantly enabled.
    Grabbed,
}

impl Default for MouseMode {
    fn default() -> Self {
        Self::Normal
    }
}
```

#### `MouseDelta` Struct

Represents mouse movement for a frame:

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseDelta {
    /// Horizontal mouse movement (pixels).
    pub x: f32,
    /// Vertical mouse movement (pixels).
    pub y: f32,
}

impl MouseDelta {
    /// Creates a new MouseDelta with zero movement.
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Creates a new MouseDelta with the given values.
    pub fn new_with(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
```

#### `InputState` Struct

Tracks basic input state (part of `AppState`):

```rust
#[derive(Debug, Default)]
pub struct InputState {
    /// Currently pressed keys
    pub pressed_keys: Vec<winit::keyboard::Key>,
    /// Mouse position
    pub mouse_position: Option<(f64, f64)>,
    /// Mouse buttons pressed
    pub mouse_buttons: Vec<u16>,
    /// Scroll delta
    pub scroll_delta: (f64, f64),
}

impl InputState {
    /// Create a new input state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the mouse position.
    pub fn set_mouse_position(&mut self, x: f64, y: f64)

    /// Clear the scroll delta (should be called after processing).
    pub fn clear_scroll(&mut self)
}
```

#### `InputController` Struct

The main input controller that tracks key states and handles window events:

```rust
#[derive(Debug)]
pub struct InputController {
    /// Set of currently pressed keys (stored as lowercase strings)
    pressed_keys: HashSet<String>,
    /// Current mouse movement delta for the frame.
    mouse_delta: MouseDelta,
    /// Previous cursor position for calculating delta.
    prev_cursor_pos: Option<(f64, f64)>,
    /// Current mouse input mode.
    mouse_mode: MouseMode,
    /// Whether shift key is currently pressed (for normal mode mouse look).
    shift_pressed: bool,
}

impl Default for InputController {
    fn default() -> Self {
        Self::new()
    }
}
```

**Methods:**

```rust
impl InputController {
    /// Creates a new, empty InputController.
    pub fn new() -> Self

    /// Processes a window event and updates the internal key and mouse state.
    pub fn handle_window_event(&mut self, event: &WindowEvent)

    /// Checks if a specific key is currently pressed.
    pub fn is_key_pressed(&self, key: &str) -> bool

    /// Returns a reference to the set of currently pressed keys.
    pub fn pressed_keys(&self) -> &HashSet<String>

    /// Returns the current mouse mode.
    pub fn get_mouse_mode(&self) -> MouseMode

    /// Sets the mouse mode.
    pub fn set_mouse_mode(&mut self, mode: MouseMode)

    /// Returns whether shift key is currently pressed.
    pub fn is_shift_pressed(&self) -> bool

    /// Returns whether mouse look should be active based on current mode.
    pub fn is_mouse_look_active(&self) -> bool

    /// Clears all pressed key states.
    pub fn clear(&mut self)

    /// Gets the current mouse delta and resets it to zero.
    pub fn take_mouse_delta(&mut self) -> MouseDelta

    /// Gets the current mouse delta without resetting it.
    pub fn get_mouse_delta(&self) -> MouseDelta

    /// Resets the mouse delta to zero.
    pub fn reset_mouse_delta(&mut self)

    /// Creates a PlayerInput with mouse delta filtered based on mouse mode.
    pub fn get_player_input(&mut self) -> crate::player::PlayerInput
}
```

### Usage Example

```rust
use renderlib::input::{InputController, MouseMode};

// Create input controller
let mut input = InputController::new();

// In your input handler:
input.handle_window_event(&event);

// In your render loop:
if input.is_key_pressed("w") {
    // Move forward
}
if input.is_key_pressed("a") {
    // Move left
}

// Get mouse delta for camera control
let delta = input.take_mouse_delta();
if input.is_mouse_look_active() {
    // Apply mouse look
    camera.orbit(delta.x, delta.y);
}

// Get player input (automatically filtered based on mouse mode)
let player_input = input.get_player_input();
player.apply_input(&player_input, delta_time);
```

### Mouse Mode Handling

The `InputController` supports two mouse modes:

1. **Normal Mode**: Mouse look is only active when Shift is pressed
2. **Grabbed Mode**: Mouse look is always active

Toggle between modes with the tilde key (`):

```rust
// In handle_window_event:
if let Key::Character(c) = &key_event.logical_key {
    let key_str = c.to_ascii_lowercase();
    if key_str == "`" && key_event.state.is_pressed() {
        self.mouse_mode = match self.mouse_mode {
            MouseMode::Normal => MouseMode::Grabbed,
            MouseMode::Grabbed => MouseMode::Normal,
        };
    }
}
```

---

## 9. mesh Module

### Overview

The `mesh` module provides **GLTF/GLB mesh loading and management** with a central `MeshCache` for efficient resource management. This module has been **enhanced in Phase 2** with source deduplication.

### Key Types

#### `BoundingBox` Struct

Represents the axis-aligned bounding box of a mesh:

```rust
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl BoundingBox {
    /// Create a new bounding box from min and max points.
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self

    /// Calculate the scale factor needed to fit the bounding box in a unit cube.
    pub fn scale_factor(&self) -> f32

    /// Calculate the center point of the bounding box.
    pub fn center(&self) -> [f32; 3]
}
```

#### `Mesh` Struct

Represents a loaded mesh with vertices, indices, and metadata:

```rust
#[derive(Debug)]
pub struct Mesh {
    /// Vertex data
    pub vertices: Vec<u8>,
    /// Index data
    pub indices: Vec<u8>,
    /// Bounding box of the mesh
    pub bounding_box: BoundingBox,
    /// Scale factor to normalize the mesh
    pub scale: f32,
    /// Center point of the mesh
    pub center: [f32; 3],
}

impl Mesh {
    /// Create a new mesh from vertex and index data.
    pub fn new(vertices: Vec<u8>, indices: Vec<u8>, bounding_box: BoundingBox) -> Self

    /// Create GPU buffers for this mesh.
    pub fn create_buffers(
        &self,
        device: &wgpu::Device,
    ) -> Result<(wgpu::Buffer, wgpu::Buffer), wgpu::BufferAsyncError>
}
```

#### `MeshSource` Enum

Represents the source of a mesh (file path or built-in primitive):

```rust
#[derive(Debug, Clone)]
pub enum MeshSource {
    /// Load from a file path
    Path(String),
    /// Use a built-in primitive
    Primitive(PrimitiveType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Triangle,
    Cube,
}

impl MeshSource {
    // Implementations for Clone, Hash, PartialEq for deduplication
}
```

#### `MeshHandle` Type

A handle to a mesh in the cache:

```rust
pub type MeshHandle = u64;
```

#### `MeshAsset` Struct

The CPU-side representation of a mesh:

```rust
#[derive(Debug)]
pub struct MeshAsset {
    pub vertices: Vec<u8>,
    pub indices: Vec<u8>,
    pub bounding_box: BoundingBox,
    pub scale: f32,
    pub center: [f32; 3],
}
```

#### `MeshResource` Struct

The GPU-side representation of a mesh:

```rust
#[derive(Debug)]
pub struct MeshResource {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}
```

#### `MeshLoadError` Enum

Error types for mesh loading:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MeshLoadError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Import error: {0}")]
    ImportError(String),
    #[error("No meshes found in file")]
    NoMeshesFound,
    #[error("No vertices loaded")]
    NoVerticesLoaded,
    #[error("No position attribute found")]
    NoPositionAttribute,
}
```

#### `MeshCache` Struct

The central cache for managing mesh assets and GPU resources:

```rust
#[derive(Debug)]
pub struct MeshCache {
    device: wgpu::Device,
    cpu_assets: RefCell<HashMap<MeshHandle, Arc<MeshAsset>>>,
    gpu_resources: RefCell<HashMap<MeshHandle, Arc<MeshResource>>>,
    source_to_handle: RefCell<HashMap<MeshSource, MeshHandle>>,
    next_handle: MeshHandle,
}

impl MeshCache {
    /// Create a new mesh cache.
    pub fn new(device: &wgpu::Device) -> Self

    /// Load a mesh from the given source (old method, uses RefCell).
    pub fn load(&self, source: &MeshSource) -> Result<MeshHandle, MeshLoadError>

    /// Load a mesh from the given source (new method, more efficient).
    pub fn load_mut(&mut self, source: &MeshSource) -> Result<MeshHandle, MeshLoadError>

    /// Get the CPU asset for a mesh handle.
    pub fn get_asset(&self, handle: MeshHandle) -> Option<Arc<MeshAsset>>

    /// Get the GPU resource for a mesh handle.
    pub fn get_resource(&self, handle: MeshHandle) -> Option<Arc<MeshResource>>

    /// Get both CPU asset and GPU resource for a mesh handle.
    pub fn get_both(&self, handle: MeshHandle) -> Option<(Arc<MeshAsset>, Arc<MeshResource>)>
}
```

### Functions

```rust
/// Load a GLTF/GLB file from the given path.
pub fn load_gltf(path: &str) -> Result<Mesh, MeshLoadError>
```

### Usage Example

```rust
use renderlib::mesh::{MeshCache, MeshSource};

// In your renderer initialization:
let mesh_handle = context.state().mesh_cache.load_mut(&MeshSource::Path("mesh.gltf".to_string()))?;

// Get mesh asset and resource
let (asset, resource) = context.state().mesh_cache.get_both(mesh_handle)?;

// Use vertex and index buffers for rendering
let vertex_buffer = &resource.vertex_buffer;
let index_buffer = &resource.index_buffer;
let num_indices = resource.num_indices;

// Draw
render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
render_pass.draw_indexed(0..num_indices, 0, 0..1);
```

---

## 10. player Module

### Overview

The `player` module provides **first-person camera control** with frame-rate independent movement. This is a **new module** added as part of the Radical Separation architecture.

### Constants

```rust
/// Movement speed in units per second.
const DEFAULT_MOVE_SPEED: f32 = 2.5;

/// Deceleration rate in units per second per second.
const DEFAULT_DECELERATION: f32 = 10.0;

/// Acceleration rate in units per second per second.
const DEFAULT_ACCELERATION: f32 = 20.0;

/// Mouse look sensitivity (radians per pixel).
const DEFAULT_MOUSE_SENSITIVITY: f32 = 0.002;
```

### Key Types

#### `PlayerInput` Struct

Represents the input state for a player in a single frame:

```rust
#[derive(Debug, Clone, Default)]
pub struct PlayerInput {
    /// Whether the player should move forward.
    pub move_forward: bool,
    /// Whether the player should move backward.
    pub move_backward: bool,
    /// Whether the player should move left.
    pub move_left: bool,
    /// Whether the player should move right.
    pub move_right: bool,
    /// Mouse movement delta for this frame.
    pub mouse_delta: MouseDelta,
}

impl PlayerInput {
    /// Creates a new PlayerInput with all movement flags set to false.
    pub fn new() -> Self

    /// Set the forward movement flag.
    pub fn with_move_forward(mut self, forward: bool) -> Self

    /// Set the backward movement flag.
    pub fn with_move_backward(mut self, backward: bool) -> Self

    /// Set the left movement flag.
    pub fn with_move_left(mut self, left: bool) -> Self

    /// Set the right movement flag.
    pub fn with_move_right(mut self, right: bool) -> Self

    /// Set the mouse delta.
    pub fn with_mouse_delta(mut self, delta: MouseDelta) -> Self
}
```

#### `MovementSettings` Struct

Controls how the player moves:

```rust
#[derive(Debug, Clone)]
pub struct MovementSettings {
    /// Base movement speed in units per second.
    pub move_speed: f32,
    /// How quickly velocity decreases when no input (higher = faster deceleration).
    pub deceleration: f32,
    /// How quickly velocity increases when input is applied (higher = faster acceleration).
    pub acceleration: f32,
    /// Mouse look sensitivity in radians per pixel.
    pub mouse_sensitivity: f32,
}

impl Default for MovementSettings {
    fn default() -> Self {
        Self {
            move_speed: DEFAULT_MOVE_SPEED,
            deceleration: DEFAULT_DECELERATION,
            acceleration: DEFAULT_ACCELERATION,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }
}

impl MovementSettings {
    /// Create new movement settings with the given speed.
    pub fn with_speed(speed: f32) -> Self

    /// Set the movement speed.
    pub fn set_move_speed(&mut self, speed: f32)

    /// Set the deceleration rate.
    pub fn set_deceleration(&mut self, deceleration: f32)

    /// Set the acceleration rate.
    pub fn set_acceleration(&mut self, acceleration: f32)

    /// Set the mouse sensitivity.
    pub fn set_mouse_sensitivity(&mut self, sensitivity: f32)
}
```

#### `PlayerState` Struct

The main player state that manages position, velocity, and camera:

```rust
#[derive(Debug)]
pub struct PlayerState {
    /// Current position in world space.
    position: Point3<f32>,
    /// Current velocity (for smooth movement).
    velocity: Vector3<f32>,
    /// Camera reference (updated by player movement).
    camera: Camera,
    /// Movement settings.
    movement_settings: MovementSettings,
}

impl PlayerState {
    /// Create a new player state with the given camera.
    pub fn new(camera: Camera) -> Self

    /// Create a new player state with custom movement settings.
    pub fn with_settings(camera: Camera, settings: MovementSettings) -> Self

    /// Apply input to the player and update camera.
    pub fn apply_input(&mut self, input: &PlayerInput, delta_time: f32)

    /// Get a reference to the camera.
    pub fn get_camera(&self) -> &Camera

    /// Get a mutable reference to the camera.
    pub fn get_camera_mut(&mut self) -> &mut Camera

    /// Get the current position.
    pub fn get_position(&self) -> Point3<f32>

    /// Set the position.
    pub fn set_position(&mut self, position: Point3<f32>)

    /// Get the movement settings.
    pub fn get_movement_settings(&self) -> &MovementSettings

    /// Get a mutable reference to the movement settings.
    pub fn get_movement_settings_mut(&mut self) -> &mut MovementSettings
}
```

### Usage Example

```rust
use renderlib::player::{PlayerState, PlayerInput, MovementSettings};
use renderlib::camera::Camera;

// Create a camera
let camera = Camera::new();

// Create player state
let mut player = PlayerState::new(camera);

// Customize movement settings
player.get_movement_settings_mut().set_move_speed(5.0);
player.get_movement_settings_mut().set_mouse_sensitivity(0.003);

// In your render loop:
// Get input from InputController
let player_input = input_controller.get_player_input();

// Apply input with delta time
player.apply_input(&player_input, delta_time);

// Get updated camera for rendering
let camera = player.get_camera();
let camera_uniform = CameraUniform::from_camera(camera, aspect_ratio);
```

### How It Works

The `PlayerState::apply_input` method:

1. **Applies mouse delta** to rotate the camera (yaw and pitch)
2. **Calculates target velocity** based on input flags and movement settings
3. **Applies acceleration/deceleration** to smoothly change velocity
4. **Updates position** based on velocity and delta time
5. **Updates camera** position and target

This provides **smooth, frame-rate independent** movement that feels natural regardless of framerate.

---

## 11. state Module

### Overview

The `state` module provides **mutable application state** that changes during runtime. This is a **core component of the Radical Separation architecture** and contains all the mutable data for the application.

**Key Concept:** Unlike the immutable `GraphicsDevice`, the `AppState` contains data that changes every frame (mesh cache, camera position, input state, etc.).

### Key Types

#### `InputState` Struct

Tracks basic input state:

```rust
#[derive(Debug, Default)]
pub struct InputState {
    pub pressed_keys: Vec<winit::keyboard::Key>,
    pub mouse_position: Option<(f64, f64)>,
    pub mouse_buttons: Vec<u16>,
    pub scroll_delta: (f64, f64),
}

impl InputState {
    pub fn new() -> Self
    pub fn set_mouse_position(&mut self, x: f64, y: f64)
    pub fn clear_scroll(&mut self)
}
```

#### `TimeState` Struct

Tracks timing information:

```rust
#[derive(Debug)]
pub struct TimeState {
    /// Total time since application start (in seconds)
    pub total_time: f64,
    /// Time since last frame (in seconds)
    pub delta_time: f64,
    /// Frame count
    pub frame_count: u64,
    /// Time when the application started
    pub start_time: std::time::Instant,
}

impl TimeState {
    /// Create a new time state.
    pub fn new() -> Self

    /// Update the time state for a new frame.
    pub fn update(&mut self)
}
```

#### `AppState` Struct

The main application state struct:

```rust
#[derive(Debug)]
pub struct AppState {
    /// Central cache for managing mesh assets and GPU resources.
    pub mesh_cache: MeshCache,
    /// Main camera for the scene.
    pub camera: Camera,
    /// Input state for tracking user input.
    pub input: InputState,
    /// Timing information.
    pub time: TimeState,
    /// Currently active mesh handle (for debugging/demonstration).
    pub active_mesh: Option<MeshHandle>,
}

impl AppState {
    /// Create a new application state with the given wgpu device.
    pub fn new(device: &wgpu::Device) -> Self

    /// Update the time state for a new frame.
    pub fn update_time(&mut self)

    /// Set the active mesh handle.
    pub fn set_active_mesh(&mut self, handle: MeshHandle)

    /// Clear the active mesh handle.
    pub fn clear_active_mesh(&mut self)

    /// Get the active mesh handle.
    pub fn get_active_mesh(&self) -> Option<MeshHandle>

    /// Load a mesh and set it as active.
    pub fn load_and_set_active(
        &mut self,
        source: &MeshSource,
    ) -> Result<MeshHandle, MeshLoadError>
}
```

### Usage Example

```rust
use renderlib::state::AppState;

// Create application state (called by Application during initialization)
let mut state = AppState::new(device);

// Access components
let mesh_cache = &mut state.mesh_cache;
let camera = &mut state.camera;
let input = &mut state.input;
let time = &mut state.time;

// Update time at start of frame
state.update_time();
let delta_time = state.time.delta_time;

// Load a mesh
let mesh_handle = state.mesh_cache.load_mut(&MeshSource::Path("mesh.gltf".to_string()))?;
state.set_active_mesh(mesh_handle);

// Get active mesh
if let Some(handle) = state.get_active_mesh() {
    let (asset, resource) = state.mesh_cache.get_both(handle)?;
    // Use mesh...
}
```

### Integration with RenderContext

In the new architecture, `AppState` is accessed via `RenderContext`:

```rust
fn render(&mut self, mut context: RenderContext<'_>) {
    // Get mutable access to state
    let state = context.state();
    
    // Access mesh cache
    let mesh_handle = state.mesh_cache.load_mut(&source)?;
    
    // Access camera
    let camera = &state.camera;
    
    // Access input
    let input = &state.input;
    
    // Access time
    let time = &state.time;
    let delta_time = time.delta_time;
}
```

---

## 12. lib.rs

### Overview

The `lib.rs` file is the **library root** that exports all public modules.

### Module Exports

```rust
//! Renderlib - A wgpu/winit framework for graphics applications.
//!
//! This library provides a foundation for building graphics applications with wgpu and winit,
//! including application framework, graphics context management, device helpers, and
//! common geometry types.

pub mod app;
pub mod camera;
pub mod context;
pub mod deferred;
pub mod device;
pub mod device_helpers;
pub mod geometry;
pub mod input;
pub mod mesh;
pub mod player;
pub mod state;
```

### Documentation

The library documentation is generated from the module-level documentation in each file. Run `cargo doc` to generate HTML documentation.

---

## Summary

Renderlib provides **11 modules** organized into clear layers:

| Layer | Modules | Purpose |
|-------|---------|---------|
| **Framework** | app, context | Application management, resource access |
| **Infrastructure** | device, state | Immutable GPU, mutable state |
| **Core Systems** | camera, geometry, mesh | Camera, shapes, assets |
| **Input/Control** | input, player | User input, camera control |
| **Rendering** | deferred, device_helpers | Deferred rendering, helpers |

**All modules work together** to provide a complete, flexible foundation for building graphics applications with the new Radical Separation architecture.
