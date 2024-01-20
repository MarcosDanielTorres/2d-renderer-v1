#![allow(unused, dead_code, deprecated)]
use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;

use std::rc::Rc;

use std::ops::Index;
use std::sync::Arc;
use std::sync::Mutex;

use std::cell::RefCell;
use wgpu::RenderPass;
use wgpu::SurfaceConfiguration;
use wgpu::SurfaceTexture;
use wgpu::TextureView;
use winit::event::DeviceEvent;
use winit::event::ElementState;
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

// use crate::texture;

use wgpu::util::DeviceExt;

type Label<'a> = Option<&'a str>;

//TODO: create the pipeline necesessary to draw a quad to the screen.

// TODO: change name to Quad or Rectangle or Rect
#[derive(Default)]
pub struct Cube<'a> {
    label: Label<'a>,
    x: f32,
    y: f32,
    vertex_data: [f32; 4],
    index_data: [f32; 6],
}

impl<'a> Cube<'a> {
    pub fn new(x: f32, y: f32, label: Label<'a>) -> Self {
        let vertex_data = Cube::create_vertex_data();
        let index_data = Cube::create_index_data();
        Self {
            label,
            vertex_data,
            index_data,
            x,
            y,
        }
    }

    pub fn update(&mut self, engine: &mut Engine, new_pos: (f32, f32)) {
        self.x = new_pos.0;
        self.y = new_pos.1;
        println!("new pos of {:?}: ({}, {})", self.label, self.x, self.y);
    }

    fn create_vertex_data() -> [f32; 4] {
        [0.0; 4]
    }

    fn create_index_data() -> [f32; 6] {
        [1.0; 6]
    }
}

pub trait Application {
    fn on_update(&mut self, engine: &mut Engine);
    fn on_render(&mut self, engine: &mut Engine);
    fn on_event(&mut self, engine: &mut Engine, event: MyEvent);
}

struct QuadPipeline {
    id: u32,
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

        const VERTICES: &[Vertex] = &[
            Vertex {
                position: [0.0, 0.5, 0.0],
                tex_coords: [0.4131759, 0.00759614],
            },
            Vertex {
                position: [-0.5, -0.5, 0.0],
                tex_coords: [0.0048659444, 0.43041354],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                tex_coords: [0.28081453, 0.949397],
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

        // textures
        let diffuse_bytes = include_bytes!("happy-tree.png");
        let diffuse_texture = Texture::from_bytes(
            &app_context.device,
            &app_context.queue,
            diffuse_bytes,
            "Happy tree",
        )
        .unwrap();

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
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        let translation = glam::Mat4::from_translation(glam::Vec3::ONE);
        let scale = glam::Mat4::from_scale(glam::Vec3::ONE);
        let model = scale * translation;
        // glam::Mat4::from_scale_rotation_translation(scale, rotation, translation)

        let model_mat4_buffer =
            app_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("translation Buffer"),
                    contents: bytemuck::cast_slice(&model.to_cols_array_2d()),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // modificar la estructura para contener estos putos buffers
        // modificar el shader
        // escribir a la queue desde el draw quad
        // rezar a todos los putos dioses del universo
        // si anda probar con la rotacion o mejor no... que idea de mierda
        //
        // toda esta mierda no escala ni en pedo, porque estoy creanod un render pass por cada mierda
        // en vez de meter todo en un solo lado y listo, podria intentar hacer eso que hacen en bevy
        // con el rendercontext, porque hacen algunas cosas con take y las options que toman ownership.
        // Capaz va por ahi, la verdad ni idea son tan hijos de puta los de bevy que fueron por ecs
        // todo. estoy convencido en que todos hacen todo como el orto con wgpu excepto bevy.
        //
        // Tengo que ver que mierda hace comfy con los command encoder y command buffer y render
        // passes. a ver si los guardan en algun lado o son tan hijos de puta croteros que hacen todo
        // como el orto igual que yo... yo nunca hice un engine, soy mejor que todos en realidad les
        // rompo el culo. no hay comparacion mi nivel es otro

        // let color_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //         label: Some("Color Buffer"),
        //         size: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
        //         mapped_at_creation: false,
        //         usage: wgpu::BufferUsages::UNIFORM,
        //     });

        let color_slice: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        let color_buffer =
            app_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Color Buffer"),
                    contents: bytemuck::cast_slice(&color_slice),
                    // maybe the `COPY_DST` breaks it
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
                        resource: color_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: model_mat4_buffer.as_entire_binding(),
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
    pub fn draw<'a, 'r: 'a>(&'r self, color: [f32; 4], mut render_pass: wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
    }
}

pub struct Engine {
    app_context: Arc<AppContext>,
    quad_pipeline: QuadPipeline,
    command_encoder: Arc<Mutex<wgpu::CommandEncoder>>,
    // render_pass: HashMap<u32, Rc<RefCell<RenderPass<'static>>>>,
}

impl Engine {
    pub fn new(app_context: Arc<AppContext>) -> Self {
        let quad_pipeline = QuadPipeline::new(app_context.clone());

        let command_encoder = Arc::new(Mutex::new(app_context.create_command_encoder()));

        Self {
            app_context: app_context.clone(),
            quad_pipeline,
            command_encoder,
        }
    }

