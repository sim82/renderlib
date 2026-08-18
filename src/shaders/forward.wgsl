struct GeometryUniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
};

struct Light {
    position: vec4<f32>,  // xyz = position, w = radius
    color: vec4<f32>,
};

// Maximum number of lights - must match Rust MAX_LIGHTS constant
const MAX_LIGHTS: u32 = 32;

struct LightingUniforms {
    view_position: vec4<f32>,
    view_projection: mat4x4<f32>,
    num_lights: u32,
    // 12 bytes padding
    lights: array<Light, MAX_LIGHTS>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> geometry_uniforms: GeometryUniforms;

@group(0) @binding(1)
var<uniform> lighting_uniforms: LightingUniforms;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = geometry_uniforms.mvp * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    let normal_matrix = mat3x3<f32>(
        geometry_uniforms.model[0].xyz,
        geometry_uniforms.model[1].xyz,
        geometry_uniforms.model[2].xyz
    );
    out.normal = normal_matrix * model.normal;
    out.world_pos = (geometry_uniforms.model * vec4<f32>(model.position, 1.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);

    // Start with ambient lighting
    let ambient = 0.1;
    var lighting = vec3<f32>(ambient, ambient, ambient);

    // Accumulate lighting from all active light sources
    for (var i: u32 = 0; i < lighting_uniforms.num_lights; i++) {
        let light = lighting_uniforms.lights[i];

        // Note: Screen-space culling is disabled for forward rendering
        // because it requires UV coordinates in the fragment shader.
        // The culling is implemented in deferred_lighting.wgsl where we have
        // full-screen UV from the quad vertices.

        // Calculate lighting for all lights
        let light_pos = light.position.xyz;
        let light_radius = light.position.w;
        
        let light_dir = normalize(light_pos - in.world_pos);
        let distance = length(light_pos - in.world_pos);

        // Polynomial falloff: 1.0 at distance=0, 0.0 at distance=light_radius
        // This gives a smooth quadratic transition where light contribution falls to zero
        // exactly at the specified radius
        let t = distance / light_radius;
        let falloff = max(1.0 - t * t, 0.0);

        // Diffuse lighting factor (dot product of normal and light direction)
        let diffuse = max(dot(normal, light_dir), 0.0);

        // Add diffuse lighting with light color and falloff
        lighting += diffuse * 0.8 * light.color.rgb * falloff;
    }

    // Clamp lighting to prevent overexposure
    lighting = clamp(lighting, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));

    // Apply lighting to the face color
    return vec4<f32>(in.color * lighting, 1.0);
}
