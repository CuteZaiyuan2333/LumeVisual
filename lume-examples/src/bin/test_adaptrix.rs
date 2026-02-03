use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use lume_core::{Instance, InstanceDescriptor, Backend, Device, device::*};
use lume_vulkan::VulkanInstance;
use std::sync::Arc;
use lume_adaptrix::{AdaptrixVertex, ClusterPacked, AdaptrixAsset};
use glam::{Mat4, Vec3, Vec4};

struct AdaptrixApp {
    window: Option<Arc<Window>>,
    instance: Option<VulkanInstance>,
    surface: Option<lume_vulkan::VulkanSurface>,
    device: Option<lume_vulkan::VulkanDevice>,
    swapchain: Option<lume_vulkan::VulkanSwapchain>,
    
    asset: Option<AdaptrixAsset>,

    cluster_buffer: Option<lume_vulkan::VulkanBuffer>,
    vertex_buffer: Option<lume_vulkan::VulkanBuffer>,
    vertex_index_buffer: Option<lume_vulkan::VulkanBuffer>,
    primitive_index_buffer: Option<lume_vulkan::VulkanBuffer>,
    visible_clusters_buffer: Option<lume_vulkan::VulkanBuffer>,
    visible_count_buffer: Option<lume_vulkan::VulkanBuffer>,
    zero_buffer: Option<lume_vulkan::VulkanBuffer>,
    view_buffer: Option<lume_vulkan::VulkanBuffer>,

    cull_pipeline: Option<lume_vulkan::VulkanComputePipeline>,
    cull_layout: Option<lume_vulkan::VulkanPipelineLayout>,
    cull_bind_group_0: Option<lume_vulkan::VulkanBindGroup>,
    cull_bind_group_1: Option<lume_vulkan::VulkanBindGroup>,

    vis_pipeline: Option<lume_vulkan::VulkanGraphicsPipeline>,
    vis_layout: Option<lume_vulkan::VulkanPipelineLayout>,
    vis_bind_group_0: Option<lume_vulkan::VulkanBindGroup>,
    vis_bind_group_1: Option<lume_vulkan::VulkanBindGroup>,

    resolve_pipeline: Option<lume_vulkan::VulkanGraphicsPipeline>,
    resolve_layout: Option<lume_vulkan::VulkanPipelineLayout>,
    resolve_bind_group_0: Option<lume_vulkan::VulkanBindGroup>,
    resolve_bind_group_1: Option<lume_vulkan::VulkanBindGroup>,

    vis_buffer_texture: Option<lume_vulkan::VulkanTexture>,
    vis_buffer_view: Option<lume_vulkan::VulkanTextureView>,
    vis_depth_texture: Option<lume_vulkan::VulkanTexture>,
    vis_depth_view: Option<lume_vulkan::VulkanTextureView>,
    
    vis_render_pass: Option<lume_vulkan::VulkanRenderPass>,
    vis_framebuffer: Option<lume_vulkan::VulkanFramebuffer>,
    resolve_render_pass: Option<lume_vulkan::VulkanRenderPass>,
    resolve_framebuffers: Vec<lume_vulkan::VulkanFramebuffer>,

