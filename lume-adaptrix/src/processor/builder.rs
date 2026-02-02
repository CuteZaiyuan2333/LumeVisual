use crate::processor::types::*;
use crate::{AdaptrixVertex, ClusterPacked};
use meshopt::build_meshlets;
use std::collections::HashMap;
use std::sync::Mutex;
use rayon::prelude::*;

pub struct NaniteBuilder {
    pub vertices: Vec<AdaptrixVertex>,
    pub clusters_mutex: Mutex<Vec<ClusterPacked>>,
    pub meshlet_vertex_indices_mutex: Mutex<Vec<u32>>,
    pub meshlet_primitive_indices_mutex: Mutex<Vec<u8>>,
}

impl NaniteBuilder {
    pub fn new(vertices: Vec<AdaptrixVertex>) -> Self {
        Self {
            vertices,
            clusters_mutex: Mutex::new(Vec::with_capacity(10000)),
            meshlet_vertex_indices_mutex: Mutex::new(Vec::with_capacity(100000)),
            meshlet_primitive_indices_mutex: Mutex::new(Vec::with_capacity(300000)),
        }
    }

    pub fn build(self, indices: &[u32]) -> AdaptrixFlatAsset {
        let mut current_level_indices = self.generate_level0(indices);
        
        let mut level = 0;
        while current_level_indices.len() > 1 {
            println!("Building Level {}: {} clusters", level, current_level_indices.len());
            let next_indices = self.build_next_level(current_level_indices.clone(), level);
            
            if next_indices.len() >= current_level_indices.len() {
                break;
            }
            current_level_indices = next_indices;
            level += 1;
        }

        AdaptrixFlatAsset {
            clusters: self.clusters_mutex.into_inner().unwrap(),
            vertices: self.vertices,
            meshlet_vertex_indices: self.meshlet_vertex_indices_mutex.into_inner().unwrap(),
            meshlet_primitive_indices: self.meshlet_primitive_indices_mutex.into_inner().unwrap(),
        }
    }

    fn generate_level0(&self, indices: &[u32]) -> Vec<usize> {
        let meshlets = build_meshlets(indices, self.vertices.len(), 64, 124);
        let mut cluster_indices = Vec::with_capacity(meshlets.len());

        for m in meshlets.iter() {
            let flat_indices: &[u8] = bytemuck::cast_slice(m.indices.as_slice());
            let actual_indices = &flat_indices[.. (m.triangle_count as usize * 3)];
            let idx = self.push_cluster_thread_safe(m.vertices.as_slice(), actual_indices, 0.0, 1e10);
            cluster_indices.push(idx);
        }

        cluster_indices
    }

