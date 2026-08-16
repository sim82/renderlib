// Deferred rendering geometry pass shader
// Outputs position, normal, and albedo to G-buffer

struct Uniforms {
    mvp: mat4x4<f32>,
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
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.mvp * vec4<f32>(model.position, 1.0);
    out.world_position = (uniforms.model * vec4<f32>(model.position, 1.0)).xyz;

    // Transform normal by model matrix (upper 3x3)
    let normal_matrix = mat3x3<f32>(
        uniforms.model[0].xyz,
        uniforms.model[1].xyz,
        uniforms.model[2].xyz
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

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    // Location 0: world position (as vec4 with w=1.0)
    out.position = vec4<f32>(in.world_position, 1.0);
    // Location 1: world normal (as vec4 with w=0.0 for packing)
    out.normal = vec4<f32>(in.world_normal, 0.0);
    // Location 2: albedo (as vec4 with w=1.0)
    out.albedo = vec4<f32>(in.albedo, 1.0);
    return out;
}
