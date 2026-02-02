use crate::processor::types::*;
use crate::{AdaptrixVertex, ClusterPacked};
use meshopt::build_meshlets;
use std::collections::HashMap;

pub struct NaniteBuilder {
    pub vertices: Vec<AdaptrixVertex>,
    pub clusters: Vec<ClusterPacked>,
    pub meshlet_vertex_indices: Vec<u32>,
    pub meshlet_primitive_indices: Vec<u8>,
}

impl NaniteBuilder {
    pub fn new(vertices: Vec<AdaptrixVertex>) -> Self {
        Self {
            vertices,
            clusters: Vec::with_capacity(10000),
            meshlet_vertex_indices: Vec::with_capacity(100000),
            meshlet_primitive_indices: Vec::with_capacity(300000),
        }
    }

    pub fn build(&mut self, indices: &[u32]) -> AdaptrixFlatAsset {
        let mut current_level_indices = self.generate_level0(indices);
        
        let mut level = 0;
        while current_level_indices.len() > 1 {
            println!("Building Level {}: {} clusters", level, current_level_indices.len());
            let next_indices = self.build_next_level(current_level_indices.clone(), level);
            
            // 如果某一层无法再进行简化（可能是因为几何结构太散），则停止
            if next_indices.len() >= current_level_indices.len() {
                break;
            }
            current_level_indices = next_indices;
            level += 1;
        }

        AdaptrixFlatAsset {
            clusters: self.clusters.clone(),
            vertices: self.vertices.clone(),
            meshlet_vertex_indices: self.meshlet_vertex_indices.clone(),
            meshlet_primitive_indices: self.meshlet_primitive_indices.clone(),
        }
    }

    fn generate_level0(&mut self, indices: &[u32]) -> Vec<usize> {
        let meshlets = build_meshlets(indices, self.vertices.len(), 64, 124);
        let mut cluster_indices = Vec::with_capacity(meshlets.len());

        for m in meshlets.iter() {
            let flat_indices: &[u8] = bytemuck::cast_slice(m.indices.as_slice());
            let actual_indices = &flat_indices[.. (m.triangle_count as usize * 3)];
            let idx = self.push_cluster(m.vertices.as_slice(), actual_indices, 0.0, 1e10);
            cluster_indices.push(idx);
        }

        cluster_indices
    }