    fn build_next_level(&self, current_indices: Vec<usize>, level: u32) -> Vec<usize> {
        // 由于需要访问 meshlet_vertex_indices 和 meshlet_primitive_indices，我们需要先锁定
        let meshlet_v_indices = self.meshlet_vertex_indices_mutex.lock().unwrap();
        let meshlet_p_indices = self.meshlet_primitive_indices_mutex.lock().unwrap();

        let mut cluster_vertices = Vec::with_capacity(current_indices.len());
        for &idx in &current_indices {
            let clusters = self.clusters_mutex.lock().unwrap();
            let cluster = &clusters[idx];
            let start = cluster.vertex_offset as usize;
            let vertex_count = (cluster.counts & 0xFF) as usize;
            let end = start + vertex_count;
            cluster_vertices.push(meshlet_v_indices[start..end].to_vec());
        }
        // 释放锁
        drop(meshlet_v_indices);
        drop(meshlet_p_indices);

        let group_size = if level < 2 { 8 } else { 12 }; 
        let adj = crate::processor::partitioner::build_adjacency(&current_indices, &cluster_vertices);
        let groups = crate::processor::partitioner::partition_clusters(&current_indices, &adj, group_size);
        
        let next_level_indices_mutex = Mutex::new(Vec::with_capacity(groups.len()));
        let total_original_tris = std::sync::atomic::AtomicUsize::new(0);
        let total_simplified_tris = std::sync::atomic::AtomicUsize::new(0);

        groups.into_par_iter().for_each(|group| {
            let mut group_vertices = Vec::new();
            let mut group_indices = Vec::new();
            let mut vertex_map = HashMap::new();
            let mut group_to_global_map = Vec::new();

            // 重新加锁读取
            let meshlet_v_indices = self.meshlet_vertex_indices_mutex.lock().unwrap();
            let meshlet_p_indices = self.meshlet_primitive_indices_mutex.lock().unwrap();
            let clusters_read = self.clusters_mutex.lock().unwrap();

            for &c_idx in &group.cluster_indices {
                let cluster = &clusters_read[c_idx];
                let v_start = cluster.vertex_offset as usize;
                let t_start = cluster.triangle_offset as usize;
                let triangle_count = ((cluster.counts >> 8) & 0xFF) as usize;
                
                for i in 0..(triangle_count * 3) {
                    let tri_byte_offset = t_start + i;
                    let local_v_idx = meshlet_p_indices[tri_byte_offset];

                    let global_v_idx = meshlet_v_indices[v_start + local_v_idx as usize];
                    let v = self.vertices[global_v_idx as usize];

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
            drop(meshlet_v_indices);
            drop(meshlet_p_indices);
            
            // 计算子节点的平均误差
            let mut max_child_error = 0.0f32;
            for &c_idx in &group.cluster_indices {
                max_child_error = max_child_error.max(clusters_read[c_idx].lod_error);
            }
            drop(clusters_read);

            total_original_tris.fetch_add(group_indices.len() / 3, std::sync::atomic::Ordering::Relaxed);

            let error_threshold = 0.01 * (2.0f32.powi(level as i32));
            let reduction_ratio = if level < 3 { 0.5 } else { 0.25 };
            let target_tris = ((group_indices.len() / 3) as f32 * reduction_ratio) as usize;
            let target_tris = target_tris.max(1);

            let locked = vec![false; group_vertices.len()];
            let simplified = crate::processor::simplifier::simplify_group(&group_vertices, &group_indices, target_tris, error_threshold, &locked);
            
            total_simplified_tris.fetch_add(simplified.indices.len() / 3, std::sync::atomic::Ordering::Relaxed);
            
            // 改进：误差是累加的，确保每一层都比下一层误差大
            let current_lod_error = max_child_error + simplified.error + 0.001; 
            
            let next_meshlets = build_meshlets(&simplified.indices, group_vertices.len(), 64, 124);

            let mut local_next_indices = Vec::new();
            for m in next_meshlets.iter() {
                let mut parent_v_indices = Vec::new();
                for &local_v in m.vertices.as_slice() {
                    parent_v_indices.push(group_to_global_map[local_v as usize]); 
                }

                let flat_tris: &[u8] = bytemuck::cast_slice(m.indices.as_slice());
                let actual_tris = &flat_tris[.. (m.triangle_count as usize * 3)];
                
                let idx = self.push_cluster_thread_safe(&parent_v_indices, actual_tris, current_lod_error, 1e10);
                
                let mut clusters = self.clusters_mutex.lock().unwrap();
                for &child_idx in &group.cluster_indices {
                    if clusters[child_idx].parent_error >= 1e9 {
                        clusters[child_idx].parent_error = current_lod_error;
                    }
                }
                drop(clusters);
                local_next_indices.push(idx);
            }
            
            next_level_indices_mutex.lock().unwrap().extend(local_next_indices);
        });
        
        let next_level_indices = next_level_indices_mutex.into_inner().unwrap();
        let total_simplified = total_simplified_tris.load(std::sync::atomic::Ordering::Relaxed);
        let total_original = total_original_tris.load(std::sync::atomic::Ordering::Relaxed);
        let ratio = total_simplified as f32 / total_original as f32;
        println!("Level {} Summary: Tris {} -> {}, Ratio: {:.2}", level, total_original, total_simplified, ratio);
        
        next_level_indices
    }

    fn push_cluster_thread_safe(&self, local_verts: &[u32], local_tris: &[u8], lod_error: f32, parent_error: f32) -> usize {
        let mut v_indices = self.meshlet_vertex_indices_mutex.lock().unwrap();
        let mut p_indices = self.meshlet_primitive_indices_mutex.lock().unwrap();
        let mut clusters = self.clusters_mutex.lock().unwrap();

        let v_offset = v_indices.len() as u32;
        let t_offset = p_indices.len() as u32;

        v_indices.extend_from_slice(local_verts);
        p_indices.extend_from_slice(local_tris);
        
        while p_indices.len() % 4 != 0 {
            p_indices.push(0);
        }

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

        let counts = (local_verts.len() as u32 & 0xFF) | (( (local_tris.len() / 3) as u32 & 0xFF) << 8);

        let cluster = ClusterPacked {
            center_radius: [center.x, center.y, center.z, radius],
            vertex_offset: v_offset,
            triangle_offset: t_offset,
            counts,
            lod_error,
            parent_error,
            _padding: [0; 3],
        };

        let idx = clusters.len();
        clusters.push(cluster);
        idx
    }
}
