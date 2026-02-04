use lume_core::device::*;
use lume_core::LumeResult;
use crate::{AdaptrixAsset, ClusterPacked, AdaptrixVertex};

pub struct AdaptrixMeshGPU<D: Device> {
    pub cluster_buffer: D::Buffer,
    pub vertex_buffer: D::Buffer,
    pub vertex_index_buffer: D::Buffer,
    pub primitive_index_buffer: D::Buffer,
    pub cluster_count: u32,
}

impl<D: Device> AdaptrixMeshGPU<D> {
    pub fn new(device: &D, asset: &AdaptrixAsset) -> LumeResult<Self> {
        let cluster_buffer = device.create_buffer(BufferDescriptor {
            size: (asset.clusters.len() * std::mem::size_of::<ClusterPacked>()) as u64,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            mapped_at_creation: true,
        })?;
        cluster_buffer.write_data(0, bytemuck::cast_slice(&asset.clusters))?;

        let vertex_buffer = device.create_buffer(BufferDescriptor {
            size: (asset.vertices.len() * std::mem::size_of::<AdaptrixVertex>()) as u64,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            mapped_at_creation: true,
        })?;
        vertex_buffer.write_data(0, bytemuck::cast_slice(&asset.vertices))?;

        let vertex_index_buffer = device.create_buffer(BufferDescriptor {
            size: (asset.meshlet_vertex_indices.len() * 4) as u64,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            mapped_at_creation: true,
        })?;
        vertex_index_buffer.write_data(0, bytemuck::cast_slice(&asset.meshlet_vertex_indices))?;

        let primitive_index_buffer = device.create_buffer(BufferDescriptor {
            size: asset.meshlet_primitive_indices.len() as u64,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            mapped_at_creation: true,
        })?;
        primitive_index_buffer.write_data(0, &asset.meshlet_primitive_indices)?;

        Ok(Self {
            cluster_buffer,
            vertex_buffer,
            vertex_index_buffer,
            primitive_index_buffer,
            cluster_count: asset.clusters.len() as u32,
        })
    }
}

pub struct AdaptrixFrameData<D: Device> {
    pub hw_visible_clusters: D::Buffer,
    pub hw_indirect_args: D::Buffer,
    pub sw_visible_clusters: D::Buffer,
    pub sw_indirect_args: D::Buffer,
    
    // Software Rasterizer Target
    pub sw_vis_buffer: D::Texture,
    pub sw_vis_view: D::TextureView,

    // Hardware Rasterizer Targets
    pub hw_vis_buffer: D::Texture,
    pub hw_vis_view: D::TextureView,
    pub hw_depth_buffer: D::Texture,
    pub hw_depth_view: D::TextureView,
}

impl<D: Device> AdaptrixFrameData<D> {
    pub fn new(device: &D, width: u32, height: u32, max_clusters: u32) -> LumeResult<Self> {
        let hw_visible_clusters = device.create_buffer(BufferDescriptor {
            size: (max_clusters * 4) as u64,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_SRC,
            mapped_at_creation: false,
        })?;

        let hw_indirect_args = device.create_buffer(BufferDescriptor {
            size: 20, // 5 * u32
            usage: BufferUsage::STORAGE | BufferUsage::INDIRECT | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        })?;

        let sw_visible_clusters = device.create_buffer(BufferDescriptor {
            size: (max_clusters * 4) as u64,
            usage: BufferUsage::STORAGE,
            mapped_at_creation: false,
        })?;

        let sw_indirect_args = device.create_buffer(BufferDescriptor {
            size: 12, // 3 * u32
            usage: BufferUsage::STORAGE | BufferUsage::INDIRECT | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        })?;

        // SW VisBuffer (R32Uint)
        let sw_vis_buffer = device.create_texture(TextureDescriptor {
            width, height, depth: 1,
            format: TextureFormat::R32Uint,
            usage: TextureUsage::STORAGE_BINDING | TextureUsage::TEXTURE_BINDING | TextureUsage::COPY_DST, // COPY_DST for clear
        })?;
        let sw_vis_view = device.create_texture_view(&sw_vis_buffer, TextureViewDescriptor { format: None })?;

        // HW VisBuffer (R32Uint - ID only)
        let hw_vis_buffer = device.create_texture(TextureDescriptor {
            width, height, depth: 1,
            format: TextureFormat::R32Uint, // Or R32Uint
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        })?;
        let hw_vis_view = device.create_texture_view(&hw_vis_buffer, TextureViewDescriptor { format: None })?;

        // HW DepthBuffer
        let hw_depth_buffer = device.create_texture(TextureDescriptor {
            width, height, depth: 1,
            format: TextureFormat::Depth32Float,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        })?;
        let hw_depth_view = device.create_texture_view(&hw_depth_buffer, TextureViewDescriptor { format: None })?;

        Ok(Self {
            hw_visible_clusters,
            hw_indirect_args,
            sw_visible_clusters,
            sw_indirect_args,
            sw_vis_buffer,
            sw_vis_view,
            hw_vis_buffer,
            hw_vis_view,
            hw_depth_buffer,
            hw_depth_view,
        })
    }
}