    command_pool: Option<lume_vulkan::VulkanCommandPool>,
    command_buffer: Option<lume_vulkan::VulkanCommandBuffer>,
    start_time: std::time::Instant,
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform {
    view_proj: [Vec4; 4],
    inv_view_proj: [Vec4; 4],
    camera_pos_and_threshold: Vec4,
    viewport_size: Vec4,
}

impl AdaptrixApp {
    fn new() -> Self {
        let path = if std::path::Path::new("facade.lad").exists() { "facade.lad" } 
                  else if std::path::Path::new("猴头Atest.lad").exists() { "猴头Atest.lad" } 
                  else { "test.lad" };
        println!("Mmap Loading: {}", path);
        let start = std::time::Instant::now();
        let asset = AdaptrixAsset::load_from_file(path).expect("Failed to mmap lad file");
        println!("Mmap Loaded in {:.2}ms: {} clusters, {} vertices", 
                 start.elapsed().as_secs_f32() * 1000.0, asset.clusters.len(), asset.vertices.len());
        
        Self {
            window: None, instance: None, surface: None, device: None, swapchain: None,
            asset: Some(asset),
            cluster_buffer: None, vertex_buffer: None, vertex_index_buffer: None, primitive_index_buffer: None,
            visible_clusters_buffer: None, visible_count_buffer: None, zero_buffer: None, view_buffer: None,
            cull_pipeline: None, cull_layout: None, cull_bind_group_0: None, cull_bind_group_1: None,
            vis_pipeline: None, vis_layout: None, vis_bind_group_0: None, vis_bind_group_1: None,
            resolve_pipeline: None, resolve_layout: None, resolve_bind_group_0: None, resolve_bind_group_1: None,
            vis_buffer_texture: None, vis_buffer_view: None, vis_depth_texture: None, vis_depth_view: None,
            vis_render_pass: None, vis_framebuffer: None, resolve_render_pass: None, resolve_framebuffers: Vec::new(),
            command_pool: None, command_buffer: None, start_time: std::time::Instant::now(),
        }
    }

