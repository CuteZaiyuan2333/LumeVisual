use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use lume_core::{Instance, InstanceDescriptor, Backend, Device, device::*};
use lume_vulkan::VulkanInstance;
use std::sync::Arc;
use std::fs::File;
use std::io::Read;
use lume_adaptrix::{AdaptrixVertex, ClusterPacked};
use bytemuck::{self, Zeroable};
use glam::{Mat4, Vec4, Vec3};

struct AdaptrixApp {
    window: Option<Arc<Window>>,
    instance: Option<VulkanInstance>,
    surface: Option<lume_vulkan::VulkanSurface>,
    device: Option<lume_vulkan::VulkanDevice>,
    swapchain: Option<lume_vulkan::VulkanSwapchain>,
    clusters: Vec<ClusterPacked>,
    vertices: Vec<AdaptrixVertex>,
    indices: Vec<u32>,
    cluster_buffer: Option<lume_vulkan::VulkanBuffer>,
    vertex_buffer: Option<lume_vulkan::VulkanBuffer>,
    index_buffer: Option<lume_vulkan::VulkanBuffer>,
    visible_clusters_buffer: Option<lume_vulkan::VulkanBuffer>,
    visible_count_buffer: Option<lume_vulkan::VulkanBuffer>,
    instance_buffer: Option<lume_vulkan::VulkanBuffer>,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform {
    view_proj: Mat4,
    inv_view_proj: Mat4,
    frustum: [Vec4; 6],
    viewport_size: [f32; 2],
    error_threshold: f32,
    _padding: f32,
}

impl AdaptrixApp {
    fn new() -> Self {
        let mut file = File::open("test.lad").expect("Failed to open test.lad.");
        let mut magic = [0u8; 4]; file.read_exact(&mut magic).unwrap();
        let mut version = [0u8; 4]; file.read_exact(&mut version).unwrap();
        let mut counts = [0u8; 12]; file.read_exact(&mut counts).unwrap();
        let cluster_count = u32::from_le_bytes(counts[0..4].try_into().unwrap());
        let vertex_count = u32::from_le_bytes(counts[4..8].try_into().unwrap());
        let index_count = u32::from_le_bytes(counts[8..12].try_into().unwrap());
        let mut clusters = vec![ClusterPacked::zeroed(); cluster_count as usize]; file.read_exact(bytemuck::cast_slice_mut(&mut clusters)).unwrap();
        let mut vertices = vec![AdaptrixVertex::zeroed(); vertex_count as usize]; file.read_exact(bytemuck::cast_slice_mut(&mut vertices)).unwrap();
        let mut indices = vec![0u32; index_count as usize]; file.read_exact(bytemuck::cast_slice_mut(&mut indices)).unwrap();
        Self {
            window: None, instance: None, surface: None, device: None, swapchain: None,
            clusters, vertices, indices,
            cluster_buffer: None, vertex_buffer: None, index_buffer: None,
            visible_clusters_buffer: None, visible_count_buffer: None, instance_buffer: None, view_buffer: None,
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
        self.cluster_buffer = Some(device.create_buffer(BufferDescriptor { size: (self.clusters.len() * 48) as u64, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.cluster_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(&self.clusters)).unwrap();
        self.vertex_buffer = Some(device.create_buffer(BufferDescriptor { size: (self.vertices.len() * 32) as u64, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.vertex_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(&self.vertices)).unwrap();
        self.index_buffer = Some(device.create_buffer(BufferDescriptor { size: (self.indices.len() * 4) as u64, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.index_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(&self.indices)).unwrap();
        
        // 关键修复：初始化 visible_clusters_buffer，默认为全可见
        let initial_visible: Vec<u32> = (0..self.clusters.len() as u32).collect();
        self.visible_clusters_buffer = Some(device.create_buffer(BufferDescriptor { size: (self.clusters.len() * 4) as u64, usage: BufferUsage::STORAGE, mapped_at_creation: true }).unwrap());
        self.visible_clusters_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(&initial_visible)).unwrap();

        self.visible_count_buffer = Some(device.create_buffer(BufferDescriptor { size: 4, usage: BufferUsage::STORAGE | BufferUsage::COPY_SRC | BufferUsage::COPY_DST | BufferUsage::INDIRECT, mapped_at_creation: true }).unwrap());
        self.view_buffer = Some(device.create_buffer(BufferDescriptor { size: 144, usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.instance_buffer = Some(device.create_buffer(BufferDescriptor { size: 80, usage: BufferUsage::STORAGE | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        
        self.vis_buffer_texture = Some(device.create_texture(TextureDescriptor { width: size.width, height: size.height, depth: 1, format: TextureFormat::Rg32Uint, usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING }).unwrap());
        self.vis_buffer_view = Some(device.create_texture_view(self.vis_buffer_texture.as_ref().unwrap(), TextureViewDescriptor { format: None }).unwrap());
        self.vis_depth_texture = Some(device.create_texture(TextureDescriptor { width: size.width, height: size.height, depth: 1, format: TextureFormat::Depth32Float, usage: TextureUsage::DEPTH_STENCIL_ATTACHMENT }).unwrap());
        self.vis_depth_view = Some(device.create_texture_view(self.vis_depth_texture.as_ref().unwrap(), TextureViewDescriptor { format: None }).unwrap());
        let cull_module = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/cull.wgsl"))).unwrap()).unwrap();
        let vis_v_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/visbuffer.vert.wgsl"))).unwrap()).unwrap();
        let vis_f_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/visbuffer.frag.wgsl"))).unwrap()).unwrap();
        let res_v_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/resolve.vert.wgsl"))).unwrap()).unwrap();
        let res_f_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/resolve.frag.wgsl"))).unwrap()).unwrap();
        let vis_rp = device.create_render_pass(RenderPassDescriptor { color_format: TextureFormat::Rg32Uint, depth_stencil_format: Some(TextureFormat::Depth32Float) }).unwrap();
        self.vis_framebuffer = Some(device.create_framebuffer(FramebufferDescriptor { render_pass: &vis_rp, attachments: &[self.vis_buffer_view.as_ref().unwrap(), self.vis_depth_view.as_ref().unwrap()], width: size.width, height: size.height }).unwrap());
        self.vis_render_pass = Some(vis_rp);
        let res_rp = device.create_render_pass(RenderPassDescriptor { color_format: TextureFormat::Bgra8UnormSrgb, depth_stencil_format: None }).unwrap();
        for i in 0..3 { self.resolve_framebuffers.push(device.create_framebuffer(FramebufferDescriptor { render_pass: &res_rp, attachments: &[self.swapchain.as_ref().unwrap().get_view(i as u32)], width: size.width, height: size.height }).unwrap()); }
        self.resolve_render_pass = Some(res_rp);
        let cull_bgl0 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, BindGroupLayoutEntry { binding: 3, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }] }).unwrap();
        let cull_bgl1 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::UniformBuffer }] }).unwrap();
        let cull_layout = device.create_pipeline_layout(PipelineLayoutDescriptor { bind_group_layouts: &[&cull_bgl0, &cull_bgl1] }).unwrap();
        self.cull_pipeline = Some(device.create_compute_pipeline(ComputePipelineDescriptor { shader: &cull_module, layout: &cull_layout }).unwrap());
        self.cull_bind_group_0 = Some(device.create_bind_group(BindGroupDescriptor { layout: &cull_bgl0, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.cluster_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.instance_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.visible_clusters_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 3, resource: BindingResource::Buffer(self.visible_count_buffer.as_ref().unwrap()) }] }).unwrap());
        self.cull_bind_group_1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &cull_bgl1, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.view_buffer.as_ref().unwrap()) }] }).unwrap());
        self.cull_layout = Some(cull_layout);
        let vis_bgl0 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer }, BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer }, BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer }, BindGroupLayoutEntry { binding: 3, visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer }] }).unwrap();
        let vis_bgl1 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT, ty: BindingType::UniformBuffer }] }).unwrap();
        let vis_layout = device.create_pipeline_layout(PipelineLayoutDescriptor { bind_group_layouts: &[&vis_bgl0, &vis_bgl1] }).unwrap();
        self.vis_pipeline = Some(device.create_graphics_pipeline(GraphicsPipelineDescriptor { vertex_shader: &vis_v_mod, fragment_shader: &vis_f_mod, render_pass: self.vis_render_pass.as_ref().unwrap(), layout: &vis_layout, primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList }, vertex_layout: None, depth_stencil: Some(DepthStencilState { format: TextureFormat::Depth32Float, depth_write_enabled: true, depth_compare: CompareFunction::LessEqual }) }).unwrap());
        self.vis_bind_group_0 = Some(device.create_bind_group(BindGroupDescriptor { layout: &vis_bgl0, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.cluster_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.vertex_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.index_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 3, resource: BindingResource::Buffer(self.visible_clusters_buffer.as_ref().unwrap()) }] }).unwrap());
        self.vis_bind_group_1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &vis_bgl1, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.view_buffer.as_ref().unwrap()) }] }).unwrap());
        self.vis_layout = Some(vis_layout);
        let res_bgl0 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer }, BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer }, BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer }] }).unwrap();
        let res_bgl1 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::FRAGMENT, ty: BindingType::UniformBuffer }, BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::FRAGMENT, ty: BindingType::SampledTexture }] }).unwrap();
        let res_layout = device.create_pipeline_layout(PipelineLayoutDescriptor { bind_group_layouts: &[&res_bgl0, &res_bgl1] }).unwrap();
        self.resolve_pipeline = Some(device.create_graphics_pipeline(GraphicsPipelineDescriptor { vertex_shader: &res_v_mod, fragment_shader: &res_f_mod, render_pass: self.resolve_render_pass.as_ref().unwrap(), layout: &res_layout, primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList }, vertex_layout: None, depth_stencil: None }).unwrap());
        self.resolve_bind_group_0 = Some(device.create_bind_group(BindGroupDescriptor { layout: &res_bgl0, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.cluster_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.vertex_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.index_buffer.as_ref().unwrap()) }] }).unwrap());
        self.resolve_bind_group_1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &res_bgl1, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.view_buffer.as_ref().unwrap()) }, BindGroupEntry { binding: 1, resource: BindingResource::TextureView(self.vis_buffer_view.as_ref().unwrap()) }] }).unwrap());
        self.resolve_layout = Some(res_layout);
        self.command_pool = Some(device.create_command_pool().unwrap());
        self.command_buffer = Some(self.command_pool.as_ref().unwrap().allocate_command_buffer().unwrap());
    }
}