    pub fn begin_render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            });
        }
    }

    pub fn end_render(&mut self, mut encoder: wgpu::CommandEncoder, frame: SurfaceTexture) {
        // let hdp = self.command_encoder.clone();
        // puedo submittear una collecion de command buffers, que salen cuando hago finish en un
        // encoder
        // self.app_context.queue.submit(Some(hdp.lock().unwrap().finish()));
        self.app_context.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    pub fn draw_quad(
        &mut self,
        position: glam::Mat4,
        scale: glam::Mat4,
        rotation: glam::Mat4,
        color: [f32; 4],
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.app_context.queue.write_buffer(
            &self.quad_pipeline.color_buffer,
            0,
            bytemuck::cast_slice(&color),
        );

        let model = scale * position * rotation;
        self.app_context.queue.write_buffer(
            &self.quad_pipeline.model_mat4_buffer,
            0,
            bytemuck::cast_slice(&model.to_cols_array_2d()),
        );
        self.quad_pipeline.draw(color, render_pass);
    }
}

/*
 * se crea la pipeline y cuando se quiere crear un puto quad solo se escribe el color_buffer o la
 * texture y listo
 *
 * QuadPipeline con todo
 *
 * engine.draw_quad()
 *  setea color o texture
 *  quad_pipeline(encoder)
 */

#[derive(Debug, Copy, Clone)]
pub enum MyEvent {
    KeyboardInput {
        state: ElementState,
        virtual_keycode: VirtualKeyCode,
    },
}

