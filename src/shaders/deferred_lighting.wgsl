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
    position: vec4<f32>,  // xyz = position, w = radius
    color: vec4<f32>,
};

struct LightingUniforms {
    view_position: vec4<f32>,
    view_projection: mat4x4<f32>,
    num_lights: u32,
    // 12 bytes padding
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
    // with screen-space culling optimization
    for (var i: u32 = 0; i < lighting_uniforms.num_lights; i++) {
        let light = lighting_uniforms.lights[i];

        // Extract light position (xyz) and radius (w)
        let light_pos = light.position.xyz;
        let light_radius = light.position.w;

        // Project light position to clip space
        let light_clip = lighting_uniforms.view_projection * vec4<f32>(light_pos, 1.0);

        // Perspective divide
        let light_ndc = light_clip.xyz / light_clip.w;

        // Convert to screen UV coordinates (0-1 range)
        // Note: NDC is [-1,1] so we remap to [0,1]
        let light_screen_uv = vec2<f32>(
            (light_ndc.x + 1.0) * 0.5,
            (light_ndc.y + 1.0) * 0.5
        );

        // Calculate approximate screen-space radius
        // Scale world-space radius by estimated screen scale
        // This is an approximation - for accurate culling, we'd need to
        // project the light's bounding sphere to screen space
        let light_to_camera = distance(lighting_uniforms.view_position.xyz, light_pos);
        let screen_radius = light_radius / max(light_to_camera, 0.1);

        // Skip if pixel is outside light's screen-space influence area
        // Use squared distance for efficiency (avoid sqrt)
        let pixel_to_light_diff = in.uv - light_screen_uv;
        let pixel_to_light_dist_sq = dot(pixel_to_light_diff, pixel_to_light_diff);
        if (pixel_to_light_dist_sq > screen_radius * screen_radius) {
            continue;
        }

        // Light passes screen-space culling - calculate lighting
        let light_dir = normalize(light_pos - position);
        let distance = length(light_pos - position);

        // Polynomial falloff: 1.0 at distance=0, 0.0 at distance=light_radius
        // This gives a smooth quadratic transition where light contribution falls to zero
        // exactly at the specified radius
        let t = distance / light_radius;
        let falloff = max(1.0 - t * t, 0.0);

        // Diffuse lighting
        let diffuse = max(dot(n, light_dir), 0.0);

        // Simple specular (Blinn-Phong)
        let half_vec = normalize(light_dir + view_dir);
        let specular = pow(max(dot(n, half_vec), 0.0), 32.0);

        // Add diffuse and specular lighting with light color and falloff
        lighting += (diffuse * 0.8 + specular * 0.5) * light.color.rgb * falloff;
    }

    // Clamp lighting to prevent overexposure
    lighting = clamp(lighting, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(2.0, 2.0, 2.0));

    // Apply lighting to albedo
    return vec4<f32>(albedo * lighting, 1.0);
}
