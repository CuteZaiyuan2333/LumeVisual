pub mod processor;
pub mod renderer;

pub use processor::types::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, serde::Serialize, serde::Deserialize)]
pub struct AdaptrixVertex {
    pub position: [f32; 3], // 12字节
    pub normal: [f32; 3],   // 12字节
    pub uv: [f32; 2],       // 8字节，总计 32 字节
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshInstance {
    pub world_from_local: glam::Mat4,
    pub cluster_base: u32,
    pub cluster_count: u32,
    pub _padding: [u32; 2],
}