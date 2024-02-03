#![allow(unused, dead_code, deprecated)]
use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;

use std::f32::consts::FRAC_PI_2;
use std::f32::consts::FRAC_PI_3;
use std::f32::consts::FRAC_PI_4;
use std::num::NonZeroU64;
use std::rc::Rc;

use std::ops::Index;
use std::sync::Arc;
use std::sync::Mutex;

use std::cell::RefCell;
use wgpu::BufferSize;
use wgpu::RenderPass;
use wgpu::SurfaceConfiguration;
use wgpu::SurfaceTexture;
use wgpu::TextureView;
use winit::event::DeviceEvent;
use winit::event::ElementState;
use winit::window::Fullscreen;
use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::*,
    window::WindowBuilder,
};

mod context;
mod pipeline;
mod texture;
use texture::Texture;

use crate::context::AppContext;

use crate::pipeline::RenderPipelineBuilder;
use crate::pipeline::VertexDescriptor;

use wgpu::util::{align_to, DeviceExt};

pub trait Application {
    fn on_update(&mut self, engine: &mut Engine);
    fn on_render(&mut self, engine: &mut Engine);
    fn on_event(&mut self, engine: &mut Engine, event: MyEvent);
}

struct TransformComponent {
    position: glam::Mat4,
    scale: glam::Mat4,
    rotation: glam::Mat4,
}

struct LineComponent {
    orig: glam::Mat4,
    dest: glam::Mat4,
}

// LINE
struct LineInfo {
    // updated for every `draw_quad`
    transform: LineComponent,
    color: [f32; 4],
}

struct LinePipeline {
    id: u32,
    line_info: Vec<LineInfo>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    model_mat4_buffer: wgpu::Buffer,
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
                position: [-0.5, 0.0, 0.0],
            },
            Vertex {
                position: [0.5, 0.0, 0.0],
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
                            buffer: &model_mat4_buffer,
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
            id: 323,
            line_info: vec![],
            render_pipeline,
            vertex_buffer,
            color_buffer,
            model_mat4_buffer,
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
}

