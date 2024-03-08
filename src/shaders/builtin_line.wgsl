struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    if (vertex_index == u32(0)) {
        out.clip_position = model_mat4_first_vertex * vec4<f32>(model.position, 1.0);
    } else {
        out.clip_position = model_mat4_second_vertex * vec4<f32>(model.position, 1.0);
    }
    return out;
}

// fragment
@group(0) @binding(0)
var<uniform> color: vec4<f32>;
// fragment
@group(0) @binding(1)
var<uniform> model_mat4_first_vertex: mat4x4<f32>;
@group(0) @binding(2)
var<uniform> model_mat4_second_vertex: mat4x4<f32>;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.clip_position * color;
}

/*
The reason why I choose to have the color as a uniform is because i thought about changing it and had the idea
that because the vertices weren't changing those weren't uniforms. But know, there is the problem with the lines...

Maybe I can just do the exact same logic and position the lines using the MVP, which I think is the correct approach.
    - I think this is the solution. basically draw_line(origin, dest, color) => origin = pos, dest = scale.
      it looks weird though... should roll with this for now, we'll later check some resources. Solve the problem.


But this begs the question: can I modify the vertex buffers with a call to `queue.write_buffer()`?
*/

