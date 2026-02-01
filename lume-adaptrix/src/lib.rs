use bytemuck::{Pod, Zeroable};
use glam::{Vec4, Mat4};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Cluster {
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub bounding_sphere: Vec4, // 16字节
    pub error_metric: f32,     // 4字节
    pub parent_error: f32,     // 4字节
    pub _padding: [f32; 2],    // 8字节，使总大小对齐到 16 的倍数 (48字节)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct AdaptrixVertex {
    pub position: [f32; 3], // 12字节
    pub normal: [f32; 3],   // 12字节
    pub uv: [f32; 2],       // 8字节，总计 32 字节 (完美对齐)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MeshInstance {
    pub world_from_local: Mat4,
    pub cluster_base: u32,
    pub cluster_count: u32,
    pub _padding: [u32; 2],
}

pub struct AdaptrixMesh {
    pub clusters: Vec<Cluster>,
    pub vertices: Vec<AdaptrixVertex>,
    pub indices: Vec<u32>,
}