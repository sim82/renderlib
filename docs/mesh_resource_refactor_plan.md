# Mesh Resource Refactor Plan

## Status
- ✅ **Phase 1: Core Types** - COMPLETE
- ✅ **Phase 2: Unified Cache** - COMPLETE
- ⏳ Phase 3: GraphicsContext Integration
- ⏳ Phase 4: Migrate Renderers
- ⏳ Phase 5: Cleanup and Validation
- ⏳ Phase 6: Documentation

---

## Phase 1 Implementation Summary

### Completed Tasks
1. ✅ **Added `MeshHandle`**: Opaque identifier for cached meshes using `AtomicU64` for ID generation.
2. ✅ **Renamed `Mesh` to `MeshAsset`**: CPU-side mesh data with all original fields plus a `name` field.
3. ✅ **Added `MeshResource`**: GPU-side struct containing `vertex_buffer`, `index_buffer`, `num_indices`, and `vertex_layout`.
4. ✅ **Updated `MeshAsset` methods**:
   - Added `with_name()` constructor for custom mesh names.
   - Added `vertex_layout()` method to get the vertex buffer layout.
   - Renamed `create_buffers()` to `create_resource()` (returns `MeshResource`).
5. ✅ **Updated `load_gltf`**: Now returns `MeshAsset` with a name derived from the file path.
6. ✅ **Updated Binaries**: Modified `forward.rs`, `deferred.rs`, and `deferred_with_camera_controls.rs` to use `MeshAsset`.
7. ✅ **Removed Deprecated Code**: Removed the deprecated `Mesh` type as it was unused.

---

## Phase 2 Implementation Summary

### Completed Tasks
1. ✅ **Added `MeshSource` enum**: Supports loading from `Path` or `Primitive` type.
2. ✅ **Added `PrimitiveType` enum**: Supports `Cube`, `Sphere`, and `Quad` primitives.
3. ✅ **Implemented `Hash` for `PrimitiveType`**: Enables deduplication in the cache.
4. ✅ **Added `MeshCache` struct**: Central cache with the following methods:
   - `new(device)`: Create a new cache.
   - `load(source)`: Load a mesh from a source (returns `MeshHandle`).
   - `get_asset(handle)`: Get the CPU asset.
   - `get_resource(handle)`: Get the GPU resource.
   - `get_both(handle)`: Get both CPU and GPU data.
   - `contains(handle)`: Check if a handle is valid.
   - `remove(handle)`: Remove a mesh from the cache.
   - `clear()`: Clear all meshes.
   - `len()` / `is_empty()`: Cache statistics.
5. ✅ **Added internal `generate_handle` method**: Uses hashing for deduplication.

### Files Modified
- `renderlib/src/mesh.rs`: Added `MeshSource`, `PrimitiveType`, `MeshCache`, and related implementations.

### Verification
- ✅ `cargo check` passes
- ✅ `cargo test` passes
- ✅ All bins compile successfully

---

## Phase 1 Implementation Summary
- `renderlib/src/mesh.rs`: Core types and backward compatibility.
- `renderlib/src/bin/forward.rs`: Updated to use `MeshAsset`.
- `renderlib/src/bin/deferred.rs`: Updated to use `MeshAsset`.
- `renderlib/src/bin/deferred_with_camera_controls.rs`: Updated to use `MeshAsset`.

### Verification
- ✅ `cargo check` passes with only deprecation warnings for old `Mesh` usage.
- ✅ `cargo test` passes.
- ✅ `cargo build --bins` succeeds.

---

## Overview
This plan outlines the implementation of a decoupled mesh resource system for `renderlib`, separating CPU-side mesh data (`MeshAsset`) from GPU-side resources (`MeshResource`) while maintaining a unified cache interface for simplicity.

---

## Goals
1. **Decouple CPU/GPU Representations**: Separate mesh data (CPU) from GPU buffers to enable:
   - CPU-side operations (e.g., physics, culling) without GPU overhead.
   - Thread-safe loading (CPU data can be loaded in background threads).
   - Memory efficiency (avoid redundant CPU data for GPU-only use cases).

2. **Unified Cache Interface**: Use a single `MeshCache` to manage both CPU assets and GPU resources, with opaque `MeshHandle` references for users.

3. **Backward Compatibility**: Ensure existing renderers (e.g., `DeferredRenderer`) can migrate with minimal changes.

---

## Non-Goals
- Full multi-threaded rendering (out of scope for this refactor).
- Advanced features like LOD or collision meshes (future extensions).
- Replacing `wgpu` buffer management (we still use `wgpu::Buffer`).