struct QuadPipeline {
    id: u32,
    quad_info: Vec<QuadInfo>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    model_mat4_buffer: wgpu::Buffer,
    diffuse_texture: texture::Texture,
    diffuse_bind_group: wgpu::BindGroup,
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(color_uniform_size),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
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
        let color_slice: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        let color_buffer = app_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Color Buffer"),
            size: 40 * color_uniform_alignment,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let diffuse_bind_group = app_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("diffuse_bind_group"),
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
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &color_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(color_uniform_size),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &model_mat4_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(transform_uniform_size),
                        }),
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
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })
            .shader(module)
            .pipeline_layout_descriptor(
                "Vertex layout descriptor",
                &[&texture_bind_group_layout],
                &[],
            )
            .build(&app_context.device, "Vertex Pipeline", "vs_main", "fs_main");

        Self {
            id: 322,
            quad_info: vec![],
            render_pipeline,
            vertex_buffer,
            color_buffer,
            model_mat4_buffer,
            diffuse_bind_group,
            diffuse_texture,
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

pub struct Engine {
    app_context: Arc<AppContext>,
    quad_pipeline: QuadPipeline,
    line_pipeline: LinePipeline,
    command_encoder: Arc<Mutex<wgpu::CommandEncoder>>,
}

impl Engine {
    pub fn new(app_context: Arc<AppContext>) -> Self {
        let quad_pipeline = QuadPipeline::new(app_context.clone());
        let line_pipeline = LinePipeline::new(app_context.clone());

        let command_encoder = Arc::new(Mutex::new(app_context.create_command_encoder()));

        Self {
            app_context: app_context.clone(),
            quad_pipeline,
            line_pipeline,
            command_encoder,
        }
    }

    pub fn begin_render<'rpass>(
        &'rpass self,
        encoder: &'rpass mut wgpu::CommandEncoder,
        view: &'rpass wgpu::TextureView,
    ) -> RenderPass<'rpass> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
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

    pub fn end_render(&mut self, mut encoder: wgpu::CommandEncoder, frame: SurfaceTexture) {
        self.app_context.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    pub fn prepare_quad_data(
        &mut self,
        position: glam::Mat4,
        scale: glam::Mat4,
        rotation: glam::Mat4,
        color: [f32; 4],
    ) {
        self.quad_pipeline.quad_info.push(QuadInfo {
            color,
            transform: TransformComponent {
                position,
                scale,
                rotation,
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

        for (i, quad) in self.quad_pipeline.quad_info.iter_mut().enumerate() {
            let color_uniform_offset = ((i + 1) * color_uniform_alignment as usize) as u32;
            let transform_uniform_offset = ((i + 1) * transform_uniform_alignment as usize) as u32;
            self.app_context.queue.write_buffer(
                &self.quad_pipeline.color_buffer,
                color_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&quad.color),
            );

            // not needed as we don't have a camera YET
            // let view = glam::Mat4::look_to_lh(glam::vec3(0.0, 0.0, -1.0), glam::vec3(0.0, 0.0, 0.0), glam::Vec3::Y);
            let view = glam::Mat4::IDENTITY;

            // this is setting up the viewport basically
            let proj = glam::Mat4::orthographic_lh(0.0, 800.0, 0.0, 600.0, -1.0, 1.0);
            let model = quad.transform.position * quad.transform.rotation * quad.transform.scale;

            let new_model = proj * model;
            self.app_context.queue.write_buffer(
                &self.quad_pipeline.model_mat4_buffer,
                transform_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&new_model.to_cols_array_2d()),
            );
        }
    }

    pub fn render_quads<'pass>(
        &'pass self,
        render_pass: &mut RenderPass<'pass>,
        queue: &wgpu::Queue,
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
        // dbg!(std::mem::size_of::<glam::Mat4>()); // 64
        // dbg!(std::mem::size_of::<TransformComponent>()); // 256

        for (i, _quad) in self.quad_pipeline.quad_info.iter().enumerate() {
            render_pass.set_bind_group(
                0,
                &self.quad_pipeline.diffuse_bind_group,
                &[
                    ((i + 1) * color_uniform_alignment as usize) as u32,
                    ((i + 1) * transform_uniform_alignment as usize) as u32,
                ],
            );
            render_pass.set_vertex_buffer(0, self.quad_pipeline.vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }
    }
    pub fn render_lines<'pass>(
        &'pass self,
        render_pass: &mut RenderPass<'pass>,
        queue: &wgpu::Queue,
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
        // dbg!(std::mem::size_of::<glam::Mat4>()); // 64
        // dbg!(std::mem::size_of::<TransformComponent>()); // 256

        for (i, _line) in self.line_pipeline.line_info.iter().enumerate() {
            render_pass.set_bind_group(
                0,
                &self.line_pipeline.line_bind_group,
                &[
                    ((i + 1) * color_uniform_alignment as usize) as u32,
                    ((i + 1) * transform_uniform_alignment as usize) as u32,
                ],
            );
            render_pass.set_vertex_buffer(0, self.line_pipeline.vertex_buffer.slice(..));
            render_pass.draw(0..2, 0..1);
        }
    }

    pub fn prepare_line_data(&mut self, position: glam::Vec3, scale: glam::Vec3, color: [f32; 4]) {
        self.line_pipeline.line_info.push(LineInfo {
            color,
            transform: LineComponent {
                orig: glam::Mat4::from_translation(position),
                dest: glam::Mat4::from_translation(position),
            },
        });


    }

    pub fn update_line_data(&mut self, device: &wgpu::Device) {
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

        for (i, line) in self.line_pipeline.line_info.iter_mut().enumerate() {
            let color_uniform_offset = ((i + 1) * color_uniform_alignment as usize) as u32;
            let transform_uniform_offset = ((i + 1) * transform_uniform_alignment as usize) as u32;
            self.app_context.queue.write_buffer(
                &self.line_pipeline.color_buffer,
                color_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&line.color),
            );

            // not needed as we don't have a camera YET
            // let view = glam::Mat4::look_to_lh(glam::vec3(0.0, 0.0, -1.0), glam::vec3(0.0, 0.0, 0.0), glam::Vec3::Y);
            let view = glam::Mat4::IDENTITY;

            // this is setting up the viewport basically
            let proj = glam::Mat4::orthographic_lh(0.0, 800.0, 0.0, 600.0, -1.0, 1.0);
            let model = line.transform.position * line.transform.rotation * line.transform.scale;

            let new_model = proj * model;
            self.app_context.queue.write_buffer(
                &self.line_pipeline.model_mat4_buffer,
                transform_uniform_offset as wgpu::BufferAddress,
                bytemuck::cast_slice(&new_model.to_cols_array_2d()),
            );
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MyEvent {
    KeyboardInput {
        state: ElementState,
        virtual_keycode: VirtualKeyCode,
    },
}

pub async fn async_runner(mut app: impl Application + 'static) {
    let event_loop = EventLoop::new();
    let application_window_size = winit::dpi::PhysicalSize::new(800.0, 600.0);
    let application_window_size2 = winit::dpi::LogicalSize::new(800.0, 600.0);
    let main_window = WindowBuilder::new()
        .with_inner_size(application_window_size2)
        .with_title("Game")
        .build(&event_loop)
        .unwrap();

    // IMPORTANT: this is different than before because I had added AppContext inside App along with Renderer
    let app_context = Arc::new({ AppContext::new(&main_window).await });

    let mut engine = Engine::new(app_context.clone());

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { window_id, event } => match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        scancode,
                        state,
                        virtual_keycode: Some(VirtualKeyCode::Q),
                        modifiers,
                    },
                ..
            } => {
                dbg!("Received closed event");
                panic!("Closed");
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        scancode,
                        state,
                        virtual_keycode,
                        modifiers,
                    },
                ..
            } => {
                let new_event = MyEvent::KeyboardInput {
                    state,
                    virtual_keycode: virtual_keycode.unwrap(),
                };

                dbg!(new_event);

                app.on_event(&mut engine, new_event);
            }
            _ => (),
        },
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } => (),
        Event::RedrawRequested(_window_id) => {
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

            // IMPORTANT:
            // Identity values:
            // pos is 0
            // scale is 1
            // rot is 0

            // RENDERING
            engine.update_quad_data(&app_context.device);
            engine.update_line_data(&app_context.device);
            {
                let mut rpass = engine.begin_render(&mut encoder, &view);
                engine.render_quads(&mut rpass, &app_context.queue, &app_context.device);
                engine.render_lines(&mut rpass, &app_context.queue, &app_context.device);
            }

            engine.quad_pipeline.quad_info.clear();
            engine.line_pipeline.line_info.clear();

            app_context.queue.submit(Some(encoder.finish()));
            frame.present();

            main_window.request_redraw();
        }
        _ => (),
    })
}
