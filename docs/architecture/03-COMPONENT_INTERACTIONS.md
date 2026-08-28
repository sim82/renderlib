# Component Interactions

## Startup Sequence

```
1. Event loop resumed
2. Window created
3. GraphicsDevice initialized (async):
   - Create wgpu instance
   - Request adapter
   - Request device and queue (wrapped in Arc)
   - Create surface
   - Configure surface
4. AppState initialized:
   - Create mesh cache
   - Create camera with defaults
   - Create input state
   - Create time state
5. Renderer initialized via AppRenderer::init(RenderContext)
6. First redraw requested
```

## Render Loop

```
1. Redraw requested
2. Acquire surface texture (handles Suboptimal/Outdated/Lost)
3. Create texture view
4. Create RenderContext with:
   - Reference to GraphicsDevice
   - Mutable reference to AppState
   - Texture view
5. Application calls Renderer::render(RenderContext)
6. Renderer:
   - Accesses device via context.wgpu_device()
   - Accesses queue via context.wgpu_queue()
   - Accesses state via context.state()
   - Gets texture view via context.get_texture_view()
   - Creates command encoder
   - Begins render pass
   - Sets pipeline, bind groups, buffers
   - Draws
   - Ends render pass
7. Submit command buffer
8. Present surface texture
9. Request next redraw (for animation)
```

## Resize Handling

```
1. Window resized to new_size
2. GraphicsDevice::resize(new_size) updates surface config
3. Application creates RenderContext
4. Application calls Renderer::resize(RenderContext, new_size)
5. Renderer recreates size-dependent resources:
   - Depth texture
   - G-buffer (if deferred)
   - Swap chain resources
6. Redraw requested
```

## Mesh Loading

```
1. Renderer calls context.state().mesh_cache.load_mut(&source)
2. MeshCache:
   - Checks if source already loaded (deduplication)
   - Loads mesh from file or generates primitive
   - Creates MeshAsset (CPU)
   - Creates MeshResource (GPU buffers)
   - Returns MeshHandle
3. Renderer uses handle to access mesh via:
   - get_asset(handle) for CPU data
   - get_resource(handle) for GPU buffers
   - get_both(handle) for both
```

## Deferred Rendering Pipeline

### Geometry Pass
```
1. Bind geometry pipeline
2. Set vertex/index buffers for mesh
3. Set geometry uniform (MVP, model matrices)
4. Draw mesh to G-buffer:
   - Position texture
   - Normal texture
   - Albedo texture
```

### Lighting Pass
```
1. Bind lighting pipeline
2. Set full-screen quad vertex buffer
3. Set G-buffer bind group (position, normal, albedo textures + sampler)
4. Set lighting uniform (view position, lights)
5. Draw full-screen quad
6. Fragment shader:
   - Samples G-buffer textures
   - Calculates lighting per pixel
   - Outputs final color
```

## Uniform Buffer Management

```
1. Create uniform buffer once during init:
   - Size based on uniform struct
   - Usage: UNIFORM | COPY_DST
2. Update every frame in render():
   - Calculate new uniform values
   - queue.write_buffer(buffer, offset, data)
3. Use in rendering:
   - Buffer bound to bind group
   - Bind group set in render pass
```

## Bind Group Organization

### Triangle Demo
```
Bind Group 0:
└── Uniform Buffer: GeometryUniform (MVP matrix)
```

### Forward Demo
```
Bind Group 0:
└── Uniform Buffer: GeometryUniform (MVP, model)

Bind Group 1:
└── Uniform Buffer: LightingUniform (view position, lights)
```

### Deferred Demo
```
Geometry Pass:
Bind Group 0:
└── Uniform Buffer: GeometryUniform

Lighting Pass:
Bind Group 0:
├── Texture: Position (binding 0)
├── Texture: Normal (binding 1)
├── Texture: Albedo (binding 2)
└── Sampler: G-buffer sampler (binding 3)

Bind Group 1:
└── Uniform Buffer: LightingUniform
```