pub async fn async_runner(mut app: impl Application + 'static) {
    let event_loop = EventLoop::new();
    let main_window = WindowBuilder::new()
        .with_title("Game")
        .build(&event_loop)
        .unwrap();

    // this is different than before because I had added AppContext inside App along with Renderer
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

            // initially i thought this would go in the begin_render but
            // if ...todo
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

            // esto asi no anda, probar que pasaria si mando el renderpass creado desde aca por
            // parametro.
            //
            // Por que no anda esto? Creo que no anda porque se crea un renderpass, se escribe. Se
            // crea otro renderpass y se escribe y recien ahi se manda. Pero el ultimo renderpass
            // pisa al primero mmm testear
            //
            // I want to have two triangles on the screen at the same time.

            // clear screen
            // let clear_screen_color: [f64; 4] = [1.0, 1.0, 0.5, 1.0];
            // clear_screen(
            //     &mut encoder,
            //     &view, clear_screen_color);

            // render first triangle
            // render(
            //     &mut encoder,
            //     &view,
            //     &app_context.device,
            //     &app_context.config,
            //     &app_context.queue,
            //     1.0,
            // );
            //
            // render second triangle
            // render(
            //     &mut encoder,
            //     &view,
            //     &app_context.device,
            //     &app_context.config,
            //     &app_context.queue,
            //     0.0,
            // );



            engine.begin_render(&mut encoder, &view);

            // default values:
            // pos es 0
            // scale es 1
            // rot es 0

            // First triangle
            let color1: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
            let pos1 = glam::Vec3::new(1.5, 1.0, 0.0);
            let scale1 = glam::Vec3::new(0.3, 0.3, 0.0);
            let rotation_in_z1: f32 = 1.5708;
            //TODO: Fix it rotates around the world (0, 0) and not around its local (0, 0)
            // Brendan Galea

            // TODO: do quaternions with `from_scale_rotation_translation`
            engine.draw_quad(
                glam::Mat4::from_translation(pos1),
                glam::Mat4::from_scale(scale1),
                glam::Mat4::from_rotation_z(rotation_in_z1),
                color1,
                &mut encoder,
                &view,
            );

            // queues writes buffers but the wr gets submitted at the end, so the queue ends up
            // with only one write??? let's see if I can access the vector of commands, but not
            // yet. Lets add rottion first
            
            // Second triangle
            let color2: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
            let pos2 = glam::Vec3::new(0.0, 0.0, 0.0);
            let scale2 = glam::Vec3::new(0.3, 0.3, 0.0);
            let rotation_in_z2: f32 = 0.0;

            engine.draw_quad(
                glam::Mat4::from_translation(pos2),
                glam::Mat4::from_scale(scale2),
                glam::Mat4::from_rotation_z(rotation_in_z2),
                color2,
                &mut encoder,
                &view,
            );
            

            engine.end_render(encoder, frame);

            // uncomment that if want to use aux_render, currently not working of course
            // {
            //
            //     let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            //         label: Some("Render Pass"),
            //         color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            //             view: &view,
            //             resolve_target: None,
            //             ops: wgpu::Operations {
            //                 load: wgpu::LoadOp::Load,
            //                 store: wgpu::StoreOp::Store,
            //             },
            //         })],
            //         depth_stencil_attachment: None,
            //         timestamp_writes: None,
            //         occlusion_query_set: None,
            //     });
            //
            //         aux_render(
            //             &mut render_pass,
            //             &view,
            //             &app_context.device,
            //             &app_context.config,
            //             &app_context.queue,
            //             0.0
            //         );
            // }

            // app_context.queue.submit(Some(encoder.finish()));
            // frame.present();

            // diff points to note.
            // 1 Bevy uses: ErasedDevice(Arc<Device>) with a deref
            // 2 If I had: All this be and Arc I could then just do
            // TODO: investigate how to structure so i can borrow different parts without affecting
            // all of them for example: &mut app_context.device, &app_context.config, &app_context.queue this is
            // invalid

            // let render_pass = engine.begin_render(&mut encoder, &view);
            main_window.request_redraw();
        }
        _ => (),
    })
}
pub fn clear_screen(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    clear_screen_color: [f64; 4],
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: clear_screen_color[0],
                    g: clear_screen_color[1],
                    b: clear_screen_color[2],
                    a: clear_screen_color[3],
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
}

