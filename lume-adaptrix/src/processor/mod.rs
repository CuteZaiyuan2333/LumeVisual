pub mod types;
pub mod partitioner;
pub mod simplifier;
pub mod builder;

pub use types::*;
pub use builder::NaniteBuilder;

pub fn process_mesh(
    positions: &[f32],
    normals: &[f32],
    uvs: &[f32],
    indices: &[u32],
) -> (AdaptrixFlatAsset, u32) {
    let mut vertices = Vec::new();
    let vertex_count = positions.len() / 3;
    
    for i in 0..vertex_count {
        vertices.push(crate::AdaptrixVertex {
            position: [positions[i*3], positions[i*3+1], positions[i*3+2]],
            normal: if !normals.is_empty() { [normals[i*3], normals[i*3+1], normals[i*3+2]] } else { [0.0, 1.0, 0.0] },
            uv: if !uvs.is_empty() { [uvs[i*2], uvs[i*2+1]] } else { [0.0, 0.0] },
        });
    }

    let builder = NaniteBuilder::new(vertices);
    builder.build(indices)
}
