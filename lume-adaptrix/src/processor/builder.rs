use crate::processor::types::*;
use crate::{AdaptrixVertex, ClusterPacked};
use meshopt::build_meshlets;
use std::collections::HashMap;
use rayon::prelude::*;

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

    pub fn build(mut self, indices: &[u32]) -> AdaptrixFlatAsset {
        let mut current_level_indices = self.generate_level0(indices);
        
        let mut level = 0;
        while current_level_indices.len() > 1 {
            println!("Building Level {}: {} clusters", level, current_level_indices.len());
            let (next_indices, level_data) = self.build_next_level_parallel(&current_level_indices, level);
            
            if next_indices.is_empty() { break; }

            // 汇总本层
            let v_base = self.meshlet_vertex_indices.len() as u32;
            let t_base = self.meshlet_primitive_indices.len() as u32;

            for ld in level_data {
                for &child_idx in &ld.children {
                    self.clusters[child_idx as usize].parent_error = ld.parent_error;
                }
                for mut c in ld.new_clusters {
                    c.vertex_offset += v_base;
                    c.triangle_offset += t_base;
                    self.clusters.push(c);
                }
                self.meshlet_vertex_indices.extend(ld.new_v_indices);
                self.meshlet_primitive_indices.extend(ld.new_p_indices);
            }

            if next_indices.len() >= current_level_indices.len() { break; }
            current_level_indices = next_indices;
            level += 1;
        }

        AdaptrixFlatAsset {
            clusters: self.clusters,
            vertices: self.vertices,
            meshlet_vertex_indices: self.meshlet_vertex_indices,
            meshlet_primitive_indices: self.meshlet_primitive_indices,
        }
    }

    fn generate_level0(&mut self, indices: &[u32]) -> Vec<u32> {
        let meshlets = build_meshlets(indices, self.vertices.len(), 64, 124);
        let mut cluster_indices = Vec::with_capacity(meshlets.len());

        for m in meshlets.iter() {
            let idx = self.clusters.len() as u32;
            let v_offset = self.meshlet_vertex_indices.len() as u32;
            let t_offset = self.meshlet_primitive_indices.len() as u32;

            self.meshlet_vertex_indices.extend_from_slice(m.vertices.as_slice());
            let flat_indices: &[u8] = bytemuck::cast_slice(m.indices.as_slice());
            self.meshlet_primitive_indices.extend_from_slice(&flat_indices[.. (m.triangle_count as usize * 3)]);
            while self.meshlet_primitive_indices.len() % 4 != 0 { self.meshlet_primitive_indices.push(0); }

            self.clusters.push(ClusterPacked {
                center_radius: self.calculate_bounding_sphere(m.vertices.as_slice()),
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

    fn build_next_level_parallel(&self, current_indices: &[u32], level: u32) -> (Vec<u32>, Vec<LevelGroupResult>) {
        let cluster_vertex_info: Vec<(u32, u32)> = (0..self.clusters.len()).map(|i| {
            let c = &self.clusters[i];
            (c.vertex_offset, c.counts & 0xFF)
        }).collect();

        let adj = crate::processor::partitioner::build_adjacency(
            self.clusters.len(),
            current_indices,
            &self.meshlet_vertex_indices,
            &cluster_vertex_info
        );

        let groups = crate::processor::partitioner::partition_clusters(self.clusters.len(), current_indices, &adj, if level < 2 { 8 } else { 12 });

        let results: Vec<LevelGroupResult> = groups.into_par_iter().map(|group| {
            let mut group_vertices = Vec::new();
            let mut group_indices = Vec::new();
            let mut vertex_map = HashMap::new();
            let mut group_to_global_map = Vec::new();
            let mut max_child_error = 0.0f32;

            for &c_idx in &group.cluster_indices {
                let cluster = &self.clusters[c_idx as usize];
                max_child_error = max_child_error.max(cluster.lod_error);
                let triangle_count = ((cluster.counts >> 8) & 0xFF) as usize;
                
                for i in 0..(triangle_count * 3) {
                    let local_v_idx = self.meshlet_primitive_indices[(cluster.triangle_offset as usize) + i];
                    let global_v_idx = self.meshlet_vertex_indices[(cluster.vertex_offset as usize) + local_v_idx as usize];
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
            let simplified = crate::processor::simplifier::simplify_group(
                &group_vertices, &group_indices, target,
                0.01 * (2.0f32.powi(level as i32)), &locked
            );
            
            let parent_error = max_child_error + simplified.error + 0.001;
            let mut res = LevelGroupResult {
                new_clusters: Vec::new(), new_v_indices: Vec::new(), new_p_indices: Vec::new(),
                children: group.cluster_indices, parent_error,
            };

            for m in build_meshlets(&simplified.indices, group_vertices.len(), 64, 124) {
                let v_off = res.new_v_indices.len() as u32;
                let t_off = res.new_p_indices.len() as u32;
                for &lv in m.vertices.as_slice() { res.new_v_indices.push(group_to_global_map[lv as usize]); }
                let tris: &[u8] = bytemuck::cast_slice(m.indices.as_slice());
                res.new_p_indices.extend_from_slice(&tris[.. (m.triangle_count as usize * 3)]);
                while res.new_p_indices.len() % 4 != 0 { res.new_p_indices.push(0); }

                res.new_clusters.push(ClusterPacked {
                    center_radius: self.calculate_bounding_sphere(&res.new_v_indices[v_off as usize ..]),
                    vertex_offset: v_off, triangle_offset: t_off,
                    counts: (m.vertices.len() as u32 & 0xFF) | ((m.triangle_count as u32 & 0xFF) << 8),
                    lod_error: parent_error, parent_error: 1e10, _padding: [0; 3],
                });
            }
            res
        }).collect();

        let mut next_indices = Vec::new();
        let mut current_global_count = self.clusters.len() as u32;
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
    pub children: Vec<u32>,
    pub parent_error: f32,
}