    fn setup_gpu_resources(&mut self) {
        let device = self.device.as_ref().unwrap();
        let size = self.window.as_ref().unwrap().inner_size();
        let asset = self.asset.as_ref().unwrap();
        
        self.cluster_buffer = Some(device.create_buffer(BufferDescriptor { size: (asset.clusters.len() * 48) as u64, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.cluster_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(asset.clusters)).unwrap();
        self.vertex_buffer = Some(device.create_buffer(BufferDescriptor { size: (asset.vertices.len() * 32) as u64, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.vertex_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(asset.vertices)).unwrap();
        self.vertex_index_buffer = Some(device.create_buffer(BufferDescriptor { size: (asset.meshlet_vertex_indices.len() * 4) as u64, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.vertex_index_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(asset.meshlet_vertex_indices)).unwrap();
        self.primitive_index_buffer = Some(device.create_buffer(BufferDescriptor { size: asset.meshlet_primitive_indices.len() as u64, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.primitive_index_buffer.as_ref().unwrap().write_data(0, asset.meshlet_primitive_indices).unwrap();
        
        self.visible_clusters_buffer = Some(device.create_buffer(BufferDescriptor { size: (asset.clusters.len() * 8).max(2048 * 1024) as u64, usage: BufferUsage::STORAGE, mapped_at_creation: true }).unwrap());
        self.visible_count_buffer = Some(device.create_buffer(BufferDescriptor { size: 16, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST | BufferUsage::COPY_SRC | BufferUsage::INDIRECT, mapped_at_creation: true }).unwrap());
        self.visible_count_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(&[372u32, 0, 0, 0])).unwrap();
        
        let zero_buffer = device.create_buffer(BufferDescriptor {
            size: 16,
            usage: BufferUsage::COPY_SRC,
            mapped_at_creation: true,
        }).unwrap();
        // Correct layout for DrawArgs: vertexCount=372, instanceCount=0, firstVertex=0, firstInstance=0
        zero_buffer.write_data(0, bytemuck::cast_slice(&[372u32, 0, 0, 0])).unwrap();
        self.zero_buffer = Some(zero_buffer);
        self.view_buffer = Some(device.create_buffer(BufferDescriptor { size: 160, usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());

        self.vis_buffer_texture = Some(device.create_texture(TextureDescriptor { width: size.width, height: size.height, depth: 1, format: TextureFormat::Rg32Uint, usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING }).unwrap());
        self.vis_buffer_view = Some(device.create_texture_view(self.vis_buffer_texture.as_ref().unwrap(), TextureViewDescriptor { format: None }).unwrap());
        self.vis_depth_texture = Some(device.create_texture(TextureDescriptor { width: size.width, height: size.height, depth: 1, format: TextureFormat::Depth32Float, usage: TextureUsage::DEPTH_STENCIL_ATTACHMENT }).unwrap());
        self.vis_depth_view = Some(device.create_texture_view(self.vis_depth_texture.as_ref().unwrap(), TextureViewDescriptor { format: None }).unwrap());

        let cull_module = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/cull.wgsl"))).unwrap()).unwrap();
        let vis_v_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/visbuffer.vert.wgsl"))).unwrap()).unwrap();
        let vis_f_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/visbuffer.frag.wgsl"))).unwrap()).unwrap();
        let res_v_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/resolve.vert.wgsl"))).unwrap()).unwrap();
        let res_f_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/resolve.frag.wgsl"))).unwrap()).unwrap();

        // Cull
        let bgl_c0 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 5, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
        ] }).unwrap();
        let bgl_c1 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::UniformBuffer }] }).unwrap();
        let l_cull = device.create_pipeline_layout(PipelineLayoutDescriptor { 
            bind_group_layouts: &[&bgl_c0, &bgl_c1],
            push_constant_ranges: &[],
        }).unwrap();
        self.cull_pipeline = Some(device.create_compute_pipeline(ComputePipelineDescriptor { shader: &cull_module, layout: &l_cull }).unwrap());
        self.cull_bind_group_0 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_c0, entries: vec![
            BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.cluster_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.visible_clusters_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 5, resource: BindingResource::Buffer(self.visible_count_buffer.as_ref().unwrap()) },
        ] }).unwrap());
        self.cull_bind_group_1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_c1, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.view_buffer.as_ref().unwrap()) }] }).unwrap());
        self.cull_layout = Some(l_cull);

        // Vis
        let vis_rp = device.create_render_pass(RenderPassDescriptor { color_format: TextureFormat::Rg32Uint, depth_stencil_format: Some(TextureFormat::Depth32Float) }).unwrap();
        let bgl_v0 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 3, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 4, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 5, visibility: ShaderStage::VERTEX, ty: BindingType::StorageBuffer },
        ] }).unwrap();
        let bgl_v1 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::VERTEX, ty: BindingType::UniformBuffer }] }).unwrap();
        let l_vis = device.create_pipeline_layout(PipelineLayoutDescriptor { 
            bind_group_layouts: &[&bgl_v0, &bgl_v1],
            push_constant_ranges: &[],
        }).unwrap();
        self.vis_pipeline = Some(device.create_graphics_pipeline(GraphicsPipelineDescriptor { vertex_shader: &vis_v_mod, fragment_shader: &vis_f_mod, render_pass: &vis_rp, layout: &l_vis, primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList, cull_mode: CullMode::None }, vertex_layout: None, depth_stencil: Some(DepthStencilState { format: TextureFormat::Depth32Float, depth_write_enabled: true, depth_compare: CompareFunction::LessEqual }) }).unwrap());
        self.vis_bind_group_0 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_v0, entries: vec![
            BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.cluster_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.vertex_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.vertex_index_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 3, resource: BindingResource::Buffer(self.visible_clusters_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 4, resource: BindingResource::Buffer(self.primitive_index_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 5, resource: BindingResource::Buffer(self.visible_count_buffer.as_ref().unwrap()) },
        ] }).unwrap());
        self.vis_bind_group_1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_v1, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.view_buffer.as_ref().unwrap()) }] }).unwrap());
        self.vis_framebuffer = Some(device.create_framebuffer(FramebufferDescriptor { render_pass: &vis_rp, attachments: &[self.vis_buffer_view.as_ref().unwrap(), self.vis_depth_view.as_ref().unwrap()], width: size.width, height: size.height }).unwrap());
        self.vis_render_pass = Some(vis_rp);
        self.vis_layout = Some(l_vis);

        // Resolve
        let res_rp = device.create_render_pass(RenderPassDescriptor { color_format: TextureFormat::Bgra8UnormSrgb, depth_stencil_format: None }).unwrap();
        for i in 0..3 { self.resolve_framebuffers.push(device.create_framebuffer(FramebufferDescriptor { render_pass: &res_rp, attachments: &[self.swapchain.as_ref().unwrap().get_view(i as u32)], width: size.width, height: size.height }).unwrap()); }
        let bgl_r0 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 3, visibility: ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer },
        ] }).unwrap();
        let bgl_r1 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::FRAGMENT, ty: BindingType::UniformBuffer }, BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::FRAGMENT, ty: BindingType::SampledTexture }] }).unwrap();
        let l_res = device.create_pipeline_layout(PipelineLayoutDescriptor { 
            bind_group_layouts: &[&bgl_r0, &bgl_r1],
            push_constant_ranges: &[],
        }).unwrap();
        self.resolve_pipeline = Some(device.create_graphics_pipeline(GraphicsPipelineDescriptor { vertex_shader: &res_v_mod, fragment_shader: &res_f_mod, render_pass: &res_rp, layout: &l_res, primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList, cull_mode: CullMode::None }, vertex_layout: None, depth_stencil: None }).unwrap());
        self.resolve_bind_group_0 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_r0, entries: vec![
            BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.cluster_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.vertex_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.vertex_index_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 3, resource: BindingResource::Buffer(self.primitive_index_buffer.as_ref().unwrap()) },
        ] }).unwrap());
        self.resolve_bind_group_1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_r1, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.view_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 1, resource: BindingResource::TextureView(self.vis_buffer_view.as_ref().unwrap()) }] }).unwrap());
        self.resolve_render_pass = Some(res_rp);
        self.resolve_layout = Some(l_res);

        self.command_pool = Some(device.create_command_pool().unwrap());
        self.command_buffer = Some(self.command_pool.as_ref().unwrap().allocate_command_buffer().unwrap());
    }
}

