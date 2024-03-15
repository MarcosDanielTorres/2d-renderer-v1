use glam::*;

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use wgpu::RenderPass;

mod gui;


use winit::event::KeyEvent;
use winit::keyboard::KeyCode;

use winit::event::DeviceEvent;
use winit::event::ElementState;
use winit::{
    event::{Event, WindowEvent},
    event_loop::*,
    keyboard::PhysicalKey,
    window::WindowBuilder,
};

mod context;
mod pipeline;
mod texture;
use texture::Texture;

use crate::context::AppContext;

use crate::gui::Framework;
use crate::pipeline::RenderPipelineBuilder;
use crate::pipeline::VertexDescriptor;

use wgpu::util::{align_to, BufferInitDescriptor, DeviceExt};

pub trait Application {
    fn on_setup(&mut self, engine: &mut Engine);
    fn on_update(&mut self, engine: &mut Engine);
    fn on_render(&mut self, engine: &mut Engine);
    fn on_event(&mut self, engine: &mut Engine, event: MyEvent);
}

struct TransformComponent {
    position: Mat4,
    scale: Mat4,
    rotation: Mat4,
}

struct LineComponent {
    orig: Mat4,
    dest: Mat4,
}

// START CIRCLE
// CIRCLE

struct CircleInfo {
    transform: TransformComponent,
    color: [f32; 4],
    thickness: f32,
    fade: f32,
    // Radius is not needed because it is in the scale matrix. If scale is 1, then radius is 1.
}


// One pipeline for each type because they have different vertex and fragment shaders.
struct CirclePipeline {
    // Data to render
    circle_info : Vec<CircleInfo>,

    // Pipeline
    render_pipeline: wgpu::RenderPipeline,

    // Vertex - same as quad. Because it is a quad modified in the fragment shader.
    vertex_buffer: wgpu::Buffer,

    // Uniforms
    model_mat4_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    thickness_buffer: wgpu::Buffer,
    fade_buffer: wgpu::Buffer,
}

impl CirclePipeline {
    pub fn new(app_context: Arc<AppContext>) -> Self {
        // vertex
        #[repr(C)]
        #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        struct Vertex {
            position: [f32; 3],
        }
        impl<'a> VertexDescriptor<'a> for Vertex {
            fn desc() -> wgpu::VertexBufferLayout<'a> {
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                }
            }
        }

        const VERTICES: &[Vertex] = &[
            // Position of quad at the center
            Vertex {
                position: [0.5, 0.5, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.0],
            },
            // Position of quad at the center
        ];


        let vertex_buffer =
            app_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: wgpu::BufferUsages::VERTEX,
                });


        // This `transform_uniform_size` has the same name in all other parts but it should be something related to MAT4
        let transform_uniform_size = std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress;
        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        let thickness_uniform_size = std::mem::size_of::<f32>() as wgpu::BufferAddress;
        let fade_uniform_size = std::mem::size_of::<f32>() as wgpu::BufferAddress;

        let bind_group_layout =
            app_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Circle - BindGroupLayout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(transform_uniform_size),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(color_uniform_size),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(thickness_uniform_size),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(fade_uniform_size),
                            },
                            count: None,
                        },
                    ],
                });



        // BUFFER CREATION

        // alignments
        let transform_uniform_alignment = {
            let alignment = app_context
                .device
                .limits()
                .min_uniform_buffer_offset_alignment
                as wgpu::BufferAddress;
            align_to(transform_uniform_size, alignment)
        };


        let thickness_uniform_alignment = {
            let alignment = app_context
                .device
                .limits()
                .min_uniform_buffer_offset_alignment
                as wgpu::BufferAddress;
            align_to(thickness_uniform_size, alignment)
        };

        // this `alignemnt` can be calculated once
        let fade_uniform_alignment = {
            let alignment = app_context
                .device
                .limits()
                .min_uniform_buffer_offset_alignment
                as wgpu::BufferAddress;
            align_to(fade_uniform_size, alignment)
        };

        // buffers
        let model_mat4_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Circle - MAT4 Buffer"),
            size: 40 * transform_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Color doesn't use alignment because I'm initializing it with WHITE.
        let color_slice: [f32; 400] = [1.0; 400];
        let color_buffer = app_context
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: Some("Circle - Color Buffer"),
                contents: bytemuck::cast_slice(&color_slice),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let thickness_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Circle - Thickness Buffer"),
            size: 40 * thickness_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let fade_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Circle - Fade Buffer"),
            size: 40 * thickness_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });


        let bind_group = app_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Circle - BindGroup"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &model_mat4_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(color_uniform_size),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &color_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(transform_uniform_size),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &thickness_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(thickness_uniform_size),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &fade_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(fade_uniform_size),
                        }),
                    },
                ],
            });


        let module = wgpu::ShaderModuleDescriptor {
            label: Some("Builtin Circle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/builtin_circle.wgsl").into()),
        };

        let render_pipeline = RenderPipelineBuilder::new()
            .add_vertex_buffer_layout::<Vertex>()
            .add_color_target_state(wgpu::ColorTargetState {
                format: app_context.config.format,
                write_mask: wgpu::ColorWrites::ALL,
            })
            .shader(module)
            // .with_wireframe(true)
            .pipeline_layout_descriptor(
                "Vertex layout descriptor",
                &[&bind_group_layout],
                &[],
            )
            .build(&app_context.device, "Vertex Pipeline", "vs_main", "fs_main");

        Self {
            circle_info: vec![],
            render_pipeline,
            vertex_buffer,
            model_mat4_buffer,
            color_buffer,
            thickness_buffer,
            fade_buffer,
        }
    }
}


