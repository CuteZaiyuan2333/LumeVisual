use std::collections::VecDeque;

pub struct ClusterGroup {
    pub cluster_indices: Vec<u32>,
}

/// 仿 Nanite 工业级 CSR 邻居结构
pub struct Adjacency {
    pub offsets: Vec<u32>,
    pub neighbors: Vec<u32>,
}

impl Adjacency {
    pub fn get_neighbors(&self, cluster_idx: u32) -> &[u32] {
        let start = self.offsets[cluster_idx as usize] as usize;
        let end = self.offsets[cluster_idx as usize + 1] as usize;
        &self.neighbors[start..end]
    }
}

pub fn build_adjacency(
    num_clusters: usize,
    cluster_indices: &[u32],
    meshlet_vertex_indices: &[u32],
    cluster_vertex_offsets: &[(u32, u32)],
) -> Adjacency {
    // 1. 构建 (VertexID, ClusterID) 对
    let mut entries = Vec::with_capacity(cluster_indices.len() * 64);
    for &global_idx in cluster_indices {
        let (offset, count) = cluster_vertex_offsets[global_idx as usize];
        for i in 0..count {
            let v = meshlet_vertex_indices[(offset + i) as usize];
            entries.push((v, global_idx));
        }
    }

    // 2. 按顶点 ID 排序
    entries.sort_unstable_by_key(|e| e.0);

    // 3. 线性邻居提取 (O(M) 复杂度，彻底解决 OOM)
    let mut raw_adj = Vec::with_capacity(entries.len());
    let mut i = 0;
    while i < entries.len() {
        let mut j = i + 1;
        while j < entries.len() && entries[j].0 == entries[i].0 {
            j += 1;
        }
        // 关键改进：只建立相邻 Cluster 的连接 (C1-C2, C2-C3...)
        // 这足以维持图的连通性，且边数量仅为 M-1 而不是 M(M-1)
        if j - i > 1 {
            for k in i..(j-1) {
                let c1 = entries[k].1;
                let c2 = entries[k+1].1;
                if c1 != c2 {
                    raw_adj.push((c1.min(c2), c1.max(c2)));
                }
            }
        }
        i = j;
    }

    // 4. 排序并去重
    raw_adj.sort_unstable();
    raw_adj.dedup();

    // 5. 转换为 CSR 格式
    let mut offsets = vec![0u32; num_clusters + 1];
    for &(c1, c2) in &raw_adj {
        offsets[c1 as usize + 1] += 1;
        offsets[c2 as usize + 1] += 1;
    }

    // 前缀和
    for i in 0..num_clusters {
        offsets[i + 1] += offsets[i];
    }

    let mut current_offsets = offsets.clone();
    let mut neighbors = vec![0u32; (raw_adj.len() * 2) as usize];
    for (c1, c2) in raw_adj {
        neighbors[current_offsets[c1 as usize] as usize] = c2;
        current_offsets[c1 as usize] += 1;
        neighbors[current_offsets[c2 as usize] as usize] = c1;
        current_offsets[c2 as usize] += 1;
    }

    Adjacency { offsets, neighbors }
}

pub fn partition_clusters(
    num_clusters: usize,
    cluster_indices: &[u32],
    adj: &Adjacency,
    target_group_size: usize,
) -> Vec<ClusterGroup> {
    // 使用 BitSet 代替 HashSet，内存占用降低 64 倍
    let mut visited = vec![0u64; (num_clusters + 63) / 64];
    let mut groups = Vec::new();
    
    // 快速索引集
    let mut in_current_level = vec![0u64; (num_clusters + 63) / 64];
    for &idx in cluster_indices {
        in_current_level[idx as usize / 64] |= 1 << (idx as usize % 64);
    }

    let is_visited = |v: &[u64], i: usize| (v[i / 64] & (1 << (i % 64))) != 0;
    let set_visited = |v: &mut [u64], i: usize| v[i / 64] |= 1 << (i % 64);

    for &start_idx in cluster_indices {
        if is_visited(&visited, start_idx as usize) { continue; }

        let mut current_group = Vec::with_capacity(target_group_size);
        let mut queue = VecDeque::with_capacity(target_group_size * 2);
        
        queue.push_back(start_idx);
        set_visited(&mut visited, start_idx as usize);

        while let Some(idx) = queue.pop_front() {
            current_group.push(idx);
            if current_group.len() >= target_group_size { break; }

            for &neighbor in adj.get_neighbors(idx) {
                if !is_visited(&visited, neighbor as usize) && is_visited(&in_current_level, neighbor as usize) {
                    set_visited(&mut visited, neighbor as usize);
                    queue.push_back(neighbor);
                }
            }
        }

        if !current_group.is_empty() {
            groups.push(ClusterGroup { cluster_indices: current_group });
        }
    }
    groups
}