pub struct AdaptrixRenderer<D: Device> {
    pub culling_pipeline: D::ComputePipeline,
    pub soft_raster_pipeline: D::ComputePipeline,
    pub visbuffer_pipeline: D::GraphicsPipeline,
    pub resolve_pipeline: D::GraphicsPipeline,
    
    pub culling_layout: D::PipelineLayout,
    pub soft_raster_layout: D::PipelineLayout,
    pub visbuffer_layout: D::PipelineLayout,
    pub resolve_layout: D::PipelineLayout,

    pub cull_bind_group_layout: D::BindGroupLayout,
    pub soft_raster_bind_group_layout_0: D::BindGroupLayout,
    pub soft_raster_bind_group_layout_1: D::BindGroupLayout,
    
    // We also need BGLs for HW VisBuffer and Resolve if we create them dynamically
    // Assuming VisBuffer pipeline layout is compatible with Cull Group 0 for cluster data?
    // VisBuffer Pipeline:
    // Group 0: Cluster Data (Clusters, Vertices, Indices, VisibleClusters)
    // Group 1: View
    pub vis_bind_group_layout_0: D::BindGroupLayout,
    
    // Resolve Pipeline:
    // Group 0: Cluster Data (Clusters, Vertices, Indices)
    // Group 1: Resolve Data (View, HW Vis, HW Depth, SW Vis)
    pub resolve_bind_group_layout_0: D::BindGroupLayout,
    pub resolve_bind_group_layout_1: D::BindGroupLayout,
}

