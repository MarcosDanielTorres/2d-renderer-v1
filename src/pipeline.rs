pub trait VertexDescriptor<'a> {
    fn desc() -> wgpu::VertexBufferLayout<'a>;
}

pub struct RenderPipelineBuilder<'a> {
    shader_module_descriptor: Option<wgpu::ShaderModuleDescriptor<'a>>,
    pipeline_layout_descriptor: Option<wgpu::PipelineLayoutDescriptor<'a>>,
    render_pipeline_descriptor: Option<wgpu::RenderPipelineDescriptor<'a>>,
    wireframe: bool,
    topology: wgpu::PrimitiveTopology,
    vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn new() -> Self {
        Self {
            shader_module_descriptor: None,
            pipeline_layout_descriptor: None,
            render_pipeline_descriptor: None,
            wireframe: false,
            topology: wgpu::PrimitiveTopology::TriangleList,
            vertex_buffer_layouts: vec![],
            color_target_states: vec![],
        }
    }

    /*
    Change desing of API to store a ShaderModuleDescriptor instead of a ShaderModule
    and use self.device.create_shader_module in the build method

    Could or not use a new Option<ShaderModuleDescriptor> instead of the old Option<ShaderModule>
    or just remove the option entirely. What are the benefits of both approaches?
    */
    pub fn shader(mut self, shader_module_descriptor: wgpu::ShaderModuleDescriptor<'a>) -> Self {
        self.shader_module_descriptor = Some(shader_module_descriptor);
        self
    }

    pub fn pipeline_layout_descriptor(
        mut self,
        label: &'a str,
        bind_group_layouts: &'a [&wgpu::BindGroupLayout],
        push_constant_ranges: &'a [wgpu::PushConstantRange],
    ) -> Self {
        // I think bind_group_layouts are only used with uniforms
        // vertex and index buffer only need to be in the render pass once created
        // only the description of the vertex will go in the VertexState
        let pipeline_layout_descriptor = wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts,
            push_constant_ranges,
        };

        self.pipeline_layout_descriptor = Some(pipeline_layout_descriptor);
        self
    }

    #[allow(unused)]
    pub fn with_wireframe(mut self) -> Self {
        self.wireframe = true;
        self
    }

    #[allow(unused)]
    pub fn with_topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    #[allow(unused)]
    pub fn add_vertex_buffer_layout<V: VertexDescriptor<'a>>(mut self) -> Self {
        self.vertex_buffer_layouts.push(V::desc());
        self
    }

    #[allow(unused)]
    pub fn add_color_target_state(mut self, color_target_state: wgpu::ColorTargetState) -> Self {
        self.color_target_states.push(Some(color_target_state));
        self
    }

    pub fn build(
        self,
        device: &wgpu::Device,
        pipeline_label: &str,
        vs_entry_point: &str,
        fs_entry_point: &str,
    ) -> wgpu::RenderPipeline {
        let Some(shader_module_descriptor) = self.shader_module_descriptor else {
            panic!("Shader Module is None")
        };

        let shader_module = device.create_shader_module(shader_module_descriptor);

        let vertex_state = wgpu::VertexState {
            module: &shader_module,
            entry_point: vs_entry_point,
            buffers: &self.vertex_buffer_layouts,
        };

        let fragment_state = wgpu::FragmentState {
            module: &shader_module,
            entry_point: fs_entry_point,
            targets: &self.color_target_states,
        };

        let Some(pipeline_layout_descriptor) = self.pipeline_layout_descriptor else {
            panic!("Pipeline Layout Descriptor is None")
        };

        let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_descriptor);

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(pipeline_label),
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            fragment: Some(fragment_state),
            primitive: wgpu::PrimitiveState {
                // topology: wgpu::PrimitiveTopology::TriangleList,
                topology: self.topology,
                strip_index_format: None,
                //clock
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: if self.wireframe {
                    wgpu::PolygonMode::Line
                } else {
                    wgpu::PolygonMode::Fill
                },
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }
}

impl Clone for RenderPipelineBuilder<'_> {
    fn clone(&self) -> Self {
        Self {
            shader_module_descriptor: self.shader_module_descriptor.clone(),
            pipeline_layout_descriptor: self.pipeline_layout_descriptor.clone(),
            render_pipeline_descriptor: self.render_pipeline_descriptor.clone(),
            wireframe: self.wireframe,
            topology: self.topology,
            vertex_buffer_layouts: self.vertex_buffer_layouts.clone(),
            color_target_states: self.color_target_states.clone(),
        }
    }
}

pub struct ComputePipelineBuilder<'a> {
    shader_module_descriptor: Option<wgpu::ShaderModuleDescriptor<'a>>,
    pipeline_layout_descriptor: Option<wgpu::PipelineLayoutDescriptor<'a>>,
}

impl<'a> ComputePipelineBuilder<'a> {
    pub fn new() -> Self {
        Self {
            shader_module_descriptor: None,
            pipeline_layout_descriptor: None,
        }
    }

    pub fn shader(mut self, shader_module_descriptor: wgpu::ShaderModuleDescriptor<'a>) -> Self {
        self.shader_module_descriptor = Some(shader_module_descriptor);
        self
    }

    pub fn pipeline_layout_descriptor<'r: 'a>(
        mut self,
        label: &'a str,
        bind_group_layouts: &'r [&'r wgpu::BindGroupLayout],
        push_constant_ranges: &'r [wgpu::PushConstantRange],
    ) -> Self {
        // I think bind_group_layouts are only used with uniforms
        // vertex and index buffer only need to be in the render pass once created
        // only the description of the vertex will go in the VertexState

        let pipeline_layout_descriptor = wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts,
            push_constant_ranges,
        };

        self.pipeline_layout_descriptor = Some(pipeline_layout_descriptor);
        self
    }

    pub fn build(
        self,
        device: &wgpu::Device,
        pipeline_label: &str,
        entry_point: &str,
    ) -> wgpu::ComputePipeline {
        let Some(shader_module_descriptor) = self.shader_module_descriptor else {
            panic!("Shader Module is None")
        };

        let shader_module = device.create_shader_module(shader_module_descriptor);

        let Some(pipeline_layout_descriptor) = self.pipeline_layout_descriptor else {
            panic!("Pipeline Layout Descriptor is None")
        };

        let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_descriptor);

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: Some(&pipeline_layout),
            entry_point,
            module: &shader_module,
            label: Some(pipeline_label),
        })
    }
}

impl Clone for ComputePipelineBuilder<'_> {
    fn clone(&self) -> Self {
        Self {
            shader_module_descriptor: self.shader_module_descriptor.clone(),
            pipeline_layout_descriptor: self.pipeline_layout_descriptor.clone(),
        }
    }
}
