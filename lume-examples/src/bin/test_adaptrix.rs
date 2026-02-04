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

    // Soft raster (补洞层)
    sw_visible_clusters_buffer: Option<lume_vulkan::VulkanBuffer>,
    sw_dispatch_args_buffer: Option<lume_vulkan::VulkanBuffer>,
    sw_zero_dispatch_buffer: Option<lume_vulkan::VulkanBuffer>,
    sw_depth_buffer: Option<lume_vulkan::VulkanBuffer>,
    sw_id_buffer: Option<lume_vulkan::VulkanBuffer>,
    clear_sw_pipeline: Option<lume_vulkan::VulkanComputePipeline>,
    clear_sw_layout: Option<lume_vulkan::VulkanPipelineLayout>,
    clear_sw_bg: Option<lume_vulkan::VulkanBindGroup>,
    soft_pipeline: Option<lume_vulkan::VulkanComputePipeline>,
    soft_layout: Option<lume_vulkan::VulkanPipelineLayout>,
    soft_bg0: Option<lume_vulkan::VulkanBindGroup>,
    soft_bg1: Option<lume_vulkan::VulkanBindGroup>,
    soft_view_proj_buffer: Option<lume_vulkan::VulkanBuffer>,
    soft_viewport_buffer: Option<lume_vulkan::VulkanBuffer>,

    // HZB (one texture per mip for now)
    hzb_textures: Vec<lume_vulkan::VulkanTexture>,
    hzb_views: Vec<lume_vulkan::VulkanTextureView>,
    hzb_pipeline: Option<lume_vulkan::VulkanComputePipeline>,
    hzb_layout: Option<lume_vulkan::VulkanPipelineLayout>,
    hzb_bind_groups: Vec<lume_vulkan::VulkanBindGroup>,
    
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
            sw_visible_clusters_buffer: None,
            sw_dispatch_args_buffer: None,
            sw_zero_dispatch_buffer: None,
            sw_depth_buffer: None,
            sw_id_buffer: None,
            clear_sw_pipeline: None,
            clear_sw_layout: None,
            clear_sw_bg: None,
            soft_pipeline: None,
            soft_layout: None,
            soft_bg0: None,
            soft_bg1: None,
            soft_view_proj_buffer: None,
            soft_viewport_buffer: None,
            hzb_textures: Vec::new(),
            hzb_views: Vec::new(),
            hzb_pipeline: None,
            hzb_layout: None,
            hzb_bind_groups: Vec::new(),
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

        // SW visible list + dispatch args
        self.sw_visible_clusters_buffer = Some(device.create_buffer(BufferDescriptor {
            size: (asset.clusters.len() * 4).min(256 * 1024 * 1024) as u64,
            usage: BufferUsage::STORAGE,
            mapped_at_creation: true,
        }).unwrap());
        self.sw_dispatch_args_buffer = Some(device.create_buffer(BufferDescriptor {
            size: 12,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST | BufferUsage::COPY_SRC | BufferUsage::INDIRECT,
            mapped_at_creation: true,
        }).unwrap());
        let sw_zero = device.create_buffer(BufferDescriptor { size: 12, usage: BufferUsage::COPY_SRC, mapped_at_creation: true }).unwrap();
        sw_zero.write_data(0, bytemuck::cast_slice(&[0u32, 1u32, 1u32])).unwrap();
        self.sw_zero_dispatch_buffer = Some(sw_zero);
        
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
        // Depth needs to be sampled to build HZB
        self.vis_depth_texture = Some(device.create_texture(TextureDescriptor {
            width: size.width,
            height: size.height,
            depth: 1,
            format: TextureFormat::Depth32Float,
            usage: TextureUsage::DEPTH_STENCIL_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        }).unwrap());
        self.vis_depth_view = Some(device.create_texture_view(self.vis_depth_texture.as_ref().unwrap(), TextureViewDescriptor { format: None }).unwrap());

        // SW overlay buffers (width*height u32)
        let pixel_count = (size.width as u64) * (size.height as u64);
        self.sw_depth_buffer = Some(device.create_buffer(BufferDescriptor {
            size: pixel_count * 4,
            usage: BufferUsage::STORAGE,
            mapped_at_creation: true,
        }).unwrap());
        self.sw_id_buffer = Some(device.create_buffer(BufferDescriptor {
            size: pixel_count * 4,
            usage: BufferUsage::STORAGE,
            mapped_at_creation: true,
        }).unwrap());

        let cull_module = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/cull.wgsl"))).unwrap()).unwrap();
        let hzb_module = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/hzb.wgsl"))).unwrap()).unwrap();
        let clear_module = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/clear_sw_buffers.wgsl"))).unwrap()).unwrap();
        let soft_module = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/soft_raster.wgsl"))).unwrap()).unwrap();
        let vis_v_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/visbuffer.vert.wgsl"))).unwrap()).unwrap();
        let vis_f_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/visbuffer.frag.wgsl"))).unwrap()).unwrap();
        let res_v_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/resolve.vert.wgsl"))).unwrap()).unwrap();
        let res_f_mod = device.create_shader_module(&lume_core::shader::compile_shader(lume_core::shader::ShaderSource::Wgsl(include_str!("../../../lume-adaptrix/src/shaders/resolve.frag.wgsl"))).unwrap()).unwrap();

        // Cull
        let bgl_c0 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 3, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
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
            BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.sw_visible_clusters_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 3, resource: BindingResource::Buffer(self.sw_dispatch_args_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 5, resource: BindingResource::Buffer(self.visible_count_buffer.as_ref().unwrap()) },
        ] }).unwrap());
        self.cull_bind_group_1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_c1, entries: vec![BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.view_buffer.as_ref().unwrap()) }] }).unwrap());
        self.cull_layout = Some(l_cull);

        // Clear SW buffers
        let bgl_clear = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer },
        ]}).unwrap();
        let l_clear = device.create_pipeline_layout(PipelineLayoutDescriptor {
            bind_group_layouts: &[&bgl_clear],
            push_constant_ranges: &[PushConstantRange { stages: ShaderStage::COMPUTE, offset: 0, size: 4 }],
        }).unwrap();
        self.clear_sw_pipeline = Some(device.create_compute_pipeline(ComputePipelineDescriptor { shader: &clear_module, layout: &l_clear }).unwrap());
        self.clear_sw_layout = Some(l_clear);
        self.clear_sw_bg = Some(device.create_bind_group(BindGroupDescriptor {
            layout: &bgl_clear,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.sw_depth_buffer.as_ref().unwrap()) },
                BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.sw_id_buffer.as_ref().unwrap()) },
            ],
        }).unwrap());

        // Soft raster
        let bgl_s0 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, // clusters
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, // vertices
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, // v_indices
            BindGroupLayoutEntry { binding: 3, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, // p_indices
            BindGroupLayoutEntry { binding: 4, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, // visible_clusters (sw list)
            BindGroupLayoutEntry { binding: 5, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, // sw_dispatch_args
        ]}).unwrap();
        let bgl_s1 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, // sw_depth
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageBuffer }, // sw_id
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::COMPUTE, ty: BindingType::UniformBuffer },  // view_proj
            BindGroupLayoutEntry { binding: 3, visibility: ShaderStage::COMPUTE, ty: BindingType::UniformBuffer },  // viewport
        ]}).unwrap();
        let l_soft = device.create_pipeline_layout(PipelineLayoutDescriptor {
            bind_group_layouts: &[&bgl_s0, &bgl_s1],
            push_constant_ranges: &[],
        }).unwrap();
        self.soft_pipeline = Some(device.create_compute_pipeline(ComputePipelineDescriptor { shader: &soft_module, layout: &l_soft }).unwrap());
        self.soft_layout = Some(l_soft);
        self.soft_bg0 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_s0, entries: vec![
            BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.cluster_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.vertex_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.vertex_index_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 3, resource: BindingResource::Buffer(self.primitive_index_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 4, resource: BindingResource::Buffer(self.sw_visible_clusters_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 5, resource: BindingResource::Buffer(self.sw_dispatch_args_buffer.as_ref().unwrap()) },
        ]}).unwrap());

        self.soft_view_proj_buffer = Some(device.create_buffer(BufferDescriptor { size: 64, usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());
        self.soft_viewport_buffer = Some(device.create_buffer(BufferDescriptor { size: 16, usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST, mapped_at_creation: true }).unwrap());

        self.soft_bg1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_s1, entries: vec![
            BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.sw_depth_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 1, resource: BindingResource::Buffer(self.sw_id_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.soft_view_proj_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 3, resource: BindingResource::Buffer(self.soft_viewport_buffer.as_ref().unwrap()) },
        ]}).unwrap());
        // HZB
        // Build a mip chain as separate R32Float textures (to avoid mip-level view complexity for now)
        self.hzb_textures.clear();
        self.hzb_views.clear();
        self.hzb_bind_groups.clear();
        let mut w = size.width.max(1);
        let mut h = size.height.max(1);
        while w > 1 || h > 1 {
            w = (w / 2).max(1);
            h = (h / 2).max(1);
            let tex = device.create_texture(TextureDescriptor {
                width: w,
                height: h,
                depth: 1,
                format: TextureFormat::R32Float,
                usage: TextureUsage::STORAGE_BINDING | TextureUsage::TEXTURE_BINDING,
            }).unwrap();
            let view = device.create_texture_view(&tex, TextureViewDescriptor { format: None }).unwrap();
            self.hzb_textures.push(tex);
            self.hzb_views.push(view);
            if w == 1 && h == 1 { break; }
        }

        let bgl_hzb = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::COMPUTE, ty: BindingType::SampledTexture },
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::COMPUTE, ty: BindingType::StorageTexture },
        ] }).unwrap();

        // push constants: src_size: vec2<u32>
        let l_hzb = device.create_pipeline_layout(PipelineLayoutDescriptor {
            bind_group_layouts: &[&bgl_hzb],
            push_constant_ranges: &[PushConstantRange { stages: ShaderStage::COMPUTE, offset: 0, size: 8 }],
        }).unwrap();
        self.hzb_pipeline = Some(device.create_compute_pipeline(ComputePipelineDescriptor { shader: &hzb_module, layout: &l_hzb }).unwrap());
        self.hzb_layout = Some(l_hzb);

        // Bind groups per mip: level0 reads depth, writes hzb[0]; subsequent reads hzb[i-1], writes hzb[i]
        if !self.hzb_views.is_empty() {
            // first
            self.hzb_bind_groups.push(device.create_bind_group(BindGroupDescriptor {
                layout: &bgl_hzb,
                entries: vec![
                    BindGroupEntry { binding: 0, resource: BindingResource::TextureView(self.vis_depth_view.as_ref().unwrap()) },
                    BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&self.hzb_views[0]) },
                ],
            }).unwrap());

            for i in 1..self.hzb_views.len() {
                self.hzb_bind_groups.push(device.create_bind_group(BindGroupDescriptor {
                    layout: &bgl_hzb,
                    entries: vec![
                        BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&self.hzb_views[i - 1]) },
                        BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&self.hzb_views[i]) },
                    ],
                }).unwrap());
            }
        }

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
        let bgl_r1 = device.create_bind_group_layout(BindGroupLayoutDescriptor { entries: vec![
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStage::FRAGMENT, ty: BindingType::UniformBuffer },
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStage::FRAGMENT, ty: BindingType::SampledTexture },
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStage::FRAGMENT, ty: BindingType::StorageBuffer },
        ] }).unwrap();
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
        self.resolve_bind_group_1 = Some(device.create_bind_group(BindGroupDescriptor { layout: &bgl_r1, entries: vec![
            BindGroupEntry { binding: 0, resource: BindingResource::Buffer(self.view_buffer.as_ref().unwrap()) },
            BindGroupEntry { binding: 1, resource: BindingResource::TextureView(self.vis_buffer_view.as_ref().unwrap()) },
            BindGroupEntry { binding: 2, resource: BindingResource::Buffer(self.sw_id_buffer.as_ref().unwrap()) },
        ] }).unwrap());
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

                    // Soft raster uniforms (mat4x4<f32> as 4x vec4)
                    if let Some(buf) = self.soft_view_proj_buffer.as_ref() {
                        let cols = [vp.col(0), vp.col(1), vp.col(2), vp.col(3)];
                        buf.write_data(0, bytemuck::cast_slice(&cols)).unwrap();
                    }
                    if let Some(buf) = self.soft_viewport_buffer.as_ref() {
                        // viewport: x,y,w,h
                        let v = [0.0f32, 0.0f32, 1280.0f32, 720.0f32];
                        buf.write_data(0, bytemuck::cast_slice(&v)).unwrap();
                    }

                    let cmd = self.command_buffer.as_mut().unwrap();
                    cmd.reset().unwrap(); cmd.begin().unwrap();
                    
                    cmd.copy_buffer_to_buffer(self.zero_buffer.as_ref().unwrap(), self.visible_count_buffer.as_ref().unwrap(), 16);
                    // reset sw dispatch args (x=0)
                    cmd.copy_buffer_to_buffer(self.sw_zero_dispatch_buffer.as_ref().unwrap(), self.sw_dispatch_args_buffer.as_ref().unwrap(), 12);
                    cmd.compute_barrier();

                    cmd.bind_compute_pipeline(self.cull_pipeline.as_ref().unwrap());
                    cmd.bind_bind_group(0, self.cull_bind_group_0.as_ref().unwrap());
                    cmd.bind_bind_group(1, self.cull_bind_group_1.as_ref().unwrap());
                    cmd.dispatch((self.asset.as_ref().unwrap().clusters.len() as u32 + 63) / 64, 1, 1);
                    cmd.compute_barrier();

                    // Ensure vis targets are in correct layouts
                    cmd.texture_barrier(self.vis_buffer_view.as_ref().unwrap(), ImageLayout::Undefined, ImageLayout::ColorAttachment);
                    cmd.texture_barrier(self.vis_depth_view.as_ref().unwrap(), ImageLayout::Undefined, ImageLayout::DepthStencilAttachment);

                    cmd.begin_render_pass(self.vis_render_pass.as_ref().unwrap(), self.vis_framebuffer.as_ref().unwrap(), [0.0, 0.0, 0.0, 0.0]);
                    cmd.set_viewport(0.0, 0.0, 1280.0, 720.0); cmd.set_scissor(0, 0, 1280, 720);
                    cmd.bind_graphics_pipeline(self.vis_pipeline.as_ref().unwrap());
                    cmd.bind_bind_group(0, self.vis_bind_group_0.as_ref().unwrap());
                    cmd.bind_bind_group(1, self.vis_bind_group_1.as_ref().unwrap());
                    cmd.draw_indirect(self.visible_count_buffer.as_ref().unwrap(), 0, 1, 16);
                    cmd.end_render_pass();

                    // SW overlay clear + soft raster
                    if self.clear_sw_pipeline.is_some() && self.soft_pipeline.is_some() {
                        // clear sw buffers
                        cmd.bind_compute_pipeline(self.clear_sw_pipeline.as_ref().unwrap());
                        cmd.bind_bind_group(0, self.clear_sw_bg.as_ref().unwrap());
                        let count = 1280u32 * 720u32;
                        cmd.set_push_constants(self.clear_sw_layout.as_ref().unwrap(), ShaderStage::COMPUTE, 0, bytemuck::bytes_of(&count));
                        cmd.dispatch((count + 255) / 256, 1, 1);
                        cmd.compute_barrier();

                        // soft raster dispatch: over-approx; shader reads sw_dispatch_args.x and early-outs
                        cmd.bind_compute_pipeline(self.soft_pipeline.as_ref().unwrap());
                        cmd.bind_bind_group(0, self.soft_bg0.as_ref().unwrap());
                        cmd.bind_bind_group(1, self.soft_bg1.as_ref().unwrap());
                        // Use indirect dispatch for SW rasterizer to avoid processing empty groups
                        cmd.dispatch_indirect(self.sw_dispatch_args_buffer.as_ref().unwrap(), 0);
                        cmd.compute_barrier();
                    }

                    // Build HZB from depth for next frame's occlusion culling (currently just generated/validated)
                    if self.hzb_pipeline.is_some() && !self.hzb_bind_groups.is_empty() {
                        cmd.texture_barrier(self.vis_depth_view.as_ref().unwrap(), ImageLayout::DepthStencilAttachment, ImageLayout::ShaderReadOnly);

                        cmd.bind_compute_pipeline(self.hzb_pipeline.as_ref().unwrap());
                        let mut src_w = 1280u32;
                        let mut src_h = 720u32;
                        for (i, bg) in self.hzb_bind_groups.iter().enumerate() {
                            cmd.bind_bind_group(0, bg);
                            // push constants: src_size (u32x2)
                            let pc = [src_w, src_h];
                            cmd.set_push_constants(self.hzb_layout.as_ref().unwrap(), ShaderStage::COMPUTE, 0, bytemuck::bytes_of(&pc));
                            let dst_w = (src_w / 2).max(1);
                            let dst_h = (src_h / 2).max(1);
                            cmd.dispatch((dst_w + 15) / 16, (dst_h + 15) / 16, 1);
                            cmd.compute_barrier();

                            // Prepare next level input
                            src_w = dst_w;
                            src_h = dst_h;
                            if i < self.hzb_views.len() {
                                cmd.texture_barrier(&self.hzb_views[i], ImageLayout::General, ImageLayout::ShaderReadOnly);
                            }
                        }

                        // Keep depth sampled layout until next frame; we barrier back at frame start
                    }

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