impl<D: Device> AdaptrixRenderer<D> {
    pub fn new(
        device: &D, 
        cull_spv: &[u32],
        soft_raster_spv: &[u32], 
        vis_vert_spv: &[u32], 
        vis_frag_spv: &[u32],
        resolve_vert_spv: &[u32], 
        resolve_frag_spv: &[u32],
        
        cull_layout: D::PipelineLayout,
        soft_raster_layout: D::PipelineLayout,
        vis_layout: D::PipelineLayout,
        resolve_layout: D::PipelineLayout,

        cull_bg_layout: D::BindGroupLayout,
        soft_raster_bg_layout_0: D::BindGroupLayout,
        soft_raster_bg_layout_1: D::BindGroupLayout,
        vis_bg_layout_0: D::BindGroupLayout,
        resolve_bg_layout_0: D::BindGroupLayout,
        resolve_bg_layout_1: D::BindGroupLayout,

        vis_pass: &D::RenderPass,
        resolve_pass: &D::RenderPass,
    ) -> LumeResult<Self> {
        let cull_mod = device.create_shader_module(cull_spv)?;
        let culling_pipeline = device.create_compute_pipeline(ComputePipelineDescriptor {
            shader: &cull_mod,
            layout: &cull_layout, 
        })?;

        let soft_mod = device.create_shader_module(soft_raster_spv)?;
        let soft_raster_pipeline = device.create_compute_pipeline(ComputePipelineDescriptor {
            shader: &soft_mod,
            layout: &soft_raster_layout, 
        })?;

        let vis_vert = device.create_shader_module(vis_vert_spv)?;
        let vis_frag = device.create_shader_module(vis_frag_spv)?;
        
        let visbuffer_pipeline = device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            vertex_shader: &vis_vert,
            fragment_shader: &vis_frag,
            render_pass: vis_pass,
            layout: &vis_layout,
            primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList, cull_mode: CullMode::None },
            vertex_layout: None,
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
            }),
        })?;

        let res_vert = device.create_shader_module(resolve_vert_spv)?;
        let res_frag = device.create_shader_module(resolve_frag_spv)?;

        let resolve_pipeline = device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            vertex_shader: &res_vert,
            fragment_shader: &res_frag,
            render_pass: resolve_pass,
            layout: &resolve_layout,
            primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList, cull_mode: CullMode::None },
            vertex_layout: None,
            depth_stencil: None,
        })?;

        Ok(Self {
            culling_pipeline,
            soft_raster_pipeline,
            visbuffer_pipeline,
            resolve_pipeline,
            culling_layout: cull_layout,
            soft_raster_layout,
            visbuffer_layout: vis_layout,
            resolve_layout,
            cull_bind_group_layout: cull_bg_layout,
            soft_raster_bind_group_layout_0: soft_raster_bg_layout_0,
            soft_raster_bind_group_layout_1: soft_raster_bg_layout_1,
            vis_bind_group_layout_0: vis_bg_layout_0,
            resolve_bind_group_layout_0: resolve_bg_layout_0,
            resolve_bind_group_layout_1: resolve_bg_layout_1,
        })
    }

    pub fn render(
        &self,
        encoder: &mut D::CommandBuffer,
        frame: &AdaptrixFrameData<D>,
        mesh: &AdaptrixMeshGPU<D>,
        view_bind_group: &D::BindGroup,
        view_uniform_buffer: &D::Buffer,
        output_view: &D::TextureView,
        device: &D,
    ) -> LumeResult<()> {
        // 1. Reset Counters
        let max_indices_per_cluster = 124 * 3; 
        let zero_args = [max_indices_per_cluster, 0, 0, 0, 0]; 
        frame.hw_indirect_args.write_data(0, bytemuck::cast_slice(&zero_args))?;

        let sw_zero = [0u32, 1, 1];
        frame.sw_indirect_args.write_data(0, bytemuck::cast_slice(&sw_zero))?;
        
        // Clear SW Vis Buffer (Manual clear via copy or compute, but let's assume cleared by user or new frame)
        // TODO: Implement clear logic for SW Vis Buffer (e.g. fill with 0)

        // 2. Cull Pass
        encoder.bind_compute_pipeline(&self.culling_pipeline);
        
        let cull_bg = device.create_bind_group(BindGroupDescriptor {
            layout: &self.cull_bind_group_layout,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::Buffer(&mesh.cluster_buffer) },
                BindGroupEntry { binding: 1, resource: BindingResource::Buffer(&frame.hw_visible_clusters) },
                BindGroupEntry { binding: 2, resource: BindingResource::Buffer(&frame.hw_indirect_args) },
                BindGroupEntry { binding: 3, resource: BindingResource::Buffer(&frame.sw_visible_clusters) },
                BindGroupEntry { binding: 4, resource: BindingResource::Buffer(&frame.sw_indirect_args) },
            ],
        })?;

        encoder.bind_bind_group(0, &cull_bg);
        encoder.bind_bind_group(1, view_bind_group);
        
        let dispatch_x = (mesh.cluster_count + 63) / 64;
        encoder.dispatch(dispatch_x, 1, 1);

        encoder.compute_barrier();

        // 3. Soft Raster Pass
        encoder.bind_compute_pipeline(&self.soft_raster_pipeline);

        let soft_bg_0 = device.create_bind_group(BindGroupDescriptor {
            layout: &self.soft_raster_bind_group_layout_0,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::Buffer(&mesh.cluster_buffer) },
                BindGroupEntry { binding: 1, resource: BindingResource::Buffer(&mesh.vertex_buffer) },
                BindGroupEntry { binding: 2, resource: BindingResource::Buffer(&mesh.vertex_index_buffer) },
                BindGroupEntry { binding: 3, resource: BindingResource::Buffer(&mesh.primitive_index_buffer) },
                BindGroupEntry { binding: 4, resource: BindingResource::Buffer(&frame.sw_visible_clusters) },
            ],
        })?;
        
        let soft_bg_1 = device.create_bind_group(BindGroupDescriptor {
            layout: &self.soft_raster_bind_group_layout_1,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&frame.sw_vis_view) }, // Corrected to SW Vis
                BindGroupEntry { binding: 1, resource: BindingResource::Buffer(view_uniform_buffer) },
            ],
        })?;
        
        encoder.bind_bind_group(0, &soft_bg_0);
        encoder.bind_bind_group(1, &soft_bg_1);
        
        encoder.dispatch_indirect(&frame.sw_indirect_args, 0);
        
        encoder.compute_barrier(); // Barrier for SW Vis Buffer usage in next pass?

        // 4. Hardware Raster Pass
        encoder.begin_rendering(RenderingDescriptor {
            color_attachments: &[RenderingAttachment {
                view: &frame.hw_vis_view,
                layout: ImageLayout::ColorAttachment,
                load_op: AttachmentLoadOp::Clear,
                store_op: AttachmentStoreOp::Store,
                clear_value: ClearValue::Color([0.0, 0.0, 0.0, 0.0]), // ID 0 is invalid
            }],
            depth_attachment: Some(RenderingAttachment {
                view: &frame.hw_depth_view,
                layout: ImageLayout::DepthStencilAttachment,
                load_op: AttachmentLoadOp::Clear,
                store_op: AttachmentStoreOp::Store,
                clear_value: ClearValue::DepthStencil(1.0, 0), // Far
            }),
            stencil_attachment: None,
            view_mask: 0,
        });
        
        encoder.bind_graphics_pipeline(&self.visbuffer_pipeline);
        
        let vis_bg_0 = device.create_bind_group(BindGroupDescriptor {
            layout: &self.vis_bind_group_layout_0,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::Buffer(&mesh.cluster_buffer) },
                BindGroupEntry { binding: 1, resource: BindingResource::Buffer(&mesh.vertex_buffer) },
                BindGroupEntry { binding: 2, resource: BindingResource::Buffer(&mesh.vertex_index_buffer) },
                BindGroupEntry { binding: 3, resource: BindingResource::Buffer(&frame.hw_visible_clusters) },
            ],
        })?;
        
        encoder.bind_bind_group(0, &vis_bg_0);
        encoder.bind_bind_group(1, view_bind_group);
        
        encoder.draw_indirect(&frame.hw_indirect_args, 0, 1, 20); // 20 bytes stride? 5*u32
        
        encoder.end_rendering();

        // 5. Resolve Pass
        encoder.begin_rendering(RenderingDescriptor {
            color_attachments: &[RenderingAttachment {
                view: output_view,
                layout: ImageLayout::ColorAttachment,
                load_op: AttachmentLoadOp::Clear,
                store_op: AttachmentStoreOp::Store,
                clear_value: ClearValue::Color([0.1, 0.1, 0.1, 1.0]),
            }],
            depth_attachment: None,
            stencil_attachment: None,
            view_mask: 0,
        });
        
        encoder.bind_graphics_pipeline(&self.resolve_pipeline);
        
        let resolve_bg_0 = device.create_bind_group(BindGroupDescriptor {
            layout: &self.resolve_bind_group_layout_0,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::Buffer(&mesh.cluster_buffer) },
                BindGroupEntry { binding: 1, resource: BindingResource::Buffer(&mesh.vertex_buffer) },
                BindGroupEntry { binding: 2, resource: BindingResource::Buffer(&mesh.vertex_index_buffer) },
            ],
        })?;
        
        let resolve_bg_1 = device.create_bind_group(BindGroupDescriptor {
            layout: &self.resolve_bind_group_layout_1,
            entries: vec![
                BindGroupEntry { binding: 0, resource: BindingResource::Buffer(view_uniform_buffer) }, // Wait, logic in shader is View (Uniform), HW(Tex), Depth(Tex), SW(Tex)
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&frame.hw_vis_view) },
                BindGroupEntry { binding: 2, resource: BindingResource::TextureView(&frame.hw_depth_view) },
                BindGroupEntry { binding: 3, resource: BindingResource::TextureView(&frame.sw_vis_view) },
            ],
        })?;

        encoder.bind_bind_group(0, &resolve_bg_0);
        encoder.bind_bind_group(1, &resolve_bg_1);
        
        encoder.draw(3, 1, 0, 0); // Fullscreen triangle
        
        encoder.end_rendering();

        Ok(())
    }
}