use crate::processor::types::*;
use crate::{AdaptrixVertex, ClusterPacked};
use meshopt::build_meshlets;
use std::collections::HashMap;
use rayon::prelude::*;

pub struct NaniteBuilder {
    pub vertices: Vec<AdaptrixVertex>,
    // 基础仓库
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

    pub fn build(mut self, indices: &[u32]) -> AdaptrixFlatAsset {
        let mut current_level_indices = self.generate_level0(indices);
        
        let mut level = 0;
        while current_level_indices.len() > 1 {
            println!("Building Level {}: {} clusters", level, current_level_indices.len());
            
            // 并行生成本层级的新 Cluster
            let (next_level_indices, level_data) = self.build_next_level_zero_lock(&current_level_indices, level);
            
            if next_level_indices.is_empty() { break; }

            // 汇总本层数据
            let mut v_base = self.meshlet_vertex_indices.len() as u32;
            let mut t_base = self.meshlet_primitive_indices.len() as u32;

            for ld in level_data {
                // 更新子节点的 parent_error
                for &child_idx in &ld.children {
                    self.clusters[child_idx].parent_error = ld.parent_error;
                }

                // 修正偏移并存入全局仓库
                for mut c in ld.new_clusters {
                    c.vertex_offset += v_base;
                    c.triangle_offset += t_base;
                    self.clusters.push(c);
                }
                self.meshlet_vertex_indices.extend(ld.new_v_indices);
                self.meshlet_primitive_indices.extend(ld.new_p_indices);
                
                v_base = self.meshlet_vertex_indices.len() as u32;
                t_base = self.meshlet_primitive_indices.len() as u32;
            }

            if next_level_indices.len() >= current_level_indices.len() { break; }
            current_level_indices = next_level_indices;
            level += 1;
        }

        AdaptrixFlatAsset {
            clusters: self.clusters,
            vertices: self.vertices,
            meshlet_vertex_indices: self.meshlet_vertex_indices,
            meshlet_primitive_indices: self.meshlet_primitive_indices,
        }
    }

    fn generate_level0(&mut self, indices: &[u32]) -> Vec<usize> {
        let meshlets = build_meshlets(indices, self.vertices.len(), 64, 124);
        let mut cluster_indices = Vec::with_capacity(meshlets.len());

        for m in meshlets.iter() {
            let idx = self.clusters.len();
            let v_offset = self.meshlet_vertex_indices.len() as u32;
            let t_offset = self.meshlet_primitive_indices.len() as u32;

            self.meshlet_vertex_indices.extend_from_slice(m.vertices.as_slice());
            let flat_indices: &[u8] = bytemuck::cast_slice(m.indices.as_slice());
            self.meshlet_primitive_indices.extend_from_slice(&flat_indices[.. (m.triangle_count as usize * 3)]);
            while self.meshlet_primitive_indices.len() % 4 != 0 { self.meshlet_primitive_indices.push(0); }

            let center_radius = self.calculate_bounding_sphere(m.vertices.as_slice());
            self.clusters.push(ClusterPacked {
                center_radius,
                vertex_offset: v_offset,
                triangle_offset: t_offset,
                counts: (m.vertices.len() as u32 & 0xFF) | ((m.triangle_count as u32 & 0xFF) << 8),
                lod_error: 0.0,
                parent_error: 1e10,
                _padding: [0; 3],
            });
            cluster_indices.push(idx);
        }
        cluster_indices
    }

