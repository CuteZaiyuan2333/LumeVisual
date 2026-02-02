use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use lume_core::{
    Instance, InstanceDescriptor, Backend, Device, 
    device::*,
};
use lume_vulkan::{VulkanInstance, VulkanDevice};
use lume_adaptrix::{AdaptrixFlatAsset, renderer::{AdaptrixMeshGPU, AdaptrixRenderer}};
use std::fs::File;
use std::io::{BufReader, Read};
use glam::{Mat4, Vec3};

struct App {
    window: Option<Arc<Window>>,
    instance: Option<VulkanInstance>,
    device: Option<VulkanDevice>,
    surface: Option<lume_vulkan::VulkanSurface>,
    swapchain: Option<lume_vulkan::VulkanSwapchain>,
    command_pool: Option<lume_vulkan::VulkanCommandPool>,
    command_buffers: Vec<lume_vulkan::VulkanCommandBuffer>,
    mesh_gpu: Option<AdaptrixMeshGPU<VulkanDevice>>,
    renderer: Option<AdaptrixRenderer<VulkanDevice>>,
    depth_view: Option<lume_vulkan::VulkanTextureView>,
    vis_view: Option<lume_vulkan::VulkanTextureView>,
    vis_bind_group: Option<lume_vulkan::VulkanBindGroup>,
    resolve_bind_group: Option<lume_vulkan::VulkanBindGroup>,
    uniform_buffer: Option<lume_vulkan::VulkanBuffer>,
    vis_pass: Option<lume_vulkan::VulkanRenderPass>,
    vis_framebuffer: Option<lume_vulkan::VulkanFramebuffer>,
    resolve_pass: Option<lume_vulkan::VulkanRenderPass>,
    resolve_fbs: Vec<lume_vulkan::VulkanFramebuffer>,
    start_time: std::time::Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        eprintln!("DEBUG: Resuming App...");

        let window_attrs = Window::default_attributes()
            .with_title("Lume Adaptrix - Debug Step 8")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
        self.window = Some(window.clone());

        let instance = VulkanInstance::new(InstanceDescriptor { name: "Demo", backend: Backend::Vulkan }).unwrap();
        let surface = instance.create_surface(&window, &window).unwrap();
        let device = instance.request_device(Some(&surface)).unwrap();
        let size = window.inner_size();
        let swapchain = device.create_swapchain(&surface, SwapchainDescriptor { width: size.width, height: size.height }).unwrap();

        let command_pool = device.create_command_pool().unwrap();
        let command_buffers = vec![command_pool.allocate_command_buffer().unwrap()];

        let file = File::open("test.lad").expect("test.lad not found!");
        let asset: AdaptrixFlatAsset = bincode::deserialize_from(BufReader::new(file)).unwrap();
        let mesh_gpu = AdaptrixMeshGPU::new(&device, &asset).unwrap();

        let uniform_buffer = device.create_buffer(BufferDescriptor { size: 64, usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap();

        let depth_texture = device.create_texture(TextureDescriptor { width: size.width, height: size.height, depth: 1, format: TextureFormat::Depth32Float, usage: TextureUsage::DEPTH_STENCIL_ATTACHMENT }).unwrap();
        let depth_view = device.create_texture_view(&depth_texture, TextureViewDescriptor { format: None }).unwrap();

        let vis_texture = device.create_texture(TextureDescriptor { width: size.width, height: size.height, depth: 1, format: TextureFormat::Rg32Uint, usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING }).unwrap();
        let vis_view = device.create_texture_view(&vis_texture, TextureViewDescriptor { format: None }).unwrap();

        // Pass 1 Layout
        let vis_bg_layout = device.create_bind_group_layout(BindGroupLayoutDescriptor {
            entries: vec![
                BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
                BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
                BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
                BindGroupLayoutEntry { binding: 3, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
                BindGroupLayoutEntry { binding: 4, visibility: ShaderStage::VERTEX, ty: BindingType::UniformBuffer },
            ],
        }).unwrap();
        let vis_layout = device.create_pipeline_layout(PipelineLayoutDescriptor { bind_group_layouts: &[&vis_bg_layout] }).unwrap();

        // Pass 2 Layout
        let res_bg_layout = device.create_bind_group_layout(BindGroupLayoutDescriptor {
            entries: vec![
                BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::FRAGMENT, ty: BindingType::SampledTexture },
            ],
        }).unwrap();
        let res_layout = device.create_pipeline_layout(PipelineLayoutDescriptor { bind_group_layouts: &[&res_bg_layout] }).unwrap();

        let vis_pass = device.create_render_pass(RenderPassDescriptor { color_format: TextureFormat::Rg32Uint, depth_stencil_format: Some(TextureFormat::Depth32Float) }).unwrap();
        let vis_framebuffer = device.create_framebuffer(FramebufferDescriptor { render_pass: &vis_pass, attachments: &[&vis_view, &depth_view], width: size.width, height: size.height }).unwrap();

        let resolve_pass = device.create_render_pass(RenderPassDescriptor { color_format: TextureFormat::Bgra8UnormSrgb, depth_stencil_format: None }).unwrap();
        let mut resolve_fbs = Vec::new();
        for i in 0..3 { resolve_fbs.push(device.create_framebuffer(FramebufferDescriptor { render_pass: &resolve_pass, attachments: &[swapchain.get_view(i)], width: size.width, height: size.height }).unwrap()); }

        let vis_bind_group = device.create_bind_group(BindGroupDescriptor {
            layout: &vis_bg_layout,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::Buffer(&mesh_gpu.cluster_buffer) },
                BindGroupEntry { binding: 1, resource: BindingResource::Buffer(&mesh_gpu.vertex_buffer) },
                BindGroupEntry { binding: 2, resource: BindingResource::Buffer(&mesh_gpu.vertex_index_buffer) },
                BindGroupEntry { binding: 3, resource: BindingResource::Buffer(&mesh_gpu.primitive_index_buffer) },
                BindGroupEntry { binding: 4, resource: BindingResource::Buffer(&uniform_buffer) },
            ],
        }).unwrap();

