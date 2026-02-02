use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use serde::{Serialize, Deserialize};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Serialize, Deserialize)]
pub struct ClusterPacked {
    pub center_radius: [f32; 4], // 16
    pub vertex_offset: u32,      // 4
    pub triangle_offset: u32,    // 4
    pub counts: u32,             // 4 (v8, t8, pad16)
    pub lod_error: f32,          // 4
    pub parent_error: f32,       // 4
    pub _padding: [u32; 3],      // 12 (补齐到 48 字节)
}

#[derive(Clone, Debug)]
pub struct ClusterGeometry {
    pub vertices: Vec<u32>, // Indices into global vertex buffer
    pub indices: Vec<u8>,   // Local indices (0-255)
    pub group_id: u32,
}

pub struct NaniteNode {
    pub clusters: Vec<u32>, // Global indices of clusters in this node
    pub bounding_sphere: Vec4,
    pub lod_error: f32,
    pub parent_error: f32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct AdaptrixFlatAsset {
    pub clusters: Vec<ClusterPacked>,
    pub vertices: Vec<crate::AdaptrixVertex>,
    pub meshlet_vertex_indices: Vec<u32>,
    pub meshlet_primitive_indices: Vec<u8>,
}