// START LINE
// LINE
struct LineInfo {
    // updated for every `draw_quad`
    transform: LineComponent,
    color: [f32; 4],
}

struct LinePipeline {
    // Data to render
    line_info: Vec<LineInfo>,

    // Pipeline
    render_pipeline: wgpu::RenderPipeline,

    // Vertex
    vertex_buffer: wgpu::Buffer,

    // Uniforms
    color_buffer: wgpu::Buffer,
    orig_model_mat4_buffer: wgpu::Buffer,
    dest_model_mat4_buffer: wgpu::Buffer,
    line_bind_group: wgpu::BindGroup,
}

impl LinePipeline {
    fn new(app_context: Arc<AppContext>) -> Self {
        // vertex
        #[repr(C)]
        #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        struct Vertex {
            position: [f32; 3],
        }

        impl<'a> VertexDescriptor<'a> for Vertex {
            fn desc() -> wgpu::VertexBufferLayout<'a> {
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                }
            }
        }

        // local to the model these are NDC
        const VERTICES: &[Vertex] = &[
            Vertex {
                position: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0],
            },
        ];

        let vertex_buffer =
            app_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        let transform_uniform_size = std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress;
        let texture_bind_group_layout =
            app_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(color_uniform_size),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(transform_uniform_size),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(transform_uniform_size),
                            },
                            count: None,
                        },
                    ],
                    label: Some("line_bind_group_layout"),
                });

        // Make the `uniform_alignment` >= `entity_uniform_size` and aligned to `min_uniform_buffer_offset_alignment`.
        let transform_uniform_alignment = {
            let alignment = app_context
                .device
                .limits()
                .min_uniform_buffer_offset_alignment
                as wgpu::BufferAddress;
            align_to(transform_uniform_size, alignment)
        };

        let orig_model_mat4_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line MVP Buffer"),
            size: 40 * transform_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let dest_model_mat4_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line MVP Buffer"),
            size: 40 * transform_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        // Make the `uniform_alignment` >= `entity_uniform_size` and aligned to `min_uniform_buffer_offset_alignment`.
        let color_uniform_alignment = {
            let alignment = app_context
                .device
                .limits()
                .min_uniform_buffer_offset_alignment
                as wgpu::BufferAddress;
            align_to(color_uniform_size, alignment)
        };

        let color_slice: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        let color_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line Color Buffer"),
            size: 40 * color_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let line_bind_group = app_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("line_bind_group"),
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &color_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(color_uniform_size),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &orig_model_mat4_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(transform_uniform_size),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &dest_model_mat4_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(transform_uniform_size),
                        }),
                    },
                ],
            });

        let module = wgpu::ShaderModuleDescriptor {
            label: Some("Builtin Line Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/builtin_line.wgsl").into()),
        };

        let render_pipeline = RenderPipelineBuilder::new()
            .with_topology(wgpu::PrimitiveTopology::LineList)
            .add_vertex_buffer_layout::<Vertex>()
            .add_color_target_state(wgpu::ColorTargetState {
                format: app_context.config.format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })
            .shader(module)
            .pipeline_layout_descriptor(
                "Line Vertex Layout Descriptor",
                &[&texture_bind_group_layout],
                &[],
            )
            .build(
                &app_context.device,
                "Line Vertex Pipeline",
                "vs_main",
                "fs_main",
            );

        Self {
            line_info: vec![],
            render_pipeline,
            vertex_buffer,
            color_buffer,
            orig_model_mat4_buffer,
            dest_model_mat4_buffer,
            line_bind_group,
        }
    }
}

