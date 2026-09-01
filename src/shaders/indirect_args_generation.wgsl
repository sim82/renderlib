// Indirect args generation compute shader
// Reads visible indices and atomic counter from frustum culling pass
// Generates indirect draw arguments for the geometry pass

@group(0) @binding(0)
var<storage, read> visible_indices: array<u32>;

@group(0) @binding(1)
var<storage, read> atomic_counter: atomic<u32>;

@group(0) @binding(2)
var<storage, read_write> visible_count: array<u32>;

@group(0) @binding(3)
var<storage, read_write> compacted_indices: array<u32>;

@group(0) @binding(4)
var<storage, read_write> indirect_draw_args: array<u32>;

@group(0) @binding(5)
var<uniform> mesh_info: vec2<u32>; // [index_count, vertex_count]

@compute @workgroup_size(1)
fn main() {
    // Read atomic counter to get visible count
    let visible_count_val = atomicLoad(&atomic_counter);
    
    // Copy visible count to dedicated buffer
    visible_count[0] = visible_count_val;
    
    // Copy visible indices to compacted buffer (first 'visible_count_val' elements)
    for (var i: u32 = 0; i < visible_count_val; i++) {
        compacted_indices[i] = visible_indices[i];
    }
    
    // Generate indirect draw arguments
    // Structure for DrawIndexedIndirect: [index_count, instance_count, first_index, base_vertex, first_instance]
    // Note: base_vertex is i32, but we store it as u32 and cast in the geometry shader if needed
    let index_count = mesh_info.x;    // Number of indices in the mesh
    let instance_count = visible_count_val;
    let first_index = 0u;
    let base_vertex = 0u; // Will be cast to i32 as needed
    let first_instance = 0u;
    
    indirect_draw_args[0] = index_count;    // index_count (called vertex_count in some contexts)
    indirect_draw_args[1] = instance_count;  // instance_count
    indirect_draw_args[2] = first_index;     // first_index
    indirect_draw_args[3] = base_vertex;     // base_vertex (stored as u32, will be reinterpreted)
    indirect_draw_args[4] = first_instance;  // first_instance
}