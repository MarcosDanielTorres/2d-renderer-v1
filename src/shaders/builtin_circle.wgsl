@group(0)
@binding(0)
var<uniform> u_circle: Circle;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct Circle {
    model_mat: mat4x4<f32>,
    color: vec4<f32>,
    thickness: f32,
    fade: f32,
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
    out.clip_position = u_circle.model_mat * vec4<f32>(model.position, 1.0);
    out.pos = model.position;
    return out;
}

// fragment

//@group(0) @binding(0)
//var<uniform> model_mat4: mat4x4<f32>;

//@group(0) @binding(1)
//var<uniform> color: vec4<f32>;

//@group(0) @binding(2)
//var<uniform> thickness: f32;

//@group(0) @binding(3)
//var<uniform> fade: f32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var d = 1.0 - length(in.pos.xyz * 2.0);
    var alpha = smoothstep(0.0, u_circle.fade, d);
    alpha *= smoothstep(u_circle.thickness + u_circle.fade, u_circle.thickness, d);

    var out_color = vec4<f32>(u_circle.color.x, u_circle.color.y, u_circle.color.z, u_circle.color.w * alpha);
    return out_color;
}