    fn build_next_level_zero_lock(&self, current_indices: &[usize], level: u32) -> (Vec<usize>, Vec<LevelGroupResult>) {
        // 预提取本层顶点数据，供并行任务只读访问
        let cluster_vertices: Vec<Vec<u32>> = current_indices.iter().map(|&idx| {
            let c = &self.clusters[idx];
            self.meshlet_vertex_indices[c.vertex_offset as usize .. (c.vertex_offset + (c.counts & 0xFF)) as usize].to_vec()
        }).collect();

        let adj = crate::processor::partitioner::build_adjacency(current_indices, &cluster_vertices);
        let groups = crate::processor::partitioner::partition_clusters(current_indices, &adj, if level < 2 { 8 } else { 12 });

        let results: Vec<LevelGroupResult> = groups.into_par_iter().map(|group| {
            let mut group_vertices = Vec::new();
            let mut group_indices = Vec::new();
            let mut vertex_map = HashMap::new();
            let mut group_to_global_map = Vec::new();
            let mut max_child_error = 0.0f32;

            for &c_idx in &group.cluster_indices {
                let cluster = &self.clusters[c_idx];
                max_child_error = max_child_error.max(cluster.lod_error);
                let v_start = cluster.vertex_offset as usize;
                let t_start = cluster.triangle_offset as usize;
                let triangle_count = ((cluster.counts >> 8) & 0xFF) as usize;
                
                for i in 0..(triangle_count * 3) {
                    let local_v_idx = self.meshlet_primitive_indices[t_start + i];
                    let global_v_idx = self.meshlet_vertex_indices[v_start + local_v_idx as usize];
                    let v = self.vertices[global_v_idx as usize];

                    let weld_key = [(v.position[0]*1000.0) as i32, (v.position[1]*1000.0) as i32, (v.position[2]*1000.0) as i32];
                    let new_idx = *vertex_map.entry(weld_key).or_insert_with(|| {
                        let idx = group_vertices.len() as u32;
                        group_vertices.push(v);
                        group_to_global_map.push(global_v_idx);
                        idx
                    });
                    group_indices.push(new_idx);
                }
            }

            let reduction = if level < 3 { 0.5 } else { 0.25 };
            let target = (((group_indices.len() / 3) as f32 * reduction) as usize).max(1);
            let locked = vec![false; group_vertices.len()];
            let simplified = crate::processor::simplifier::simplify_group(&group_vertices, &group_indices, target, 0.01 * (2.0f32.powi(level as i32)), &locked);
            
            let mut res = LevelGroupResult {
                new_clusters: Vec::new(),
                new_v_indices: Vec::new(),
                new_p_indices: Vec::new(),
                children: group.cluster_indices,
                parent_error: max_child_error + simplified.error + 0.001,
            };

            let next_meshlets = build_meshlets(&simplified.indices, group_vertices.len(), 64, 124);
            for m in next_meshlets.iter() {
                let v_off = res.new_v_indices.len() as u32;
                let t_off = res.new_p_indices.len() as u32;
                for &lv in m.vertices.as_slice() { res.new_v_indices.push(group_to_global_map[lv as usize]); }
                let tris: &[u8] = bytemuck::cast_slice(m.indices.as_slice());
                res.new_p_indices.extend_from_slice(&tris[.. (m.triangle_count as usize * 3)]);
                while res.new_p_indices.len() % 4 != 0 { res.new_p_indices.push(0); }

                res.new_clusters.push(ClusterPacked {
                    center_radius: self.calculate_bounding_sphere(&res.new_v_indices[v_off as usize ..]),
                    vertex_offset: v_off,
                    triangle_offset: t_off,
                    counts: (m.vertices.len() as u32 & 0xFF) | ((m.triangle_count as u32 & 0xFF) << 8),
                    lod_error: res.parent_error,
                    parent_error: 1e10,
                    _padding: [0; 3],
                });
            }
            res
        }).collect();

        // 汇总下一层的全局索引
        let mut next_indices = Vec::new();
        let mut current_global_count = self.clusters.len();
        for ld in &results {
            for _ in 0..ld.new_clusters.len() {
                next_indices.push(current_global_count);
                current_global_count += 1;
            }
        }

        (next_indices, results)
    }

    fn calculate_bounding_sphere(&self, verts: &[u32]) -> [f32; 4] {
        if verts.is_empty() { return [0.0; 4]; }
        let mut center = glam::Vec3::ZERO;
        for &idx in verts { center += glam::Vec3::from_slice(&self.vertices[idx as usize].position); }
        center /= verts.len() as f32;
        let mut radius = 0.0f32;
        for &idx in verts { radius = radius.max(center.distance(glam::Vec3::from_slice(&self.vertices[idx as usize].position))); }
        [center.x, center.y, center.z, radius]
    }
}

pub struct LevelGroupResult {
    pub new_clusters: Vec<ClusterPacked>,
    pub new_v_indices: Vec<u32>,
    pub new_p_indices: Vec<u8>,
    pub children: Vec<usize>,
    pub parent_error: f32,
}