use lume_core::device::*;
use lume_core::LumeResult;
use crate::{AdaptrixFlatAsset, ClusterPacked, AdaptrixVertex};

pub struct AdaptrixMeshGPU<D: Device> {
    pub cluster_buffer: D::Buffer,
    pub vertex_buffer: D::Buffer,
    pub vertex_index_buffer: D::Buffer,
    pub primitive_index_buffer: D::Buffer,
    pub cluster_count: u32,
}

impl<D: Device> AdaptrixMeshGPU<D> {
    pub fn new(device: &D, asset: &AdaptrixFlatAsset) -> LumeResult<Self> {
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

pub struct AdaptrixRenderer<D: Device> {
    pub culling_pipeline: D::ComputePipeline,
    pub visbuffer_pipeline: D::GraphicsPipeline,
    pub resolve_pipeline: D::GraphicsPipeline,
    pub culling_layout: D::PipelineLayout,
    pub visbuffer_layout: D::PipelineLayout,
    pub resolve_layout: D::PipelineLayout,
}

impl<D: Device> AdaptrixRenderer<D> {
    // 这里我们传入编译好的 Shader 字节码
    pub fn new(
        device: &D, 
        cull_spv: &[u32], 
        vis_vert_spv: &[u32], 
        vis_frag_spv: &[u32],
        resolve_vert_spv: &[u32], 
        resolve_frag_spv: &[u32],
        vis_layout: D::PipelineLayout,
        resolve_layout: D::PipelineLayout,
        vis_pass: &D::RenderPass,
        resolve_pass: &D::RenderPass,
    ) -> LumeResult<Self> {
        // 1. Create Pipelines
        let cull_mod = device.create_shader_module(cull_spv)?;
        let culling_pipeline = device.create_compute_pipeline(ComputePipelineDescriptor {
            shader: &cull_mod,
            layout: &vis_layout, 
        })?;

        let vis_vert = device.create_shader_module(vis_vert_spv)?;
        let vis_frag = device.create_shader_module(vis_frag_spv)?;
        
        let visbuffer_pipeline = device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            vertex_shader: &vis_vert,
            fragment_shader: &vis_frag,
            render_pass: vis_pass,
            layout: &vis_layout,
            primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList },
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
            primitive: PrimitiveState { topology: PrimitiveTopology::TriangleList },
            vertex_layout: None,
            depth_stencil: None,
        })?;

        Ok(Self {
            culling_pipeline,
            visbuffer_pipeline,
            resolve_pipeline,
            culling_layout: vis_layout.clone(),
            visbuffer_layout: vis_layout,
            resolve_layout: resolve_layout,
        })
    }
}