impl ApplicationHandler for AdaptrixApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes().with_title("LumeVisual - Adaptrix Test").with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());
            let instance = VulkanInstance::new(InstanceDescriptor { name: "Adaptrix Test", backend: Backend::Vulkan }).unwrap();
            let surface = instance.create_surface(&window, &window).unwrap();
            let device = instance.request_device(Some(&surface)).unwrap();
            let swapchain = device.create_swapchain(&surface, SwapchainDescriptor { width: 1280, height: 720 }).unwrap();
            self.instance = Some(instance); self.surface = Some(surface); self.device = Some(device); self.swapchain = Some(swapchain);
            self.setup_gpu_resources();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let (Some(device), Some(swapchain)) = (&self.device, self.swapchain.as_mut()) {
                    let token = device.begin_frame(swapchain).expect("Failed to begin frame");
                    let elapsed = self.start_time.elapsed().as_secs_f32();
                    let cam_pos = Vec3::new(elapsed.cos() * 4.0, 1.5, elapsed.sin() * 4.0);
                    let view_mat = Mat4::look_at_rh(cam_pos, Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
                    
                    // 修正投影矩阵 Y 轴
                    let mut proj_mat = Mat4::perspective_rh(45.0f32.to_radians(), 1280.0/720.0, 0.1, 100.0);
                    proj_mat.col_mut(1).y *= -1.0; 
                    let view_proj = proj_mat * view_mat;
                    
                    self.view_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(&[ViewUniform { view_proj, inv_view_proj: view_proj.inverse(), frustum: [Vec4::ZERO; 6], viewport_size: [1280.0, 720.0], error_threshold: 1.0, _padding: 0.0 }])).unwrap();
                    
                    // 重置 Draw Args (instanceCount 设为 0)
                    // draw_args 结构: [vertexCount, instanceCount, firstVertex, firstInstance]
                    let initial_args: [u32; 4] = [124 * 3, 0, 0, 0];
                    self.visible_count_buffer.as_ref().unwrap().write_data(0, bytemuck::cast_slice(&initial_args)).unwrap();

                    let cmd = self.command_buffer.as_mut().unwrap();
                    cmd.reset().unwrap(); cmd.begin().unwrap();
                    
                    // --- Pass 0: Nanite DAG Traversal (Compute) ---
                    cmd.bind_compute_pipeline(self.cull_pipeline.as_ref().unwrap());
                    cmd.bind_bind_group(0, self.cull_bind_group_0.as_ref().unwrap());
                    cmd.bind_bind_group(1, self.cull_bind_group_1.as_ref().unwrap());
                    let group_count = (self.clusters.len() as u32 + 63) / 64;
                    cmd.dispatch(group_count, 1, 1);
                    cmd.compute_barrier();

                    // Pass 1: VisBuffer (渲染被选中的集群)
                    cmd.begin_render_pass(self.vis_render_pass.as_ref().unwrap(), self.vis_framebuffer.as_ref().unwrap(), [0.0, 0.0, 0.0, 0.0]);
                    cmd.set_viewport(0.0, 0.0, 1280.0, 720.0); cmd.set_scissor(0, 0, 1280, 720);
                    cmd.bind_graphics_pipeline(self.vis_pipeline.as_ref().unwrap());
                    cmd.bind_bind_group(0, self.vis_bind_group_0.as_ref().unwrap());
                    cmd.bind_bind_group(1, self.vis_bind_group_1.as_ref().unwrap());
                    
                    // 这里我们暂时还是手动调用 draw，但 instance_count 会被 VisibleClusters 缓冲区影响
                    // 如果你的底层支持 draw_indirect，请通知我替换。
                    // 目前我们根据 self.clusters.len() 绘制，在 shader 里根据可见性 buffer 处理
                    cmd.draw(124 * 3, self.clusters.len() as u32, 0, 0); 
                    cmd.end_render_pass();

                    // Pass 2: Resolve
                    cmd.begin_render_pass(self.resolve_render_pass.as_ref().unwrap(), &self.resolve_framebuffers[token.image_index as usize], [0.05, 0.05, 0.07, 1.0]);
                    cmd.set_viewport(0.0, 0.0, 1280.0, 720.0); cmd.set_scissor(0, 0, 1280, 720);
                    cmd.bind_graphics_pipeline(self.resolve_pipeline.as_ref().unwrap());
                    cmd.bind_bind_group(0, self.resolve_bind_group_0.as_ref().unwrap());
                    cmd.bind_bind_group(1, self.resolve_bind_group_1.as_ref().unwrap());
                    cmd.draw(3, 1, 0, 0); 
                    cmd.end_render_pass();
                    
                    cmd.end().unwrap();
                    device.end_frame(swapchain, token, &[cmd]).expect("Failed to end frame");
                }
            }
            _ => (),
        }
        if let Some(window) = &self.window { window.request_redraw(); }
    }
}
fn main() { env_logger::init(); let event_loop = EventLoop::new().unwrap(); let mut app = AdaptrixApp::new(); event_loop.run_app(&mut app).unwrap(); }