// LINE

// QUAD
struct QuadInfo {
    // updated for every `draw_quad`
    transform: TransformComponent,
    color: [f32; 4],
    texture_name: Option<String>,
    // texture_path: Option<&'static Path>,
    // texture_path: Option<&'static [u8]>,
}

struct QuadPipeline {
    quad_info: Vec<QuadInfo>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,

    // Uniforms
    color_buffer: wgpu::Buffer,
    model_mat4_buffer: wgpu::Buffer,

    // This `wgpu::BindGroup`` holds uniform data for the quad that can change every frame
    bind_group: wgpu::BindGroup,

    // Here I don't need a `wgpu::BindGroup` as this bindgroup is associated to a texture, not a buffer.
    // So in order to keep remapping textures I need to create one `wgpu::BindGroup` for each texture
    // and remember the `wgpu::BindGroupLayout`
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl QuadPipeline {
    fn new(app_context: Arc<AppContext>) -> Self {
        // vertex
        #[repr(C)]
        #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        struct Vertex {
            position: [f32; 3],
            tex_coords: [f32; 2],
        }

        impl<'a> VertexDescriptor<'a> for Vertex {
            fn desc() -> wgpu::VertexBufferLayout<'a> {
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }
            }
        }

        // local to the model
        const VERTICES: &[Vertex] = &[
            // Position of quad at the top-right corner
            // Vertex {
            //     position: [0.0, 0.0, 0.0],
            //     tex_coords: [1.0, 0.0],
            // },
            // Vertex {
            //     position: [-1.0, 0.0, 0.0],
            //     tex_coords: [0.0, 0.0],
            // },
            // Vertex {
            //     position: [-1.0, -1.0, 0.0],
            //     tex_coords: [0.0, 1.0],
            // },
            // Vertex {
            //     position: [-1.0, -1.0, 0.0],
            //     tex_coords: [0.0, 1.0],
            // },
            // Vertex {
            //     position: [0.0, -1.0, 0.0],
            //     tex_coords: [1.0, 1.0],
            // },
            // Vertex {
            //     position: [0.0, 0.0, 0.0],
            //     tex_coords: [1.0, 0.0],
            // },
            // Position of quad at the top-right corner

            // Position of quad at the center
            Vertex {
                position: [0.5, 0.5, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.0],
                tex_coords: [1.0, 0.0],
            },
            // Position of quad at the center
        ];

        let vertex_buffer =
            app_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        // textures
        let diffuse_bytes = include_bytes!("happy-tree.png");
        let diffuse_texture = Texture::from_bytes(
            &app_context.device,
            &app_context.queue,
            diffuse_bytes,
            "Happy tree",
        )
        .unwrap();

        // let diffuse_texture = {
        //     let rgba = img.to_rgba8();
        //     let dimensions = img.dimensions();

        //     let texture_extent = wgpu::Extent3d {
        //         width: dimensions.0,
        //         height: dimensions.1,
        //         depth_or_array_layers: 1,
        //     };
        //     let texture = device.create_texture(&wgpu::TextureDescriptor {
        //         label,
        //         size: texture_extent,
        //         mip_level_count: 1,
        //         sample_count: 1,
        //         dimension: wgpu::TextureDimension::D2,
        //         format: wgpu::TextureFormat::Rgba8UnormSrgb,
        //         view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        //         usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        //     });

        //     // dimensions are 256 x 256 for `happy-tree.png`
        //     queue.write_texture(
        //         wgpu::ImageCopyTexture {
        //             aspect: wgpu::TextureAspect::All,
        //             texture: &texture,
        //             mip_level: 0,
        //             origin: wgpu::Origin3d::ZERO,
        //         },
        //         &rgba,
        //         wgpu::ImageDataLayout {
        //             offset: 0,
        //             // bytes_per_row: Some(4 * dimensions.0),
        //             // rows_per_image: Some(dimensions.1),
        //             bytes_per_row: Some(4 * dimensions.0),
        //             rows_per_image: Some(dimensions.1),
        //         },
        //         texture_extent,
        //     );

        //     let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        //     let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        //         address_mode_u: wgpu::AddressMode::ClampToEdge,
        //         address_mode_v: wgpu::AddressMode::ClampToEdge,
        //         address_mode_w: wgpu::AddressMode::ClampToEdge,
        //         mag_filter: wgpu::FilterMode::Linear,
        //         min_filter: wgpu::FilterMode::Nearest,
        //         mipmap_filter: wgpu::FilterMode::Nearest,
        //         ..Default::default()
        //     });

        //     Texture {
        //         texture,
        //         texture_view,
        //         texture_sampler,
        //         texture_extent,
        //     }
        // };

        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        let transform_uniform_size = std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress;

        let bind_group_layout =
            app_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(color_uniform_size),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(transform_uniform_size),
                            },
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        let texture_bind_group_layout =
            app_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        // Make the `uniform_alignment` >= `entity_uniform_size` and aligned to `min_uniform_buffer_offset_alignment`.
        let transform_uniform_alignment = {
            let alignment = app_context
                .device
                .limits()
                .min_uniform_buffer_offset_alignment
                as wgpu::BufferAddress;
            align_to(transform_uniform_size, alignment)
        };

        let model_mat4_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("translation Buffer"),
            size: 40 * transform_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        // Make the `uniform_alignment` >= `entity_uniform_size` and aligned to `min_uniform_buffer_offset_alignment`.
        let color_uniform_alignment = {
            let alignment = app_context
                .device
                .limits()
                .min_uniform_buffer_offset_alignment
                as wgpu::BufferAddress;
            align_to(color_uniform_size, alignment)
        };

        ////////////////////////////////////////
        /////////////// QUAD ///////////////////
        //////// DEFAULT COLOR VALUE ///////////
        ////////////////////////////////////////

        // color_uniform_alignment = 256. Its too high, thats why I thought both ways would workd
        // and it seemed they are. But this is just allocating space in the buffer for future use.
        // with default color value
        // for 40 colors actually
        let color_slice: [f32; 400] = [1.0; 400];
        let color_buffer = app_context
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: Some("Line Color Buffer"),
                contents: bytemuck::cast_slice(&color_slice),
                // size: 40 * color_uniform_alignment,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                // mapped_at_creation: false,
            });

        // with no default color value
        //
        // let color_slice: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        // let color_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
        //     label: Some("Line Color Buffer"),
        //     size: 2 * color_uniform_alignment,
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        //     mapped_at_creation: false,
        // });

        ////////////////////////////////////////
        /////////////// QUAD ///////////////////
        //////// DEFAULT COLOR VALUE ///////////
        ////////////////////////////////////////
        let bind_group = app_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bind_group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &color_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(color_uniform_size),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &model_mat4_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(transform_uniform_size),
                        }),
                    },
                ],
            });

        let texture_bind_group = app_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("texture_bind_group"),
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.texture_sampler),
                    },
                ],
            });

        let module = wgpu::ShaderModuleDescriptor {
            label: Some("Builtin Quad Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/builtin_quad.wgsl").into()),
        };

        let render_pipeline = RenderPipelineBuilder::new()
            .add_vertex_buffer_layout::<Vertex>()
            .add_color_target_state(wgpu::ColorTargetState {
                format: app_context.config.format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })
            .shader(module)
            // .with_wireframe(true)
            .pipeline_layout_descriptor(
                "Vertex layout descriptor",
                &[&texture_bind_group_layout, &bind_group_layout],
                &[],
            )
            .build(&app_context.device, "Vertex Pipeline", "vs_main", "fs_main");

        Self {
            quad_info: vec![],
            render_pipeline,
            vertex_buffer,
            color_buffer,
            model_mat4_buffer,

            // uniforms
            bind_group,

            // textures
            texture_bind_group_layout,
        }
    }

    // What the actual fuck just happened?
    // pub fn draw<'r>(&'r self, color: [f32; 4], render_pass: &'r mut wgpu::RenderPass<'r>) {
    // pub fn draw<'a, 'r: 'a>(&'r self, color: [f32; 4], mut render_pass: wgpu::RenderPass<'a>) {
    //     render_pass.set_pipeline(&self.render_pipeline);
    //     render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
    //     render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    //     render_pass.draw(0..3, 0..1);
    // }
}
// QUAD

