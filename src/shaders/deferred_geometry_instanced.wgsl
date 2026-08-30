// Deferred rendering geometry pass shader with instancing
// Outputs position, normal, and albedo to G-buffer
// Uses instanced rendering for efficient multi-mesh rendering

// Maximum number of instances
const MAX_INSTANCES: u32 = 1024;

// Camera uniforms (shared, binding 0)
struct CameraUniforms {
    view_proj: mat4x4<f32>,
};

// Instance uniforms (per-instance, binding 1)
struct InstanceUniforms {
    model: mat4x4<f32>,
};

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
var<uniform> camera: CameraUniforms;

@group(0) @binding(1)
var<uniform> instances: array<InstanceUniforms, MAX_INSTANCES>;

@vertex
fn vs_main(
    model: VertexInput,
    @location(3) instance_index: u32,
) -> VertexOutput {
    let instance = instances[instance_index];

    var out: VertexOutput;
    let mvp = camera.view_proj * instance.model;
    out.clip_position = mvp * vec4<f32>(model.position, 1.0);
    out.world_position = (instance.model * vec4<f32>(model.position, 1.0)).xyz;

    // Transform normal by model matrix (upper 3x3)
    let normal_matrix = mat3x3<f32>(
        instance.model[0].xyz,
        instance.model[1].xyz,
        instance.model[2].xyz
    );
    out.world_normal = normalize(normal_matrix * model.normal);
    out.albedo = model.color;
    return out;
}

// Fragment output struct with multiple locations
struct FragmentOutput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) albedo: vec4<f32>,
};

@group(1)
@binding(0)
var position_texture: texture_2d<f32>;
@group(1)
@binding(1)
var normal_texture: texture_2d<f32>;
@group(1)
@binding(2)
var albedo_texture: texture_2d<f32>;
@group(1)
@binding(3)
var gbuffer_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.position = vec4<f32>(in.world_position, 1.0);
    out.normal = vec4<f32>(normalize(in.world_normal), 1.0);
    out.albedo = vec4<f32>(in.albedo, 1.0);
    return out;
}
