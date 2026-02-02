use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use std::sync::Arc;
use std::time::SystemTime;
use image::GenericImageView;
use glam::{Mat4, Vec3};
use lume_core::{Instance, InstanceDescriptor, Backend, Device, shader::{compile_shader, ShaderSource}, device::{SwapchainDescriptor, RenderPassDescriptor, TextureFormat, PipelineLayoutDescriptor, GraphicsPipelineDescriptor, PrimitiveState, PrimitiveTopology, CommandPool, CommandBuffer, FramebufferDescriptor, Swapchain, Buffer, BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStage, BindingType, BindGroupDescriptor, BindGroupEntry, BindingResource, TextureDescriptor, TextureUsage, SamplerDescriptor, FilterMode, AddressMode, TextureViewDescriptor, ImageLayout, DepthStencilState, CompareFunction}};
use lume_vulkan::VulkanInstance;

struct App {
    window: Option<Arc<Window>>,
    instance: Option<VulkanInstance>,
    surface: Option<lume_vulkan::VulkanSurface>,
    device: Option<lume_vulkan::VulkanDevice>,
    swapchain: Option<lume_vulkan::VulkanSwapchain>,
    render_pass: Option<lume_vulkan::VulkanRenderPass>,
    pipeline_layout: Option<lume_vulkan::VulkanPipelineLayout>,
    pipeline: Option<lume_vulkan::VulkanGraphicsPipeline>,
    shaders: Vec<lume_vulkan::VulkanShaderModule>,
    vertex_buffer: Option<lume_vulkan::VulkanBuffer>,
    uniform_buffer: Option<lume_vulkan::VulkanBuffer>,
    texture: Option<lume_vulkan::VulkanTexture>,
    texture_view: Option<lume_vulkan::VulkanTextureView>,
    sampler: Option<lume_vulkan::VulkanSampler>,
    depth_texture: Option<lume_vulkan::VulkanTexture>,
    depth_view: Option<lume_vulkan::VulkanTextureView>,
    bind_group_layout: Option<lume_vulkan::VulkanBindGroupLayout>,
    bind_group: Option<lume_vulkan::VulkanBindGroup>,
    start_time: SystemTime,
    
    command_pool: Option<lume_vulkan::VulkanCommandPool>,
    command_buffers: Vec<lume_vulkan::VulkanCommandBuffer>,
    framebuffers: Vec<lume_vulkan::VulkanFramebuffer>,
    image_available_semaphore: Option<lume_vulkan::VulkanSemaphore>,
    render_finished_semaphore: Option<lume_vulkan::VulkanSemaphore>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("LumeVisual - Backend Agnostic Cube")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            let instance_desc = InstanceDescriptor {
                name: "Textured Cube",
                backend: Backend::Vulkan,
            };
            
            let instance = VulkanInstance::new(instance_desc).expect("Failed to create Lume Instance");
            let surface = instance.create_surface(&window, &window).expect("Failed to create surface");
            let device = instance.request_device(Some(&surface)).expect("Failed to request device");

            log::info!("Creating Swapchain...");
            let size = window.inner_size();
            let swapchain = device.create_swapchain(&surface, SwapchainDescriptor {
                width: size.width,
                height: size.height,
            }).expect("Failed to create swapchain");

            log::info!("Loading Texture...");
            // Load Texture Data
            let img = image::open("lume-examples/assets/texture.jpg").expect("Failed to load texture");
            let (width, height) = img.dimensions();
            let rgba = img.to_rgba8();
            let pixels = rgba.as_raw();

            let texture = device.create_texture(TextureDescriptor {
                width,
                height,
                depth: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::TEXTURE_BINDING | TextureUsage::COPY_DST,
            }).expect("Failed to create texture");

            let staging_buffer = device.create_buffer(lume_core::device::BufferDescriptor {
                size: pixels.len() as u64,
                usage: lume_core::device::BufferUsage::COPY_SRC,
                mapped_at_creation: true,
            }).expect("Failed to create staging buffer");

            staging_buffer.write_data(0, pixels).expect("Failed to write to staging buffer");

            let texture_view = device.create_texture_view(&texture, TextureViewDescriptor {
                format: Some(TextureFormat::Rgba8Unorm),
            }).expect("Failed to create texture view");

