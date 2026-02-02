use std::collections::{HashMap, HashSet, VecDeque};

pub struct ClusterGroup {
    pub cluster_indices: Vec<usize>,
}

/// 核心优化：基于排序的邻居查找，内存占用极低，支持千万级顶点
pub fn build_adjacency(
    cluster_indices: &[usize],
    clusters_vertices: &[Vec<u32>],
) -> HashMap<usize, HashSet<usize>> {
    let mut entries = Vec::with_capacity(cluster_indices.len() * 64);
    for (local_idx, &global_cluster_idx) in cluster_indices.iter().enumerate() {
        for &v in &clusters_vertices[local_idx] {
            entries.push((v, global_cluster_idx));
        }
    }

    // 关键：按顶点 ID 排序
    entries.sort_unstable_by_key(|e| e.0);

    let mut adjacencies: HashMap<usize, HashSet<usize>> = HashMap::with_capacity(cluster_indices.len());
    
    // 线性扫描排序后的数组，找到共享相同顶点的集群对
    let mut i = 0;
    while i < entries.len() {
        let mut j = i + 1;
        while j < entries.len() && entries[j].0 == entries[i].0 {
            j += 1;
        }

        // entries[i..j] 全都共享同一个顶点
        if j - i > 1 {
            for k1 in i..j {
                for k2 in (k1 + 1)..j {
                    let c1 = entries[k1].1;
                    let c2 = entries[k2].1;
                    if c1 != c2 {
                        adjacencies.entry(c1).or_default().insert(c2);
                        adjacencies.entry(c2).or_default().insert(c1);
                    }
                }
            }
        }
        i = j;
    }
    adjacencies
}

pub fn partition_clusters(
    cluster_indices: &[usize],
    adjacencies: &HashMap<usize, HashSet<usize>>,
    target_group_size: usize,
) -> Vec<ClusterGroup> {
    let mut visited = HashSet::new();
    let mut groups = Vec::new();
    let index_set: HashSet<usize> = cluster_indices.iter().cloned().collect();

    for &start_idx in cluster_indices {
        if visited.contains(&start_idx) { continue; }

        let mut current_group = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start_idx);
        visited.insert(start_idx);

        while let Some(idx) = queue.pop_front() {
            current_group.push(idx);
            if current_group.len() >= target_group_size { break; }

            if let Some(neighbors) = adjacencies.get(&idx) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) && index_set.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if !current_group.is_empty() {
            groups.push(ClusterGroup { cluster_indices: current_group });
        }
    }
    groups
}