---

## Phase 1: Core Types (CPU/GPU Decoupling)
**Location**: `renderlib/src/mesh.rs`
**Estimated Effort**: 2-3 hours

### Tasks
1. **Rename existing `Mesh` to `MeshAsset`**:
   - Update struct name and all references in `mesh.rs`.
   - Keep all existing fields (`vertices`, `indices`, `bounding_box`, etc.).
   - Add `name: String` field for debugging.

2. **Add `MeshHandle`**:
   - Opaque identifier for mesh references (e.g., `pub struct MeshHandle(u64)`).
   - Use a simple atomic counter for ID generation (or `id-allocator` crate if available).

3. **Add `MeshResource`**:
   - GPU-only struct with fields:
     ```rust
     pub struct MeshResource {
         pub vertex_buffer: wgpu::Buffer,
         pub index_buffer: wgpu::Buffer,
         pub num_indices: u32,
         pub vertex_layout: wgpu::VertexBufferLayout<'static>,
     }
     ```
   - No reference to `MeshAsset` (fully decoupled).

4. **Update `MeshAsset` methods**:
   - Rename `create_buffers` to `create_resource` (returns `MeshResource` instead of raw buffers).
   - Add `vertex_layout()` method to `MeshAsset` for pipeline compatibility.

### Files to Modify
- `renderlib/src/mesh.rs` (primary changes).

---

## Phase 2: Unified Mesh Cache
**Location**: `renderlib/src/mesh.rs`
**Estimated Effort**: 3-4 hours

### Tasks
1. **Add `MeshCache` struct**:
   ```rust
   pub struct MeshCache {
       device: wgpu::Device,
       cpu_assets: HashMap<MeshHandle, Arc<MeshAsset>>,
       gpu_resources: HashMap<MeshHandle, Arc<MeshResource>>,
       next_handle: AtomicU64,  // For generating MeshHandle IDs
   }
   ```

2. **Implement `MeshCache` methods**:
   - `new(device: &wgpu::Device) -> Self`: Initialize cache with device.
   - `load(&mut self, path: &str) -> Result<MeshHandle, MeshLoadError>`:
     - Load `MeshAsset` from GLTF/primitive.
     - Create `MeshResource` from the asset.
     - Store both in caches and return `MeshHandle`.
   - `get_asset(&self, handle: MeshHandle) -> Option<Arc<MeshAsset>>`: Retrieve CPU data.
   - `get_resource(&self, handle: MeshHandle) -> Option<Arc<MeshResource>>`: Retrieve GPU data.
   - `clear(&mut self)`: Drop all GPU resources (e.g., on device loss).

3. **Add primitive support**:
   - Extend `load` to accept a `MeshSource` enum:
     ```rust
     pub enum MeshSource {
         Path(String),
         Primitive(PrimitiveType),
     }
     ```
   - Support `PrimitiveType::Cube`, `PrimitiveType::Sphere`, etc.

### Files to Modify
- `renderlib/src/mesh.rs` (add `MeshCache`).

---

## Phase 3: Integration with GraphicsContext
**Location**: `renderlib/src/context.rs`
**Estimated Effort**: 1-2 hours

### Tasks
1. **Add `MeshCache` to `GraphicsContext`**:
   ```rust
   pub struct GraphicsContext {
       pub device: wgpu::Device,
       pub queue: wgpu::Queue,
       pub size: PhysicalSize<u32>,
       pub mesh_cache: MeshCache,  // NEW
       // ... other fields
   }
   ```

2. **Initialize `MeshCache` in `GraphicsContext::new`**:
   - Pass `device` to `MeshCache::new`.

### Files to Modify
- `renderlib/src/context.rs` (add `mesh_cache` field and initialization).

---

## Phase 4: Migrate Renderers
**Location**: `renderlib/src/bin/*.rs` (e.g., `deferred.rs`, `forward.rs`)
**Estimated Effort**: 2-3 hours per renderer

### Tasks
1. **Update `DeferredRenderer`**:
   - Replace `load_gltf` + manual buffer creation with `mesh_cache.load`.
   - Store `MeshHandle` instead of raw buffers.
   - Use `mesh_cache.get_resource` in `render` method.
   - Example:
     ```rust
     // In init:
     let mesh_handle = context.mesh_cache.load("assets/duck.glb").await?;
     
     // In render:
     let mesh_resource = context.mesh_cache.get_resource(mesh_handle).unwrap();
     render_pass.set_vertex_buffer(0, mesh_resource.vertex_buffer.slice(..));
     render_pass.set_index_buffer(mesh_resource.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
     render_pass.draw_indexed(0..mesh_resource.num_indices, 0, 0..1);
     ```