        let resolve_bind_group = device.create_bind_group(BindGroupDescriptor {
            layout: &res_bg_layout,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&vis_view) },
            ],
        }).unwrap();

        let renderer = AdaptrixRenderer::new(
            &device, &load_spv("lume-examples/shaders/cluster_cull.comp.spv"),
            &load_spv("lume-examples/shaders/visbuffer.vert.spv"), &load_spv("lume-examples/shaders/visbuffer.frag.spv"),
            &load_spv("lume-examples/shaders/fullscreen.vert.spv"), &load_spv("lume-examples/shaders/visbuffer_resolve.frag.spv"),
            vis_layout, res_layout, &vis_pass, &resolve_pass,
        ).unwrap();

        self.instance = Some(instance); self.device = Some(device); self.surface = Some(surface); self.swapchain = Some(swapchain);
        self.command_pool = Some(command_pool); self.command_buffers = command_buffers;
        self.mesh_gpu = Some(mesh_gpu); self.renderer = Some(renderer);
        self.depth_view = Some(depth_view); self.vis_view = Some(vis_view);
        self.vis_bind_group = Some(vis_bind_group); self.resolve_bind_group = Some(resolve_bind_group);
        self.uniform_buffer = Some(uniform_buffer);
        self.vis_pass = Some(vis_pass); self.vis_framebuffer = Some(vis_framebuffer);
        self.resolve_pass = Some(resolve_pass); self.resolve_fbs = resolve_fbs;
        eprintln!("DEBUG: Setup Complete.");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                if let Some(device) = &self.device { let _ = device.wait_idle(); }
                event_loop.exit()
            },
            winit::event::WindowEvent::RedrawRequested => {
                if let (Some(device), Some(renderer)) = (&self.device, &self.renderer) {
                    let swapchain = self.swapchain.as_mut().unwrap();
                    
                    eprintln!("TR: begin_frame");
                    let token = device.begin_frame(swapchain).unwrap();
                    let cmd = &mut self.command_buffers[0];
                    
                    eprintln!("TR: update_uniforms");
                    let mvp_array = [0.0f32; 16]; 
                    self.uniform_buffer.as_ref().unwrap().write_data(0, bytemuck::bytes_of(&mvp_array)).unwrap();

                    eprintln!("TR: reset");
                    cmd.reset().unwrap();
                    eprintln!("TR: begin");
                    cmd.begin().unwrap();

                    eprintln!("TR: Pass 1 Barrier");
                    cmd.texture_barrier(self.vis_view.as_ref().unwrap(), ImageLayout::Undefined, ImageLayout::ColorAttachment);
                    eprintln!("TR: Pass 1 Begin");
                    cmd.begin_render_pass(self.vis_pass.as_ref().unwrap(), self.vis_framebuffer.as_ref().unwrap(), [1.0, 1.0, 1.0, 1.0]);
                    
                    eprintln!("TR: Pass 1 Bind");
                    cmd.bind_graphics_pipeline(&renderer.visbuffer_pipeline);
                    cmd.bind_bind_group(0, self.vis_bind_group.as_ref().unwrap());
                    
                    eprintln!("TR: Pass 1 Draw");
                    let cluster_count = self.mesh_gpu.as_ref().unwrap().cluster_count;
                    cmd.draw(cluster_count * 372, 1, 0, 0); 
                    
                    eprintln!("TR: Pass 1 End");
                    cmd.end_render_pass();

                    eprintln!("TR: Pass 2 Barrier");
                    cmd.texture_barrier(self.vis_view.as_ref().unwrap(), ImageLayout::ColorAttachment, ImageLayout::ShaderReadOnly);
                    let fb = &self.resolve_fbs[token.image_index as usize];
                    eprintln!("TR: Pass 2 Begin");
                    cmd.begin_render_pass(self.resolve_pass.as_ref().unwrap(), fb, [0.1, 0.1, 0.1, 1.0]);
                    eprintln!("TR: Pass 2 Bind");
                    cmd.bind_graphics_pipeline(&renderer.resolve_pipeline);
                    cmd.bind_bind_group(0, self.resolve_bind_group.as_ref().unwrap());
                    eprintln!("TR: Pass 2 Draw");
                    cmd.draw(3, 1, 0, 0);
                    eprintln!("TR: Pass 2 End");
                    cmd.end_render_pass();

                    eprintln!("TR: Final Barrier");
                    cmd.texture_barrier(swapchain.get_view(token.image_index), ImageLayout::ColorAttachment, ImageLayout::Present);
                    eprintln!("TR: end");
                    cmd.end().unwrap();
                    eprintln!("TR: end_frame");
                    device.end_frame(swapchain, token, &[cmd]).unwrap();
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

fn load_spv(path: &str) -> Vec<u32> {
    let mut file = File::open(path).expect(&format!("MISSING SHADER: {}", path));
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    data.chunks_exact(4).map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        window: None, instance: None, device: None, surface: None, swapchain: None,
        command_pool: None, command_buffers: Vec::new(), mesh_gpu: None, renderer: None,
        depth_view: None, vis_view: None, vis_bind_group: None, resolve_bind_group: None,
        uniform_buffer: None, vis_pass: None, vis_framebuffer: None,
        resolve_pass: None, resolve_fbs: Vec::new(),
        start_time: std::time::Instant::now(),
    };
    event_loop.run_app(&mut app).unwrap();
}
