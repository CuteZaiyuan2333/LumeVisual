use crate::AdaptrixVertex;
use meshopt::{simplify, simplify_sloppy, VertexDataAdapter};

pub struct SimplifiedMesh {
    pub vertices: Vec<AdaptrixVertex>,
    pub indices: Vec<u32>,
    pub error: f32,
}

pub fn simplify_group(
    vertices: &[AdaptrixVertex],
    indices: &[u32],
    target_count: usize,
    error_threshold: f32,
) -> SimplifiedMesh {
    let positions: Vec<f32> = vertices.iter().flat_map(|v| v.position).collect();
    let adapter = VertexDataAdapter::new(bytemuck::cast_slice(&positions), 12, 0).unwrap();
    
    // 1. 尝试标准简化 (维持拓扑)
    let mut simplified_indices = simplify(indices, &adapter, target_count, error_threshold);
    
    // 2. 如果简化效果不佳 (例如减少不到 20%), 则执行粗暴简化 (允许拓扑变化)
    if simplified_indices.len() > (indices.len() as f32 * 0.8) as usize {
        simplified_indices = simplify_sloppy(&indices, &adapter, target_count);
    }
    
    let error = if indices.len() > 0 {
        // 粗略估计误差
        let ratio = simplified_indices.len() as f32 / indices.len() as f32;
        (1.0 - ratio).max(0.0) * error_threshold + (if ratio > 0.8 { 0.1 } else { 0.0 })
    } else {
        0.0
    };

    SimplifiedMesh {
        vertices: vertices.to_vec(),
        indices: simplified_indices,
        error,
    }
}