            // Upload texture
            let command_pool = device.create_command_pool().expect("Failed to create command pool");
            let mut upload_cmd = command_pool.allocate_command_buffer().expect("Failed to allocate upload cmd");
            
            upload_cmd.begin().expect("Failed to begin upload cmd");
            upload_cmd.texture_barrier(&texture_view, ImageLayout::Undefined, ImageLayout::TransferDst);
            upload_cmd.copy_buffer_to_texture(&staging_buffer, &texture, width, height);
            upload_cmd.texture_barrier(&texture_view, ImageLayout::TransferDst, ImageLayout::ShaderReadOnly);
            upload_cmd.end().expect("Failed to end upload cmd");

            let sampler = device.create_sampler(SamplerDescriptor {
                min_filter: FilterMode::Linear,
                mag_filter: FilterMode::Linear,
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
            }).expect("Failed to create sampler");

            // Create Depth Texture
            let depth_texture = device.create_texture(TextureDescriptor {
                width: size.width,
                height: size.height,
                depth: 1,
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::DEPTH_STENCIL_ATTACHMENT,
            }).expect("Failed to create depth texture");

            let depth_view = device.create_texture_view(&depth_texture, TextureViewDescriptor {
                format: Some(TextureFormat::Depth32Float),
            }).expect("Failed to create depth view");

            // Load & Compile Shaders using Naga
            let vert_glsl = include_str!("../../shaders/triangle.vert");
            let frag_glsl = include_str!("../../shaders/textured.frag");

            let vert_spv = compile_shader(ShaderSource::Glsl {
                source: vert_glsl,
                stage: naga::ShaderStage::Vertex,
                defines: naga::FastHashMap::default(),
            }).expect("Failed to compile vertex shader");

            let frag_spv = compile_shader(ShaderSource::Glsl {
                source: frag_glsl,
                stage: naga::ShaderStage::Fragment,
                defines: naga::FastHashMap::default(),
            }).expect("Failed to compile fragment shader");

            log::info!("Compiling Shaders...");
            let vert_module = device.create_shader_module(&vert_spv).expect("Failed to create vert shader");
            let frag_module = device.create_shader_module(&frag_spv).expect("Failed to create frag shader");

            log::info!("Creating Render Pass...");

            // Create Render Pass
            let render_pass = device.create_render_pass(RenderPassDescriptor {
                color_format: TextureFormat::Bgra8UnormSrgb,
                depth_stencil_format: Some(TextureFormat::Depth32Float),
            }).expect("Failed to create render pass");

