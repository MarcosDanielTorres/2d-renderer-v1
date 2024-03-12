struct VertexInput {
    @location(0) position: vec3<f32>,
    //@location(1) color: vec3<f32>,
    //@location(2) tex_coords: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    //@location(1) color: vec3<f32>,
    //@location(0) tex_coords: vec2<f32>,
    @location(0) tex_coords: vec2<f32>,
};


@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    //out.color = model.color;
    out.tex_coords = model.tex_coords;
    out.clip_position = model_mat4 * vec4<f32>(model.position, 1.0);
    return out;
}

// fragment
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
// fragment
@group(0) @binding(1)
var s_diffuse: sampler;
// fragment
@group(1) @binding(0)
// TODO: Document better
// For some reason I need to tag color with 'uniform' as opposed to the fucking textures
var<uniform> color: vec4<f32>;

// vertex - fragment
@group(1) @binding(1)
var<uniform> model_mat4: mat4x4<f32>;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //return textureSample(t_diffuse, s_diffuse, in.tex_coords) * vec4<f32>(in.color.xyz, 1.0);
    return textureSample(t_diffuse, s_diffuse, in.tex_coords) * color;
}

// TODO: watch Cherno videos on Textures
    // TODO: To make a red quad, do I need to add a red image to the texture or I can just draw the fucking quad
    // with no texture attached. Yes, I can just draw the fucking quad with no texture just colored vertices
// TODO: explain location



/* 
    TODOS:
        set color as uniform
        set default texture as white
            create a white texture (width, height) 
                set_data(data, size) or Texture::from_bytes
                Queue:write_texture?
                format? rgba8?
                gl_texture something 
                gl_texturesubimage2d(0, 0, width, height)
                cherno creates a texture of size 1,1

                when draw_quad(color) texture is white
                when draw_quad(texture) color is white i could have a TextureDescriptor {
                    texture: White
                    color: White
                }
                and have them both be the same fucking function

        send scale transformation matrix
        set projection or view matrix or both
        fucking use indexes
*/
