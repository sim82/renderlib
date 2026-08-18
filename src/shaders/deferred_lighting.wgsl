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

// Maximum number of lights - must match Rust MAX_LIGHTS constant
const MAX_LIGHTS: u32 = 32;

struct Light {
    position: vec4<f32>,
    color: vec4<f32>,
};

struct LightingUniforms {
    view_position: vec4<f32>,
    num_lights: u32,
    _padding: vec2<f32>,
    lights: array<Light, MAX_LIGHTS>,
};

@group(1) @binding(0)
var<uniform> lighting_uniforms: LightingUniforms;

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
    // Sample G-buffer (flip Y to account for WGPU texture coordinate system)
    let uv_flipped = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    let position = textureSample(gbuffer_position, sampler_linear, uv_flipped).xyz;
    let normal = textureSample(gbuffer_normal, sampler_linear, uv_flipped).xyz;
    let albedo = textureSample(gbuffer_albedo, sampler_linear, uv_flipped).xyz;

    // Normalize sampled normal
    let n = normalize(normal);
    
    let view_dir = normalize(lighting_uniforms.view_position.xyz - position);
    
    // Start with ambient lighting
    let ambient = 0.1;
    var lighting = vec3<f32>(ambient, ambient, ambient);
    
    // Accumulate lighting from all active light sources
    for (var i: u32 = 0; i < lighting_uniforms.num_lights; i++) {
        let light = lighting_uniforms.lights[i];
        let light_dir = normalize(light.position.xyz - position);
        
        // Diffuse lighting
        let diffuse = max(dot(n, light_dir), 0.0);
        
        // Simple specular (Blinn-Phong)
        let half_vec = normalize(light_dir + view_dir);
        let specular = pow(max(dot(n, half_vec), 0.0), 32.0);
        
        // Add diffuse and specular lighting with light color
        lighting += (diffuse * 0.8 + specular * 0.5) * light.color.rgb;
    }
    
    // Clamp lighting to prevent overexposure
    lighting = clamp(lighting, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(2.0, 2.0, 2.0));
    
    // Apply lighting to albedo
    return vec4<f32>(albedo * lighting, 1.0);
}