    fn build_next_level(&mut self, current_indices: Vec<usize>, level: u32) -> Vec<usize> {
        let mut cluster_vertices = Vec::with_capacity(current_indices.len());
        for &idx in &current_indices {
            let cluster = &self.clusters[idx];
            let start = cluster.vertex_offset as usize;
            let end = start + cluster.vertex_count as usize;
            cluster_vertices.push(self.meshlet_vertex_indices[start..end].to_vec());
        }

        // 提高分组规模：层级越高，分组越大，越利于收敛
        let group_size = if level < 2 { 8 } else { 12 }; 
        let adj = crate::processor::partitioner::build_adjacency(&current_indices, &cluster_vertices);
        let groups = crate::processor::partitioner::partition_clusters(&current_indices, &adj, group_size);
        
        let mut next_level_indices = Vec::with_capacity(groups.len());

        let mut total_simplified_tris = 0;
        let mut total_original_tris = 0;

        for group in groups {
            let mut group_vertices = Vec::new();
            let mut group_indices = Vec::new();
            let mut vertex_map = HashMap::new();
            let mut group_to_global_map = Vec::new();

            for &c_idx in &group.cluster_indices {
                let cluster = &self.clusters[c_idx];
                let v_start = cluster.vertex_offset as usize;
                let t_start = cluster.triangle_offset as usize;
                
                for i in 0..(cluster.triangle_count as usize * 3) {
                    let local_v_idx = self.meshlet_primitive_indices[t_start + i] as usize;
                    let global_v_idx = self.meshlet_vertex_indices[v_start + local_v_idx];
                    let v = self.vertices[global_v_idx as usize];

                    // 关键改进：按位置焊接顶点 (Weld by position)
                    // 我们使用一个简单的 Hash 键：将坐标乘以 1000 取整
                    let weld_key = [
                        (v.position[0] * 1000.0) as i32,
                        (v.position[1] * 1000.0) as i32,
                        (v.position[2] * 1000.0) as i32,
                    ];
                    
                    let new_idx = *vertex_map.entry(weld_key).or_insert_with(|| {
                        let idx = group_vertices.len() as u32;
                        group_vertices.push(v);
                        group_to_global_map.push(global_v_idx);
                        idx
                    });
                    group_indices.push(new_idx);
                }
            }

            total_original_tris += group_indices.len() / 3;

            // 误差增长曲线更陡峭一点：0.01, 0.02, 0.04, 0.08, 0.16...
            let error_threshold = 0.01 * (2.0f32.powi(level as i32));
            // 目标简化到 50%，如果是高层级，可以尝试简化更多
            let reduction_ratio = if level < 3 { 0.5 } else { 0.25 };
            let target_tris = ((group_indices.len() / 3) as f32 * reduction_ratio) as usize;
            let target_tris = target_tris.max(1);

            let simplified = crate::processor::simplifier::simplify_group(&group_vertices, &group_indices, target_tris, error_threshold);
            
            total_simplified_tris += simplified.indices.len() / 3;
            let current_lod_error = simplified.error; 
            
            // 重新切分
            let next_meshlets = build_meshlets(&simplified.indices, group_vertices.len(), 64, 124);

            for m in next_meshlets.iter() {
                let mut parent_v_indices = Vec::new();
                for &local_v in m.vertices.as_slice() {
                    parent_v_indices.push(group_to_global_map[local_v as usize]); 
                }

                let flat_tris: &[u8] = bytemuck::cast_slice(m.indices.as_slice());
                let actual_tris = &flat_tris[.. (m.triangle_count as usize * 3)];
                
                let idx = self.push_cluster(&parent_v_indices, actual_tris, current_lod_error, 1e10);
                
                for &child_idx in &group.cluster_indices {
                    if self.clusters[child_idx].parent_error >= 1e9 {
                        self.clusters[child_idx].parent_error = current_lod_error;
                    }
                }
                next_level_indices.push(idx);
            }
        }
        
        let ratio = total_simplified_tris as f32 / total_original_tris as f32;
        println!("Level {} Summary: Tris {} -> {}, Ratio: {:.2}", level, total_original_tris, total_simplified_tris, ratio);
        
        next_level_indices
    }

    fn push_cluster(&mut self, local_verts: &[u32], local_tris: &[u8], lod_error: f32, parent_error: f32) -> usize {
        let v_offset = self.meshlet_vertex_indices.len() as u32;
        let t_offset = self.meshlet_primitive_indices.len() as u32;

        self.meshlet_vertex_indices.extend_from_slice(local_verts);
        self.meshlet_primitive_indices.extend_from_slice(local_tris);
        
        let mut center = glam::Vec3::ZERO;
        if !local_verts.is_empty() {
            for &v_idx in local_verts {
                center += glam::Vec3::from_slice(&self.vertices[v_idx as usize].position);
            }
            center /= local_verts.len() as f32;
        }

        let mut radius: f32 = 0.0;
        for &v_idx in local_verts {
            radius = radius.max(center.distance(glam::Vec3::from_slice(&self.vertices[v_idx as usize].position)));
        }

        while self.meshlet_primitive_indices.len() % 4 != 0 {
            self.meshlet_primitive_indices.push(0);
        }

        let cluster = ClusterPacked {
            center_radius: [center.x, center.y, center.z, radius],
            vertex_offset: v_offset,
            triangle_offset: t_offset,
            vertex_count: local_verts.len() as u8,
            triangle_count: (local_tris.len() / 3) as u8,
            _pad1: 0,
            lod_error,
            parent_error,
        };

        let idx = self.clusters.len();
        self.clusters.push(cluster);
        idx
    }
}