            // Create Bind Group Layout
            let bind_group_layout = device.create_bind_group_layout(BindGroupLayoutDescriptor {
                entries: vec![
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStage::VERTEX,
                        ty: BindingType::UniformBuffer,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::SampledTexture,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Sampler,
                    },
                ],
            }).expect("Failed to create bind group layout");

            log::info!("Creating Pipeline Layout...");
            let layout = device.create_pipeline_layout(PipelineLayoutDescriptor {
                bind_group_layouts: &[&bind_group_layout],
            }).expect("Failed to create layout");

            log::info!("Creating Graphics Pipeline...");

            // Create Graphics Pipeline
            let pipeline = device.create_graphics_pipeline(GraphicsPipelineDescriptor {
                vertex_shader: &vert_module,
                fragment_shader: &frag_module,
                render_pass: &render_pass,
                layout: &layout,
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                },
                vertex_layout: Some(lume_core::device::VertexLayout {
                    array_stride: 20, // (3 + 2) * 4
                    attributes: vec![
                        lume_core::device::VertexAttribute {
                            location: 0,
                            format: lume_core::device::VertexFormat::Float32x3,
                            offset: 0,
                        },
                        lume_core::device::VertexAttribute {
                            location: 1,
                            format: lume_core::device::VertexFormat::Float32x2,
                            offset: 12,
                        },
                    ],
                }),
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::Less,
                }),
            }).expect("Failed to create pipeline");

            log::info!("Pipeline created. Creating Vertex Buffer...");
            let vertices: [f32; 180] = [
                // Front face
                -0.5, -0.5,  0.5, 0.0, 0.0,
                 0.5, -0.5,  0.5, 1.0, 0.0,
                 0.5,  0.5,  0.5, 1.0, 1.0,
                -0.5, -0.5,  0.5, 0.0, 0.0,
                 0.5,  0.5,  0.5, 1.0, 1.0,
                -0.5,  0.5,  0.5, 0.0, 1.0,
                // Back face
                -0.5, -0.5, -0.5, 0.0, 0.0,
                -0.5,  0.5, -0.5, 0.0, 1.0,
                 0.5,  0.5, -0.5, 1.0, 1.0,
                -0.5, -0.5, -0.5, 0.0, 0.0,
                 0.5,  0.5, -0.5, 1.0, 1.0,
                 0.5, -0.5, -0.5, 1.0, 0.0,
                // Left face
                -0.5,  0.5,  0.5, 1.0, 0.0,
                -0.5,  0.5, -0.5, 1.0, 1.0,
                -0.5, -0.5, -0.5, 0.0, 1.0,
                -0.5,  0.5,  0.5, 1.0, 0.0,
                -0.5, -0.5, -0.5, 0.0, 1.0,
                -0.5, -0.5,  0.5, 0.0, 0.0,
                // Right face
                 0.5,  0.5,  0.5, 1.0, 0.0,
                 0.5, -0.5,  0.5, 0.0, 0.0,
                 0.5, -0.5, -0.5, 0.0, 1.0,
                 0.5,  0.5,  0.5, 1.0, 0.0,
                 0.5, -0.5, -0.5, 0.0, 1.0,
                 0.5,  0.5, -0.5, 1.0, 1.0,
                // Top face
                -0.5,  0.5, -0.5, 0.0, 1.0,
                -0.5,  0.5,  0.5, 0.0, 0.0,
                 0.5,  0.5,  0.5, 1.0, 0.0,
                -0.5,  0.5, -0.5, 0.0, 1.0,
                 0.5,  0.5,  0.5, 1.0, 0.0,
                 0.5,  0.5, -0.5, 1.0, 1.0,
                // Bottom face
                -0.5, -0.5, -0.5, 0.0, 1.0,
                 0.5, -0.5, -0.5, 1.0, 1.0,
                 0.5, -0.5,  0.5, 1.0, 0.0,
                -0.5, -0.5, -0.5, 0.0, 1.0,
                 0.5, -0.5,  0.5, 1.0, 0.0,
                -0.5, -0.5,  0.5, 0.0, 0.0,
            ];

            let vertex_buffer = device.create_buffer(lume_core::device::BufferDescriptor {
                size: (vertices.len() * 4) as u64,
                usage: lume_core::device::BufferUsage::VERTEX,
                mapped_at_creation: true,
            }).expect("Failed to create vertex buffer");

            log::info!("Vertex Buffer created. Writing data...");
            vertex_buffer.write_data(0, unsafe {
                std::slice::from_raw_parts(vertices.as_ptr() as *const u8, vertices.len() * 4)
            }).expect("Failed to write vertex data");

            log::info!("Vertex data written. Creating Uniform Buffer...");
            let uniform_buffer = device.create_buffer(lume_core::device::BufferDescriptor {
                size: 64, // 4x4 matrix
                usage: lume_core::device::BufferUsage::UNIFORM,
                mapped_at_creation: true,
            }).expect("Failed to create uniform buffer");

            // Create Bind Group
            let bind_group = device.create_bind_group(BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: vec![
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(&uniform_buffer),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&texture_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&sampler),
                    },
                ],
            }).expect("Failed to create bind group");

            // Create Command Pool
            let command_pool = device.create_command_pool().expect("Failed to create command pool");

            // Create Framebuffers and Command Buffers
            let mut framebuffers = Vec::new();
            let mut command_buffers = Vec::new();

            for i in 0..3 { 
                let view = swapchain.get_view(i as u32);
                let framebuffer = device.create_framebuffer(FramebufferDescriptor {
                    render_pass: &render_pass,
                    attachments: &[view, &depth_view],
                    width: size.width,
                    height: size.height,
                }).expect("Failed to create framebuffer");
                
                let mut cmd = command_pool.allocate_command_buffer().expect("Failed to allocate command buffer");
                
                cmd.begin().expect("Failed to begin command buffer");
                cmd.begin_render_pass(&render_pass, &framebuffer, [0.1, 0.2, 0.3, 1.0]);
                cmd.bind_graphics_pipeline(&pipeline);
                cmd.bind_vertex_buffer(&vertex_buffer);
                cmd.bind_bind_group(0, &bind_group);
                cmd.set_viewport(0.0, 0.0, size.width as f32, size.height as f32);
                cmd.set_scissor(0, 0, size.width, size.height);
                cmd.draw(36, 1, 0, 0); 
                cmd.end_render_pass();
                cmd.end().expect("Failed to end command buffer");

                framebuffers.push(framebuffer);
                command_buffers.push(cmd);
            }

            let image_available_semaphore = device.create_semaphore().expect("Failed to create semaphore");
            let render_finished_semaphore = device.create_semaphore().expect("Failed to create semaphore");

            self.instance = Some(instance);
            self.surface = Some(surface);
            self.device = Some(device);
            self.swapchain = Some(swapchain);
            self.render_pass = Some(render_pass);
            self.pipeline_layout = Some(layout);
            self.pipeline = Some(pipeline);
            self.shaders = vec![vert_module, frag_module];
            self.vertex_buffer = Some(vertex_buffer);
            self.uniform_buffer = Some(uniform_buffer);
            self.texture = Some(texture);
            self.texture_view = Some(texture_view);
            self.sampler = Some(sampler);
            self.depth_texture = Some(depth_texture);
            self.depth_view = Some(depth_view);
            self.bind_group_layout = Some(bind_group_layout);
            self.bind_group = Some(bind_group);
            self.command_pool = Some(command_pool);
            self.command_buffers = command_buffers;
            self.framebuffers = framebuffers;
            self.image_available_semaphore = Some(image_available_semaphore);
            self.render_finished_semaphore = Some(render_finished_semaphore);

            log::info!("Backend Agnostic Cube initialized successfully!");
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                if let Some(device) = &self.device {
                    log::info!("Waiting for GPU idle before shutdown...");
                    let _ = device.wait_idle();
                }
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                if let (Some(device), Some(swapchain)) = (
                    &self.device,
                    self.swapchain.as_mut(),
                ) {
                    // 1. Begin Frame (Handles all Fence/Semaphore sync internally)
                    let token = device.begin_frame(swapchain).expect("Failed to begin frame");

                    // 2. Update Uniforms (MVP)
                    let now = SystemTime::now();
                    let duration = now.duration_since(self.start_time).unwrap();
                    let time = duration.as_secs_f32();
                    
                    let size = self.window.as_ref().unwrap().inner_size();
                    let aspect = size.width as f32 / size.height as f32;
                    
                    let mut proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 100.0);
                    proj.col_mut(1).y *= -1.0; 
                    
                    let view = Mat4::look_at_rh(Vec3::new(2.0, 2.0, 2.0), Vec3::ZERO, Vec3::Z);
                    let model = Mat4::from_rotation_z(time);
                    
                    let mvp = proj * view * model;
                    let mvp_bytes: [f32; 16] = mvp.to_cols_array();

                    self.uniform_buffer.as_ref().unwrap().write_data(0, unsafe {
                        std::slice::from_raw_parts(mvp_bytes.as_ptr() as *const u8, 64)
                    }).expect("Failed to update uniform buffer");

                    // 3. Select Command Buffer and End Frame
                    // Note: In a real engine, you'd record commands here or use pre-recorded ones
                    let cmd = &self.command_buffers[token.image_index as usize];
                    device.end_frame(swapchain, token, &[cmd]).expect("Failed to end frame");
                }
            }
            _ => (),
        }
        
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();
    log::info!("Starting Hello Triangle Example (Backend Agnostic)");

    let event_loop = EventLoop::new().unwrap();
    let mut app = App { 
        window: None, 
        instance: None, 
        surface: None, 
        device: None, 
        swapchain: None,
        render_pass: None,
        pipeline_layout: None,
        pipeline: None,
        shaders: Vec::new(),
        vertex_buffer: None,
        uniform_buffer: None,
        texture: None,
        texture_view: None,
        sampler: None,
        depth_texture: None,
        depth_view: None,
        bind_group_layout: None,
        bind_group: None,
        start_time: SystemTime::now(),
        command_pool: None,
        command_buffers: Vec::new(),
        framebuffers: Vec::new(),
        image_available_semaphore: None,
        render_finished_semaphore: None,
    };
    event_loop.run_app(&mut app).unwrap();
}