struct BindableTexture {
    texture: Texture,
    bind_group: wgpu::BindGroup,
}

pub struct Engine {
    app_context: Arc<AppContext>,
    pub texture_map: Arc<Mutex<HashMap<String, BindableTexture>>>,
    quad_pipeline: QuadPipeline,
    line_pipeline: LinePipeline,
}

impl Engine {
    pub fn new(
        app_context: Arc<AppContext>,
        texture_map: Arc<Mutex<HashMap<String, BindableTexture>>>,
    ) -> Self {
        let quad_pipeline = QuadPipeline::new(app_context.clone());
        let line_pipeline = LinePipeline::new(app_context.clone());

        Self {
            app_context: app_context.clone(),
            texture_map,
            quad_pipeline,
            line_pipeline,
        }
    }

    pub fn create_dummy_texture_u32(&self, id: String, data: &[u8]) {
        let mut texture_map = self.texture_map.lock().unwrap();

        if !texture_map.contains_key(&id) {
            use image::GenericImageView;

            let format = wgpu::TextureFormat::Rgba8UnormSrgb;

            let pixel_size = match format.block_dimensions() {
                (1, 1) => format.block_copy_size(None).unwrap() as usize,
                _ => panic!("Using pixel_size for compressed textures is invalid"),
            };

            let texture_extent = wgpu::Extent3d {
                width: 16,
                height: 16,
                depth_or_array_layers: 1,
            };

            let texture = self
                .app_context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: texture_extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    view_formats: &[],
                    // view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                });

            // dimensions are 256 x 256 for `happy-tree.png`
            self.app_context.queue.write_texture(
                // equivanet to wgpu::ImageCopyTexture... etc
                // texture.as_image_copy()
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    // bytes_per_row: Some(4 * dimensions.0),
                    bytes_per_row: Some(texture_extent.width * pixel_size as u32),
                    rows_per_image: None,
                    // rows_per_image: Some(dimensions.1),
                },
                texture_extent,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let texture_sampler =
                self.app_context
                    .device
                    .create_sampler(&wgpu::SamplerDescriptor {
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Nearest,
                        mipmap_filter: wgpu::FilterMode::Nearest,
                        ..Default::default()
                    });

            let texture = Texture {
                texture,
                texture_view,
                texture_sampler,
                texture_extent,
            };

            let bind_group =
                self.app_context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("texture bindgroup"),
                        layout: &self.quad_pipeline.texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture.texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&texture.texture_sampler),
                            },
                        ],
                    });

            texture_map.insert(
                id,
                BindableTexture {
                    texture,
                    bind_group,
                },
            );


        }
    }

    pub fn create_dummy_texture(&self, id: String, data: &[u8]) {
        let mut texture_map = self.texture_map.lock().unwrap();
        if !texture_map.contains_key(&id) {
            use image::GenericImageView;

            let format = wgpu::TextureFormat::Rgba8UnormSrgb;

            let pixel_size = match format.block_dimensions() {
                (1, 1) => format.block_copy_size(None).unwrap() as usize,
                _ => panic!("Using pixel_size for compressed textures is invalid"),
            };

            // pixel_size = 4

            // let data = vec![data; pixel_size];

            let texture_extent = wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            };

            let texture = self
                .app_context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: texture_extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    view_formats: &[],
                    // view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                });

            // dimensions are 256 x 256 for `happy-tree.png`
            self.app_context.queue.write_texture(
                // equivanet to wgpu::ImageCopyTexture... etc
                // texture.as_image_copy()
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    // bytes_per_row: Some(4 * dimensions.0),
                    bytes_per_row: Some(texture_extent.width * pixel_size as u32),
                    rows_per_image: None,
                    // rows_per_image: Some(dimensions.1),
                },
                texture_extent,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let texture_sampler =
                self.app_context
                    .device
                    .create_sampler(&wgpu::SamplerDescriptor {
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Nearest,
                        mipmap_filter: wgpu::FilterMode::Nearest,
                        ..Default::default()
                    });

            let texture = Texture {
                texture,
                texture_view,
                texture_sampler,
                texture_extent,
            };

            let bind_group =
                self.app_context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("texture bindgroup"),
                        layout: &self.quad_pipeline.texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture.texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&texture.texture_sampler),
                            },
                        ],
                    });

            texture_map.insert(
                id,
                BindableTexture {
                    texture,
                    bind_group,
                },
            );
        }
    }

    pub fn create_texture(&self, id: String, texture_path: &str) {
        let mut texture_map = self.texture_map.lock().unwrap();
        if !texture_map.contains_key(&id) {
            let bytes = std::fs::read(texture_path).unwrap();
            let texture = Texture::from_bytes(
                &self.app_context.device,
                &self.app_context.queue,
                &bytes,
                &id,
            )
            .unwrap();

            // This should be reutilizable for all quads that have a texture.
            let bind_group_layout = self.app_context.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("texture bindgroup layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                },
            );

            let bind_group =
                self.app_context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("texture bindgroup"),
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture.texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&texture.texture_sampler),
                            },
                        ],
                    });
            texture_map.insert(
                id,
                BindableTexture {
                    texture,
                    bind_group,
                },
            );
        }
    }

  
    pub fn begin_render<'rpass, 'a: 'rpass>(
        &'rpass self,
        encoder: &'a mut wgpu::CommandEncoder,
        view: &'rpass wgpu::TextureView,
    ) -> RenderPass<'rpass> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass - Clear Color"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.2,
                        g: 0.3,
                        b: 0.9,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    pub fn render_quad(
        &mut self,
        position: Vec3,
        scale: Vec3,
        angle: f32,
        color: [f32; 4],
        texture_name: Option<String>,
    ) {
        self.quad_pipeline.quad_info.push(QuadInfo {
            color,
            texture_name,
            transform: TransformComponent {
                position: Mat4::from_translation(position),
                scale: Mat4::from_scale(scale),
                rotation: Mat4::from_rotation_z(angle),
            },
        });
    }

    pub fn update_quad_data(&mut self, device: &wgpu::Device) {
        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        // take it from a struct as its always the same for quads
        let color_uniform_alignment = {
            let alignment =
                device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(color_uniform_size, alignment)
        };
        let transform_uniform_size = std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress;
        let transform_uniform_alignment = {
            let alignment =
                device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(transform_uniform_size, alignment)
        };

        // println!("COLOR UNIFORM ALIGNMENT {:?}", color_uniform_alignment);
        // println!("TRANSFORM UNIFORM ALIGNMENT {:?}", transform_uniform_alignment);

        for (i, quad) in self.quad_pipeline.quad_info.iter_mut().enumerate() {
            let color_uniform_offset = ((i + 1) * color_uniform_alignment as usize) as u32;
            let transform_uniform_offset = ((i + 1) * transform_uniform_alignment as usize) as u32;
            self.app_context.queue.write_buffer(
                &self.quad_pipeline.color_buffer,
                color_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&quad.color),
            );

            // not needed as we don't have a camera YET. To be used later.
            // let view = Mat4::look_to_lh(vec3(0.0, 0.0, -1.0), vec3(0.0, 0.0, 0.0), Vec3::Y);
            let _view = Mat4::IDENTITY;

            // this is setting up the viewport basically
            let proj = Mat4::orthographic_lh(0.0, 800.0, 0.0, 600.0, -1.0, 1.0);
            let ar = 800.0 / 600.0;
            let proj = Mat4::orthographic_lh(-ar, ar, -1.0, 1.0, -1.0, 1.0);
            // let proj = Mat4::IDENTITY;
            let model = quad.transform.position * quad.transform.rotation * quad.transform.scale;

            let new_model = proj * model;
            self.app_context.queue.write_buffer(
                &self.quad_pipeline.model_mat4_buffer,
                transform_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&new_model.to_cols_array_2d()),
            );
        }
    }

    fn render_quads<'pass>(
        &'pass self,
        texture_map: &'pass TextureMap,
        render_pass: &mut RenderPass<'pass>,
        device: &wgpu::Device,
    ) {
        render_pass.set_pipeline(&self.quad_pipeline.render_pipeline);

        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        // Make the `uniform_alignment` >= `entity_uniform_size` and aligned to `min_uniform_buffer_offset_alignment`.
        let color_uniform_alignment = {
            let alignment =
                device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(color_uniform_size, alignment)
        };
        // dbg!(color_uniform_size); // 16
        // dbg!(color_uniform_alignment); // 256

        let transform_uniform_size = std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress;
        // Make the `uniform_alignment` >= `entity_uniform_size` and aligned to `min_uniform_buffer_offset_alignment`.
        let transform_uniform_alignment = {
            let alignment =
                device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(transform_uniform_size, alignment)
        };
        // dbg!(transform_uniform_size); // 192
        // dbg!(transform_uniform_alignment); // 256
        // dbg!(std::mem::size_of::<Mat4>()); // 64
        // dbg!(std::mem::size_of::<TransformComponent>()); // 256

        for (i, quad) in self.quad_pipeline.quad_info.iter().enumerate() {
            let bind_group: &wgpu::BindGroup;

            if let Some(id) = &quad.texture_name {
                bind_group = &texture_map.get(id).unwrap().bind_group;
                render_pass.set_bind_group(0, &bind_group, &[]);
            } else {
                bind_group = &texture_map.get("1px-white").unwrap().bind_group;
                render_pass.set_bind_group(0, &bind_group, &[]);
            }

            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_bind_group(
                1,
                &self.quad_pipeline.bind_group,
                &[
                    ((i + 1) * color_uniform_alignment as usize) as u32,
                    ((i + 1) * transform_uniform_alignment as usize) as u32,
                ],
            );
            render_pass.set_vertex_buffer(0, self.quad_pipeline.vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }
    }

    fn render_lines<'pass>(
        &'pass self,
        render_pass: &mut RenderPass<'pass>,
        device: &wgpu::Device,
    ) {
        render_pass.set_pipeline(&self.line_pipeline.render_pipeline);

        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        // Make the `uniform_alignment` >= `entity_uniform_size` and aligned to `min_uniform_buffer_offset_alignment`.
        let color_uniform_alignment = {
            let alignment =
                device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(color_uniform_size, alignment)
        };
        // dbg!(color_uniform_size); // 16
        // dbg!(color_uniform_alignment); // 256

        let transform_uniform_size = std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress;
        // Make the `uniform_alignment` >= `entity_uniform_size` and aligned to `min_uniform_buffer_offset_alignment`.
        let transform_uniform_alignment = {
            let alignment =
                device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(transform_uniform_size, alignment)
        };
        // dbg!(transform_uniform_size); // 192
        // dbg!(transform_uniform_alignment); // 256
        // dbg!(std::mem::size_of::<Mat4>()); // 64
        // dbg!(std::mem::size_of::<TransformComponent>()); // 256

        for (i, _line) in self.line_pipeline.line_info.iter().enumerate() {
            render_pass.set_bind_group(
                0,
                &self.line_pipeline.line_bind_group,
                &[
                    ((i + 1) * color_uniform_alignment as usize) as u32,
                    ((i + 1) * transform_uniform_alignment as usize) as u32,
                    ((i + 1) * transform_uniform_alignment as usize) as u32,
                ],
            );
            render_pass.set_vertex_buffer(0, self.line_pipeline.vertex_buffer.slice(..));
            render_pass.draw(0..2, 0..1);
        }
    }

    pub fn render_line(&mut self, orig: Vec3, dest: Vec3, color: [f32; 4]) {
        self.line_pipeline.line_info.push(LineInfo {
            color,
            transform: LineComponent {
                orig: Mat4::from_translation(orig),
                dest: Mat4::from_translation(dest),
            },
        });
    }

    pub fn update_line_data(&mut self, device: &wgpu::Device) {
        let color_uniform_size = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
        let color_uniform_alignment = {
            let alignment =
                device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(color_uniform_size, alignment)
        };
        let transform_uniform_size = std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress;
        let transform_uniform_alignment = {
            let alignment =
                device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;
            align_to(transform_uniform_size, alignment)
        };

        for (i, line) in self.line_pipeline.line_info.iter_mut().enumerate() {
            let color_uniform_offset = ((i + 1) * color_uniform_alignment as usize) as u32;
            self.app_context.queue.write_buffer(
                &self.line_pipeline.color_buffer,
                color_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&line.color),
            );

            // not needed as we don't have a camera YET. To be used later.
            // let view = Mat4::look_to_lh(vec3(0.0, 0.0, -1.0), vec3(0.0, 0.0, 0.0), Vec3::Y);
            let _view = Mat4::IDENTITY;

            // this is setting up the viewport basically
            let proj = Mat4::orthographic_lh(0.0, 800.0, 0.0, 600.0, -1.0, 1.0);

            let new_model = proj * line.transform.orig;
            let transform_uniform_offset = ((i + 1) * transform_uniform_alignment as usize) as u32;
            self.app_context.queue.write_buffer(
                &self.line_pipeline.orig_model_mat4_buffer,
                transform_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&new_model.to_cols_array_2d()),
            );

            let new_model = proj * line.transform.dest;
            let transform_uniform_offset = ((i + 1) * transform_uniform_alignment as usize) as u32;
            self.app_context.queue.write_buffer(
                &self.line_pipeline.dest_model_mat4_buffer,
                transform_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&new_model.to_cols_array_2d()),
            );
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MyEvent {
    KeyboardInput {
        state: ElementState,
        physical_key: PhysicalKey,
    },
}

