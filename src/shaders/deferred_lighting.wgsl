// Deferred rendering lighting pass shader
// Reads from G-buffer and computes lighting

@group(0) @binding(0)
var gbuffer_position: texture_2d<f32>;
@group(0) @binding(1)
var gbuffer_normal: texture_2d<f32>;
@group(0) @binding(2)
var gbuffer_albedo: texture_2d<f32>;

@group(0) @binding(3)
var sampler_linear: sampler;

@group(1) @binding(0)
var<uniform> lighting_uniforms: LightingUniforms;

struct LightingUniforms {
    view_position: vec4<f32>,
    light_position: vec4<f32>,
};

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // Full-screen quad: position is in normalized device coordinates
    out.clip_position = vec4<f32>(model.position * 2.0 - 1.0, 0.0, 1.0);
    out.uv = model.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample G-buffer
    let position = textureSample(gbuffer_position, sampler_linear, in.uv).xyz;
    let normal = textureSample(gbuffer_normal, sampler_linear, in.uv).xyz;
    let albedo = textureSample(gbuffer_albedo, sampler_linear, in.uv).xyz;
    
    // Normalize sampled normal
    let n = normalize(normal);
    
    // Calculate lighting
    let light_dir = normalize(lighting_uniforms.light_position.xyz - position);
    let view_dir = normalize(lighting_uniforms.view_position.xyz - position);
    
    // Diffuse lighting
    let diffuse = max(dot(n, light_dir), 0.0);
    
    // Simple specular (Blinn-Phong)
    let half_vec = normalize(light_dir + view_dir);
    let specular = pow(max(dot(n, half_vec), 0.0), 32.0);
    
    // Ambient + diffuse + specular
    let ambient = 0.1;
    let lighting = ambient + diffuse * 0.8 + specular * 0.5;
    
    // Apply lighting to albedo
    return vec4<f32>(albedo * lighting, 1.0);
}