2. **Update `ForwardRenderer`**:
   - Same changes as `DeferredRenderer`.

3. **Update `DeferredWithCameraControlsRenderer`**:
   - Same changes as above.

### Files to Modify
- `renderlib/src/bin/deferred.rs`
- `renderlib/src/bin/forward.rs`
- `renderlib/src/bin/deferred_with_camera_controls.rs`

---

## Phase 5: Cleanup and Validation
**Estimated Effort**: 2-3 hours

### Tasks
1. **Remove redundant code**:
   - Delete old `load_gltf` usage in renderers.
   - Remove manual buffer creation for meshes.

2. **Update exports**:
   - Ensure `MeshHandle`, `MeshAsset`, `MeshResource`, and `MeshCache` are exported in `renderlib/src/lib.rs`.

3. **Add tests**:
   - Test `MeshCache::load` with GLTF files.
   - Test `MeshCache::load` with primitives.
   - Test `get_asset` and `get_resource` methods.

4. **Validation**:
   - Run all existing examples (`deferred`, `forward`, etc.) to ensure no regressions.
   - Verify memory usage (e.g., no duplicate CPU data for shared meshes).

---

## Phase 6: Documentation
**Estimated Effort**: 1-2 hours

### Tasks
1. **Update module docs**:
   - Add high-level overview of the mesh system in `mesh.rs`.
   - Document `MeshAsset`, `MeshResource`, `MeshHandle`, and `MeshCache`.

2. **Add examples**:
   - Create a new example (`mesh_cache_example.rs`) demonstrating:
     - Loading a mesh via `MeshCache`.
     - Accessing CPU/GPU data independently.
     - Sharing meshes across multiple instances.

---

## Timeline
| Phase | Estimated Time | Priority |
|-------|----------------|----------|
| Phase 1: Core Types | 2-3 hours | High |
| Phase 2: Unified Cache | 3-4 hours | High |
| Phase 3: GraphicsContext Integration | 1-2 hours | High |
| Phase 4: Migrate Renderers | 6-9 hours | High |
| Phase 5: Cleanup and Validation | 2-3 hours | Medium |
| Phase 6: Documentation | 1-2 hours | Low |
| **Total** | **15-23 hours** | - |

---

## Risks and Mitigations
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing renderers | High | High | Incremental migration (keep old `load_gltf` as deprecated). |
| Performance regression | Low | Medium | Benchmark before/after (e.g., frame time, memory usage). |
| Thread safety issues | Medium | High | Use `Arc` and `Mutex` where needed; document thread safety guarantees. |
| Memory leaks | Medium | Medium | Use `Arc` for shared ownership; test with `cargo test -- --nocapture`. |
| Device loss handling | Low | Medium | Add `MeshCache::clear` and call it on device loss. |

---

## Success Criteria
1. All existing examples run without errors.
2. `MeshCache` can load and cache meshes from GLTF files and primitives.
3. CPU/GPU data can be accessed independently via `MeshHandle`.
4. No memory leaks (verified with `cargo test`).
5. Documentation is complete and accurate.

---

## Future Extensions (Out of Scope)
1. **Multi-threaded loading**: Use `tokio` or `rayon` for background mesh loading.
2. **LOD Support**: Add `LodMesh` to `MeshAsset` for level-of-detail rendering.
3. **Collision Meshes**: Add simplified meshes for physics.
4. **Hot-Reloading**: Watch for file changes and reload meshes dynamically.
5. **Custom Vertex Layouts**: Support multiple `MeshResource` variants for the same `MeshAsset`.

---

## Appendix: File Changes Summary
| File | Action | Notes |
|------|--------|-------|
| `renderlib/src/mesh.rs` | Modify | Rename `Mesh` to `MeshAsset`, add `MeshHandle`, `MeshResource`, `MeshCache`. |
| `renderlib/src/context.rs` | Modify | Add `mesh_cache` field to `GraphicsContext`. |
| `renderlib/src/lib.rs` | Modify | Export new types. |
| `renderlib/src/bin/deferred.rs` | Modify | Migrate to `MeshCache`. |
| `renderlib/src/bin/forward.rs` | Modify | Migrate to `MeshCache`. |
| `renderlib/src/bin/deferred_with_camera_controls.rs` | Modify | Migrate to `MeshCache`. |
| `renderlib/docs/mesh_resource_refactor_plan.md` | Create | This file. |