pub fn render(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    device: &wgpu::Device,
    config: &SurfaceConfiguration,
    queue: &wgpu::Queue,
    offset: f32,
) {
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

    let mut VERTICES: &mut [Vertex] = &mut [
        Vertex {
            position: [0.0, 0.5, 0.0],
            tex_coords: [0.4131759, 0.00759614],
        },
        Vertex {
            position: [-0.5, -0.5, 0.0],
            tex_coords: [0.0048659444, 0.43041354],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
            tex_coords: [0.28081453, 0.949397],
        },
    ];

    if offset == 1.0 {
        VERTICES[0] = Vertex {
            position: [1.0, 0.5, 0.0],
            tex_coords: [0.4131759, 0.00759614],
        };
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // textures
    let diffuse_bytes = include_bytes!("happy-tree.png");
    let diffuse_texture =
        Texture::from_bytes(&device, &queue, diffuse_bytes, "Happy tree").unwrap();

    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

    let color_slice: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Color Buffer"),
        contents: bytemuck::cast_slice(&color_slice),
        // maybe the `COPY_DST` breaks it
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                resource: color_buffer.as_entire_binding(),
            },
        ],
    });

    let module = wgpu::ShaderModuleDescriptor {
        label: Some("Builtin Quad Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/builtin_quad.wgsl").into()),
    };

    let pipeline = RenderPipelineBuilder::new()
        .add_vertex_buffer_layout::<Vertex>()
        .add_color_target_state(wgpu::ColorTargetState {
            format: config.format,
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
        .build(device, "Vertex Pipeline", "vs_main", "fs_main");

    {
        // let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //     label: Some("Render Pass"),
        //     color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        //         view,
        //         resolve_target: None,
        //         ops: wgpu::Operations {
        //             load: wgpu::LoadOp::Clear(wgpu::Color {
        //                 r: 1.0,
        //                 g: 1.0,
        //                 b: 1.0,
        //                 a: 1.0,
        //             }),
        //             store: wgpu::StoreOp::Store,
        //         },
        //     })],
        //     depth_stencil_attachment: None,
        //     timestamp_writes: None,
        //     occlusion_query_set: None,
        // });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&pipeline);
        // render_pass.set_bind_group(0, &bind_group, &[]); for uniforms
        render_pass.set_bind_group(0, &diffuse_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
        // render_pass.set_index_buffer([], wgpu::indexformat::uint16)
        // render_pass.draw_indexed(0..6, 0..1); for indexed
    }

    // - CommandEncoder 1 to many RenderPass
    //
    // TODO: can I have a renderpass that only clears the screen without pipeline associated? So in
    // the begin_render I can run this pass to clear everything and then draw everything again usin
    // load, without clearing Done its possible, see my impl 'clear_screen_color'
    //
    // TODO: reutilizar el mismo puto pase para todos los draw del triangulo. esto implicaria
    // crearlo afuera. Despues podria tener: self.renderpass
    // o puedo tener un RenderPassDescritptor y hacer: crea el encoder con este pass
    //
    //
    // Entonces me quedaria un pass que solo clearea y despues otro pass que dibujan todos los
    // quads, despues otro para todos los something else, etc. Para el caso de un 2D renderer
    // supongo que solo usaria un quadpipeline o casi siempre al menos usaria esto ya que no hay
    // muchas mas formas que no esten formadas por un puto quad y una texture. A no ser que quiera
    // hacer un circulo, que quizas tambien podria ser un quad con una textura ciruclar arriba o un
    // circulo creado en el shader y quemado en una textura con esta forma (creo que esto es lo que
    // hace the cherno pero no estoy seguro, deberia investigar cuando llegue el momento, pero por
    // ahora es innecesario. Al backlog!)
}

// he said, separate prepartion and draw phase, maybe this is what he means by that
// pub fn aux_render<'b, 'a: 'b>(
//     render_pass: &'b mut RenderPass,
//     view: &wgpu::TextureView,
//     device: &wgpu::Device,
//     config: &SurfaceConfiguration,
//     queue: &wgpu::Queue,
//     offset: f32,
// ) {
//     // vertex
//     #[repr(C)]
//     #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
//     struct Vertex {
//         position: [f32; 3],
//         tex_coords: [f32; 2],
//     }
//
//     impl<'a> VertexDescriptor<'a> for Vertex {
//         fn desc() -> wgpu::VertexBufferLayout<'a> {
//             wgpu::VertexBufferLayout {
//                 array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
//                 step_mode: wgpu::VertexStepMode::Vertex,
//                 attributes: &[
//                     wgpu::VertexAttribute {
//                         offset: 0,
//                         shader_location: 0,
//                         format: wgpu::VertexFormat::Float32x3,
//                     },
//                     wgpu::VertexAttribute {
//                         offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
//                         shader_location: 1,
//                         format: wgpu::VertexFormat::Float32x2,
//                     },
//                 ],
//             }
//         }
//     }
//
//     let mut VERTICES: &mut [Vertex] = &mut [
//         Vertex {
//             position: [0.0, 0.5, 0.0],
//             tex_coords: [0.4131759, 0.00759614],
//         },
//         Vertex {
//             position: [-0.5, -0.5, 0.0],
//             tex_coords: [0.0048659444, 0.43041354],
//         },
//         Vertex {
//             position: [0.5, -0.5, 0.0],
//             tex_coords: [0.28081453, 0.949397],
//         },
//     ];
//
//     if offset == 1.0 {
//         VERTICES[0] = Vertex {
//             position: [1.0, 0.5, 0.0],
//             tex_coords: [0.4131759, 0.00759614],
//         };
//     }
//
//     let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//         label: Some("Vertex Buffer"),
//         contents: bytemuck::cast_slice(VERTICES),
//         usage: wgpu::BufferUsages::VERTEX,
//     });
//
//     // textures
//     let diffuse_bytes = include_bytes!("happy-tree.png");
//     let diffuse_texture =
//         Texture::from_bytes(&device, &queue, diffuse_bytes, "Happy tree").unwrap();
//
//     let texture_bind_group_layout =
//         device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//             entries: &[
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Texture {
//                         multisampled: false,
//                         view_dimension: wgpu::TextureViewDimension::D2,
//                         sample_type: wgpu::TextureSampleType::Float { filterable: true },
//                     },
//                     count: None,
//                 },
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 1,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
//                     count: None,
//                 },
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 2,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Uniform,
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 },
//             ],
//             label: Some("texture_bind_group_layout"),
//         });
//
//     // let color_buffer = device.create_buffer(&wgpu::BufferDescriptor {
//     //         label: Some("Color Buffer"),
//     //         size: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
//     //         mapped_at_creation: false,
//     //         usage: wgpu::BufferUsages::UNIFORM,
//     //     });
//
//     let color_slice: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
//     let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//         label: Some("Color Buffer"),
//         contents: bytemuck::cast_slice(&color_slice),
//         // maybe the `COPY_DST` breaks it
//         usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//     });
//
//     let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//         label: Some("diffuse_bind_group"),
//         layout: &texture_bind_group_layout,
//         entries: &[
//             wgpu::BindGroupEntry {
//                 binding: 0,
//                 resource: wgpu::BindingResource::TextureView(&diffuse_texture.texture_view),
//             },
//             wgpu::BindGroupEntry {
//                 binding: 1,
//                 resource: wgpu::BindingResource::Sampler(&diffuse_texture.texture_sampler),
//             },
//             wgpu::BindGroupEntry {
//                 binding: 2,
//                 resource: color_buffer.as_entire_binding(),
//             },
//         ],
//     });
//
//     let module = wgpu::ShaderModuleDescriptor {
//         label: Some("Builtin Quad Shader"),
//         source: wgpu::ShaderSource::Wgsl(include_str!("shaders/builtin_quad.wgsl").into()),
//     };
//
//     let pipeline = RenderPipelineBuilder::new()
//         .add_vertex_buffer_layout::<Vertex>()
//         .add_color_target_state(wgpu::ColorTargetState {
//             format: config.format,
//             blend: Some(wgpu::BlendState {
//                 color: wgpu::BlendComponent::REPLACE,
//                 alpha: wgpu::BlendComponent::REPLACE,
//             }),
//             write_mask: wgpu::ColorWrites::ALL,
//         })
//         .shader(module)
//         .pipeline_layout_descriptor(
//             "Vertex layout descriptor",
//             &[&texture_bind_group_layout],
//             &[],
//         )
//         .build(device, "Vertex Pipeline", "vs_main", "fs_main");
//
//     {
//         render_pass.set_pipeline(&pipeline);
//         render_pass.set_bind_group(0, &diffuse_bind_group, &[]);
//         render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
//         render_pass.draw(0..3, 0..1);
//     }
// }
