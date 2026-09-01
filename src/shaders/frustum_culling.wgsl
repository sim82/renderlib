// Frustum culling compute shader
// Performs the same culling logic as the CPU version in multi_mesh_instanced.rs
// but in parallel on the GPU.

struct InstanceData {
    world_center: vec3<f32>,
    world_radius: f32,
};

@group(0) @binding(0)
var<storage, read> instances: array<InstanceData>;

@group(0) @binding(1)
var<uniform> view_matrix: mat4x4<f32>;

@group(0) @binding(2)
var<uniform> camera_params: vec4<f32>; // [near, far, tan_fov_x, tan_fov_y]

@group(0) @binding(3)
var<storage, read_write> visible_indices: array<u32>;

@group(0) @binding(4)
var<storage, read_write> atomic_counter: atomic<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&instances)) {
        return;
    }

    let instance = instances[idx];

    // Transform the sphere center to view space
    let center_view = view_matrix * vec4<f32>(instance.world_center, 1.0);

    // In view space:
    // - Camera is at origin (0,0,0)
    // - Camera looks down negative Z axis
    // - Objects with z > 0 are BEHIND the camera
    // - Objects with z < 0 are IN FRONT of the camera

    // Check if sphere is in front of camera (not completely behind)
    // A sphere is in front if its closest point to camera is in front:
    // center_view.z - world_radius <= 0.0
    // But to reduce popping, we use a conservative test: only cull if the sphere
    // is COMPLETELY behind the camera (center - radius > 0)
    let completely_behind_camera = center_view.z - instance.world_radius > 0.0;

    // Check if sphere is too far away (completely beyond far plane)
    // In view space, far plane is at z = -camera.far
    let completely_beyond_far = center_view.z + instance.world_radius < -camera_params.y;

    // Check if sphere is too close (completely before near plane)
    // In view space, near plane is at z = -camera.near
    // Only cull if the sphere is COMPLETELY before the near plane
    let completely_before_near = center_view.z - instance.world_radius > -camera_params.x;

    // If the sphere is completely outside the view frustum, skip it
    if (completely_behind_camera || completely_beyond_far || completely_before_near) {
        return;
    }

    // Now check angular bounds in view space
    // The frustum in view space is a pyramid with:
    // - Left/right planes based on horizontal FOV
    // - Top/bottom planes based on vertical FOV
    // - Near/far planes (already checked)

    // In view space, at distance |z| from camera:
    // - x must be within [-|z| * tan_fov_x, |z| * tan_fov_x]
    // - y must be within [-|z| * tan_fov_y, |z| * tan_fov_y]
    // But z is negative in view space (camera looks down -Z)

    let z_abs = abs(-center_view.z);
    let x_bound = z_abs * camera_params.z; // tan_fov_x
    let y_bound = z_abs * camera_params.w; // tan_fov_y

    // Check if sphere overlaps with the frustum in x
    let inside_x = (center_view.x + instance.world_radius >= -x_bound) &&
                  (center_view.x - instance.world_radius <= x_bound);

    // Check if sphere overlaps with the frustum in y
    let inside_y = (center_view.y + instance.world_radius >= -y_bound) &&
                  (center_view.y - instance.world_radius <= y_bound);

    if (inside_x && inside_y) {
        // Atomically increment counter and write index
        let visible_idx = atomicAdd(&atomic_counter, 1u);
        visible_indices[visible_idx] = idx;
    }
}
