use bytemuck::{Pod, Zeroable, cast_slice};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::path::Path;
use memmap2::Mmap;
use anyhow::{Context, Result};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Serialize, Deserialize)]
pub struct ClusterPacked {
    pub center_radius: [f32; 4],
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub counts: u32,
    pub lod_error: f32,
    pub parent_error: f32,
    pub _padding: [u32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Serialize, Deserialize)]
pub struct LadHeader {
    pub magic: [u8; 4], // "LLAD"
    pub version: u32,
    pub num_clusters: u64,
    pub num_vertices: u64,
    pub num_v_indices: u64,
    pub num_p_indices: u64,
}

/// 仿 Nanite 零拷贝资产结构
pub struct AdaptrixAsset {
    _mmap: Mmap, 
    // 使用 Cow 或直接存指针。为了简单和安全，我们这里存储从 Mmap 派生的静态生命周期（实际受 _mmap 控制）
    pub clusters: &'static [ClusterPacked],
    pub vertices: &'static [crate::AdaptrixVertex],
    pub meshlet_vertex_indices: &'static [u32],
    pub meshlet_primitive_indices: &'static [u8],
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct AdaptrixFlatAsset {
    pub clusters: Vec<ClusterPacked>,
    pub vertices: Vec<crate::AdaptrixVertex>,
    pub meshlet_vertex_indices: Vec<u32>,
    pub meshlet_primitive_indices: Vec<u8>,
}

impl AdaptrixAsset {
    /// 真正的零拷贝加载：直接映射磁盘二进制块到内存
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open asset file: {:?}", path.as_ref()))?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        if mmap.len() < std::mem::size_of::<LadHeader>() {
            anyhow::bail!("File too small to be a LAD file");
        }

        let header = unsafe { &*(mmap.as_ptr() as *const LadHeader) };
        if &header.magic != b"LLAD" {
            anyhow::bail!("Invalid LAD magic header. Did you re-process the model?");
        }

        let mut offset = std::mem::size_of::<LadHeader>();

        let clusters_ptr = unsafe { mmap.as_ptr().add(offset) as *const ClusterPacked };
        let clusters = unsafe { std::slice::from_raw_parts(clusters_ptr, header.num_clusters as usize) };
        offset += header.num_clusters as usize * std::mem::size_of::<ClusterPacked>();

        let vertices_ptr = unsafe { mmap.as_ptr().add(offset) as *const crate::AdaptrixVertex };
        let vertices = unsafe { std::slice::from_raw_parts(vertices_ptr, header.num_vertices as usize) };
        offset += header.num_vertices as usize * std::mem::size_of::<crate::AdaptrixVertex>();

        let v_idx_ptr = unsafe { mmap.as_ptr().add(offset) as *const u32 };
        let vertex_indices = unsafe { std::slice::from_raw_parts(v_idx_ptr, header.num_v_indices as usize) };
        offset += header.num_v_indices as usize * 4;

        let p_idx_ptr = unsafe { mmap.as_ptr().add(offset) as *const u8 };
        let primitive_indices = unsafe { std::slice::from_raw_parts(p_idx_ptr, header.num_p_indices as usize) };

        // 核心安全说明：虽然我们 transmute 为 'static，但它们被封装在 AdaptrixAsset 中
        // 只要 AdaptrixAsset 存在，这些 slice 就有效。
        Ok(Self {
            _mmap: mmap,
            clusters: unsafe { std::mem::transmute(clusters) },
            vertices: unsafe { std::mem::transmute(vertices) },
            meshlet_vertex_indices: unsafe { std::mem::transmute(vertex_indices) },
            meshlet_primitive_indices: unsafe { std::mem::transmute(primitive_indices) },
        })
    }

    /// 将 Flat 资产保存为高效的二进制 LAD 格式
    pub fn save_to_file<P: AsRef<Path>>(asset: &AdaptrixFlatAsset, path: P) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
        use std::io::Write;

        let header = LadHeader {
            magic: *b"LLAD",
            version: 1,
            num_clusters: asset.clusters.len() as u64,
            num_vertices: asset.vertices.len() as u64,
            num_v_indices: asset.meshlet_vertex_indices.len() as u64,
            num_p_indices: asset.meshlet_primitive_indices.len() as u64,
        };

        writer.write_all(cast_slice(&[header]))?;
        writer.write_all(cast_slice(&asset.clusters))?;
        writer.write_all(cast_slice(&asset.vertices))?;
        writer.write_all(cast_slice(&asset.meshlet_vertex_indices))?;
        writer.write_all(cast_slice(&asset.meshlet_primitive_indices))?;

        writer.flush()?;
        Ok(())
    }
}