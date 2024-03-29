struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3<f32>
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = model_mat4 * vec4<f32>(model.position, 1.0);
    out.pos = model.position;
    return out;
}

// fragment
@group(0) @binding(0)
var<uniform> model_mat4: mat4x4<f32>;

@group(0) @binding(1)
var<uniform> color: vec4<f32>;

@group(0) @binding(2)
var<uniform> thickness: f32;

@group(0) @binding(3)
var<uniform> fade: f32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var d = 1.0 - length(in.pos.xyz * 2.0);
    var alpha = smoothstep(0.0, fade, d);
    alpha *= smoothstep(thickness + fade, thickness, d);

    var out_color = vec4<f32>(color.x, color.y, color.z, color.w * alpha);
    return out_color;
}