impl ApplicationHandler for AdaptrixApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("LumeVisual - Nanite Master Load").with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))).unwrap());
            let instance = VulkanInstance::new(InstanceDescriptor { name: "Nanite", backend: Backend::Vulkan }).unwrap();
            let surface = instance.create_surface(&window, &window).unwrap();
            let device = instance.request_device(Some(&surface)).unwrap();
            let swapchain = device.create_swapchain(&surface, SwapchainDescriptor { width: 1280, height: 720 }).unwrap();
            self.window = Some(window); self.instance = Some(instance); self.surface = Some(surface); self.device = Some(device); self.swapchain = Some(swapchain);
            self.setup_gpu_resources();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let (Some(device), Some(swapchain)) = (&self.device, self.swapchain.as_mut()) {
                    let token = device.begin_frame(swapchain).unwrap();
                    let elapsed = self.start_time.elapsed().as_secs_f32();
                    
                    let cam_pos = Vec3::new(elapsed.cos() * 4.0, 1.0, elapsed.sin() * 4.0);
                    let view_mat = Mat4::look_at_rh(cam_pos, Vec3::ZERO, Vec3::Y);
                    let mut proj = Mat4::perspective_rh(0.785, 1280.0/720.0, 0.01, 1000.0); proj.col_mut(1).y *= -1.0;
                    let vp = proj * view_mat;
                    let inv_vp = vp.inverse();
                    
                    self.view_buffer.as_ref().unwrap().write_data(0, bytemuck::bytes_of(&ViewUniform {
                        view_proj: [vp.col(0), vp.col(1), vp.col(2), vp.col(3)],
                        inv_view_proj: [inv_vp.col(0), inv_vp.col(1), inv_vp.col(2), inv_vp.col(3)],
                        camera_pos_and_threshold: glam::vec4(cam_pos.x, cam_pos.y, cam_pos.z, 1.5), 
                        viewport_size: glam::vec4(1280.0, 720.0, 0.0, 0.0),
                    })).unwrap();

                    // 偶尔读取一下 GPU 计数用于调试
                    static mut FRAME_COUNT: u32 = 0;
                    unsafe {
                        FRAME_COUNT += 1;
                        if FRAME_COUNT % 60 == 0 {
                            let _ = device.wait_idle(); 
                            let mut counts = [0u32; 4];
                            let _ = self.visible_count_buffer.as_ref().unwrap().read_data(0, bytemuck::cast_slice_mut(&mut counts));
                            let total = self.asset.as_ref().unwrap().clusters.len();
                            println!("Rendered: {} / {} clusters ({:.1}%)", 
                                     counts[1], total, (counts[1] as f32 / total as f32) * 100.0); 
                        }
                    }

                    let cmd = self.command_buffer.as_mut().unwrap();
                    cmd.reset().unwrap(); cmd.begin().unwrap();
                    
                    cmd.copy_buffer_to_buffer(self.zero_buffer.as_ref().unwrap(), self.visible_count_buffer.as_ref().unwrap(), 16);
                    cmd.compute_barrier();

                    cmd.bind_compute_pipeline(self.cull_pipeline.as_ref().unwrap());
                    cmd.bind_bind_group(0, self.cull_bind_group_0.as_ref().unwrap());
                    cmd.bind_bind_group(1, self.cull_bind_group_1.as_ref().unwrap());
                    cmd.dispatch((self.asset.as_ref().unwrap().clusters.len() as u32 + 63) / 64, 1, 1);
                    cmd.compute_barrier();

                    cmd.texture_barrier(self.vis_buffer_view.as_ref().unwrap(), ImageLayout::Undefined, ImageLayout::ColorAttachment);
                    cmd.begin_render_pass(self.vis_render_pass.as_ref().unwrap(), self.vis_framebuffer.as_ref().unwrap(), [0.0, 0.0, 0.0, 0.0]);
                    cmd.set_viewport(0.0, 0.0, 1280.0, 720.0); cmd.set_scissor(0, 0, 1280, 720);
                    cmd.bind_graphics_pipeline(self.vis_pipeline.as_ref().unwrap());
                    cmd.bind_bind_group(0, self.vis_bind_group_0.as_ref().unwrap());
                    cmd.bind_bind_group(1, self.vis_bind_group_1.as_ref().unwrap());
                    cmd.draw_indirect(self.visible_count_buffer.as_ref().unwrap(), 0, 1, 16);
                    cmd.end_render_pass();

                    cmd.texture_barrier(self.vis_buffer_view.as_ref().unwrap(), ImageLayout::ColorAttachment, ImageLayout::ShaderReadOnly);
                    cmd.begin_render_pass(self.resolve_render_pass.as_ref().unwrap(), &self.resolve_framebuffers[token.image_index as usize], [0.02, 0.02, 0.03, 1.0]);
                    cmd.bind_graphics_pipeline(self.resolve_pipeline.as_ref().unwrap());
                    cmd.bind_bind_group(0, self.resolve_bind_group_0.as_ref().unwrap());
                    cmd.bind_bind_group(1, self.resolve_bind_group_1.as_ref().unwrap());
                    cmd.draw(3, 1, 0, 0);
                    cmd.end_render_pass();

                    cmd.texture_barrier(swapchain.get_view(token.image_index), ImageLayout::ColorAttachment, ImageLayout::Present);
                    cmd.end().unwrap();
                    device.end_frame(swapchain, token, &[cmd]).unwrap();
                }
            }
            _ => (),
        }
        self.window.as_ref().unwrap().request_redraw();
    }
}
fn main() { env_logger::init(); let mut app = AdaptrixApp::new(); EventLoop::new().unwrap().run_app(&mut app).unwrap(); }