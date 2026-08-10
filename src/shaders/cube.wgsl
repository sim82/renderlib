struct Uniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    light_pos: vec3<f32>,
    _padding: f32,
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
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.mvp * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    let normal_matrix = mat3x3<f32>(
        uniforms.model[0].xyz,
        uniforms.model[1].xyz,
        uniforms.model[2].xyz
    );
    out.normal = normal_matrix * model.normal;
    out.world_pos = (uniforms.model * vec4<f32>(model.position, 1.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate lighting
    let light_dir = normalize(uniforms.light_pos - in.world_pos);
    let normal = normalize(in.normal);

    // Diffuse lighting factor (dot product of normal and light direction)
    let diffuse = max(dot(normal, light_dir), 0.0);

    // Ambient + diffuse lighting
    let ambient = 0.1;
    let lighting = ambient + diffuse * 0.8;

    // Apply lighting to the face color
    return vec4<f32>(in.color * lighting, 1.0);
}
