use std::collections::{HashMap, HashSet};

pub struct ClusterGroup {
    pub cluster_indices: Vec<usize>,
}

/// 改进的分组逻辑：确保完全使用局部索引进行邻居查找
pub fn partition_clusters(
    cluster_indices: &[usize],
    adjacencies: &HashMap<usize, HashSet<usize>>,
    target_group_size: usize,
) -> Vec<ClusterGroup> {
    let mut visited = HashSet::new();
    let mut groups = Vec::new();
    let index_set: HashSet<usize> = cluster_indices.iter().cloned().collect();

    for &start_idx in cluster_indices {
        if visited.contains(&start_idx) {
            continue;
        }

        let mut current_group = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start_idx);
        visited.insert(start_idx);

        while let Some(idx) = queue.pop_front() {
            current_group.push(idx);
            if current_group.len() >= target_group_size {
                break;
            }

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

pub fn build_adjacency(
    cluster_indices: &[usize],
    clusters_vertices: &[Vec<u32>],
) -> HashMap<usize, HashSet<usize>> {
    let mut vertex_to_clusters: HashMap<u32, Vec<usize>> = HashMap::new();
    
    // 安全检查：确保两个 slice 长度一致
    assert_eq!(cluster_indices.len(), clusters_vertices.len(), "Builder pass incorrect data to partitioner");

    for (local_idx, &global_cluster_idx) in cluster_indices.iter().enumerate() {
        for &v in &clusters_vertices[local_idx] {
            vertex_to_clusters.entry(v).or_default().push(global_cluster_idx);
        }
    }

    let mut adjacencies: HashMap<usize, HashSet<usize>> = HashMap::new();
    for (_, cluster_list) in vertex_to_clusters {
        for &c1 in &cluster_list {
            for &c2 in &cluster_list {
                if c1 != c2 {
                    adjacencies.entry(c1).or_default().insert(c2);
                }
            }
        }
    }
    adjacencies
}