pub type TextureMap = HashMap<String, BindableTexture>;

pub async fn async_runner(mut app: impl Application + 'static) {
    let event_loop = EventLoop::new().unwrap();
    let application_window_size2 = winit::dpi::LogicalSize::new(800.0, 600.0);
    let main_window = Arc::new(
        WindowBuilder::new()
            .with_inner_size(application_window_size2)
            .with_title("Game")
            .build(&event_loop)
            .unwrap(),
    );

    // IMPORTANT: this is different than before because I had added AppContext inside App along with Renderer
    let app_context = Arc::new(AppContext::new(main_window.clone()).await.unwrap());

    let texture_map: Arc<Mutex<TextureMap>> = Arc::new(Mutex::new(HashMap::new()));

    let mut engine = Engine::new(app_context.clone(), texture_map.clone());
    // engine.create_dummy_texture(String::from("1px-white"), &[0xFF, 0xFF, 0xFF, 0xFF]);
    engine.create_dummy_texture(String::from("1px-black"), &[0x00, 0x00, 0x00, 0xFF]);
    // engine.create_dummy_texture(String::from("1px-black"), bytemuck::cast_slice(&[0xff000000 as u32]));
    engine.create_dummy_texture(String::from("1px-grey"), &[0xAA, 0xAA, 0xAA, 0xFF]);

    // let x: &[u8] = bytemuck::cast_slice(&[0xff000000 as u32]);
    // let hdp = &[0x00, 0x00, 0x00, 0xFF];
    // println!("jajaj  {:?}", &x);
    // println!("jaja hdp  {:?}", &hdp);
    // panic!();

    let magenta: u32 = 0xFF00FFFF;
    // let magenta_ne = magenta.to_ne_bytes();
    // println!("jaja hdp  {:?}", &magenta_ne);
    // panic!();

    let magenta_bytes = [0xFF, 0x00, 0xFF, 0xFF];
    let black: u32 = 0x000000FF;
    // let black_bytes = black.to_ne_bytes();
    let black_bytes = [0x00, 0x00, 0x00, 0xFF];

    // let mut pixels = vec![[0; 4]; 16 * 16];
    let mut pixels: Vec<u32> = vec![0; 16 * 16];
    for x in 0..16 {
        for y in 0..16 {
            if (x % 2) ^ (y % 2) == 1 {
                // pixels[y * 16 + x] = magenta_bytes;
                pixels[y * 16 + x] = 0xffff00ff;
            } else {
                // pixels[y * 16 + x] = black_bytes;
                pixels[y * 16 + x] = 0xff000000;
            }
        }
    }


    // engine.create_magenta_texture(String::from("1px-magenta"), bytemuck::cast_slice(&pixels));
    println!("{:?}", &pixels);
    let xd: &[u8] = bytemuck::cast_slice(&pixels);
    println!("{:?}", xd);
    engine.create_dummy_texture_u32(String::from("1px-white"), bytemuck::cast_slice(&pixels));

    // engine.create_texture(id, texture_path)
    let mut framework = Framework::new(
        main_window.clone(),
        main_window.inner_size().width,
        main_window.inner_size().height,
        main_window.scale_factor() as f32,
        app_context.clone(),
    );


    app.on_setup(&mut engine);
    let _ = event_loop.run(move |event, _event_loop| match event {
        Event::WindowEvent {
            window_id: _,
            event,
        } => {
            framework.handle_event(&event);
            match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::KeyQ),
                        ..
                    },
                ..
            } => {
                dbg!("Received closed event");
                panic!("Closed");
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key,
                        ..
                    },
                ..
            } => {
                let new_event = MyEvent::KeyboardInput {
                    state,
                    physical_key,
                };

                dbg!(new_event);

                app.on_event(&mut engine, new_event);
            }

            WindowEvent::RedrawRequested => {
                framework.prepare();
                app.on_update(&mut engine);

                app.on_render(&mut engine);
                // IMPORTANT:
                // I can't store a renderpass because it needs a reference to a view and the view will
                // change upon resizing
                let frame = app_context
                    .surface
                    .get_current_texture()
                    .or_else(|_| {
                        app_context.reconfigure_surface();
                        app_context.surface.get_current_texture()
                    })
                    .unwrap();

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder =
                    app_context
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("pixels_command_encoder"),
                        });


                /////////////////////////////////////////////////
                ////////////////// RENDERING ////////////////////
                /////////////////////////////////////////////////
              
                engine.update_quad_data(&app_context.device);
                engine.update_line_data(&app_context.device);
                let texture_map = texture_map.lock().unwrap();
                {
                    let mut rpass = engine.begin_render(&mut encoder, &view);
                    engine.render_quads(&texture_map, &mut rpass, &app_context.device);
                    engine.render_lines(&mut rpass, &app_context.device);
                }

                {

                    framework.render(&mut encoder, &view, &app_context);
                }
                /////////////////////////////////////////////////
                ////////////////// RENDERING ////////////////////
                /////////////////////////////////////////////////


                engine.quad_pipeline.quad_info.clear();
                engine.line_pipeline.line_info.clear();

                app_context.queue.submit(Some(encoder.finish()));
                frame.present();

                main_window.request_redraw();
            }

            _ => (),
        }
    }
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta: _ },
            ..
        } => (),
        _ => (),
    });
}
