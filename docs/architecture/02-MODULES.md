# Modules

## app
Provides the application framework.

**Types:**
- `Application<R>` - Main application struct, implements `ApplicationHandler`
- `AppRenderer` - Trait that renderers must implement with `init()`, `render()`, `resize()`, `input()` methods

**Usage:**
```rust
struct MyRenderer { /* resources */ }

impl AppRenderer for MyRenderer {
    async fn init(context: RenderContext<'_>) -> Self { /* setup */ }
    fn render(&mut self, context: RenderContext<'_>) { /* draw */ }
    fn resize(&mut self, context: RenderContext<'_>, size: PhysicalSize<u32>) { /* recreate resources */ }
    fn input(&mut self, context: RenderContext<'_>, event: &WindowEvent) { /* handle input */ }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = Application::<MyRenderer>::new();
    event_loop.run_app(&mut app).unwrap();
}
```

## camera
Camera, transforms, and lighting.

**Types:**
- `Camera` - Position, target, up, FOV, near/far planes
- `CameraUniform` - View, projection, view-projection matrices for shaders
- `Transform` - Translation, rotation, scale with model matrix
- `Light` - Position and color
- `LightingUniform` - View position and light array for shaders

## context
Render context for accessing resources.

**Types:**
- `RenderContext<'a>` - Provides access to `GraphicsDevice` and `AppState`

**Methods:**
- `wgpu_device()` - Access wgpu device
- `wgpu_queue()` - Access wgpu queue
- `state()` - Access mutable application state
- `get_texture_view()` - Get current texture view
- `size()` - Get window size
- `surface_format()` - Get surface format

## deferred
G-buffer for deferred rendering.

**Types:**
- `GBuffer` - Position, normal, albedo textures with views and sampler

**Methods:**
- `new(device, width, height)` - Create G-buffer
- `resize(device, new_width, new_height)` - Resize G-buffer
- `bind_group_layout(device)` - Create bind group layout
- `create_bind_group(device)` - Create bind group for shader access
- `color_attachments()` - Get color attachments for geometry pass

## device
GPU infrastructure.

**Types:**
- `GraphicsDevice` - Instance, device (Arc), queue (Arc), surface config, window (Arc)
- `SurfaceConfig` - Surface, format, size with thread-safe access

**Methods:**
- `new(display_handle, window)` - Create graphics device (async)
- `wgpu_device()` - Access wgpu device
- `wgpu_queue()` - Access wgpu queue
- `size()` - Get window size
- `resize(new_size)` - Resize surface
- `surface_format()` - Get surface format
- `request_redraw()` - Request window redraw

## device_helpers
Utilities for common wgpu operations.

**Functions:**
- `create_buffer(device, size, usage, label)` - Create buffer
- `create_buffer_from_slice(device, data, usage, label)` - Create buffer with data
- `load_shader_source(path)` - Load shader source from file
- `create_shader_module(device, source, label)` - Create shader module
- `create_pipeline_layout(device, bind_group_layouts, label)` - Create pipeline layout

**Types:**
- `RenderPipelineBuilder<'a>` - Fluent API for creating render pipelines

## geometry
Vertex types and primitive generators.

**Types:**
- `PosColorVertex` - Position and color
- `PosColorNormalVertex` - Position, color, and normal

**Functions:**
- `triangle_vertices()` - Generate triangle vertices
- `cube_vertices()` - Generate cube vertices and indices

Each vertex type has a `desc()` method returning its `VertexBufferLayout`.

## input
Input state tracking.

**Types:**
- `InputController` - Tracks pressed keys and mouse movement
- `InputState` - Basic input state (keys, mouse position, buttons, scroll)
- `MouseDelta` - Mouse movement delta (x, y)
- `MouseMode` - Normal (mouse look with Shift) or Grabbed (always on)

**Methods:**
- `handle_window_event(event)` - Process window event
- `is_key_pressed(key)` - Check if key is pressed
- `take_mouse_delta()` - Get and reset mouse delta
- `get_player_input()` - Get filtered player input

## mesh
Mesh loading and caching.

**Types:**
- `MeshCache` - Central cache for mesh assets and GPU resources
- `Mesh` - Vertex and index data with bounding box
- `MeshSource` - Source of mesh (Path or Primitive)
- `MeshHandle` - Handle to a mesh in the cache
- `MeshAsset` - CPU-side mesh representation
- `MeshResource` - GPU-side mesh representation (vertex buffer, index buffer)
- `BoundingBox` - Min/max bounds

**Methods:**
- `MeshCache::new(device)` - Create mesh cache
- `MeshCache::load_mut(source)` - Load mesh
- `MeshCache::get_asset(handle)` - Get CPU asset
- `MeshCache::get_resource(handle)` - Get GPU resource
- `MeshCache::get_both(handle)` - Get both asset and resource

## player
First-person camera control.

**Types:**
- `PlayerState` - Position, velocity, camera, movement settings
- `PlayerInput` - Movement flags and mouse delta for a frame
- `MovementSettings` - Speed, acceleration, deceleration, mouse sensitivity

**Methods:**
- `PlayerState::new(camera)` - Create player state
- `PlayerState::apply_input(input, delta_time)` - Apply input and update camera
- `PlayerState::get_camera()` - Get camera reference

## state
Application state.

**Types:**
- `AppState` - Mesh cache, camera, input, time, active mesh
- `TimeState` - Total time, delta time, frame count

**Methods:**
- `AppState::new(device)` - Create application state
- `AppState::update_time()` - Update time for new frame
- `AppState::load_and_set_active(source)` - Load mesh and set as active
