use crate::{AdaptrixFlatAsset, AdaptrixVertex, ClusterPacked};
use meshopt::{build_meshlets, compute_meshlet_bounds, VertexDataAdapter};

pub fn process_mesh(
    positions: &[f32],
    normals: &[f32],
    uvs: &[f32],
    indices: &[u32],
) -> AdaptrixFlatAsset {
    // 1. 构建顶点列表
    let mut vertices = Vec::new();
    let vertex_count = positions.len() / 3;
    
    for i in 0..vertex_count {
        vertices.push(AdaptrixVertex {
            position: [positions[i*3], positions[i*3+1], positions[i*3+2]],
            normal: if !normals.is_empty() { [normals[i*3], normals[i*3+1], normals[i*3+2]] } else { [0.0, 1.0, 0.0] },
            uv: if !uvs.is_empty() { [uvs[i*2], uvs[i*2+1]] } else { [0.0, 0.0] },
        });
    }

    // 2. 生成 Meshlets
    // max_vertices: 64-128
    // max_triangles: 124-256
    let max_vertices = 64;
    let max_triangles = 124; 
    
    // meshopt::build_meshlets (indices, vertex_count, max_vertices, max_triangles)
    let meshlets = build_meshlets(&indices, vertices.len(), max_vertices, max_triangles);

    let mut global_clusters = Vec::new();
    let mut global_meshlet_vertex_indices = Vec::new();
    let mut global_meshlet_primitive_indices = Vec::new();

    // Adapter for bounds
    let adapter = VertexDataAdapter::new(bytemuck::cast_slice(positions), std::mem::size_of::<f32>() * 3, 0).unwrap();

    for meshlet in meshlets.iter() {
        let bounds = compute_meshlet_bounds(meshlet, &adapter);
        
        let vertex_offset = global_meshlet_vertex_indices.len() as u32;
        let triangle_offset = global_meshlet_primitive_indices.len() as u32;

        // 拷贝该 Meshlet 的顶点索引 (Meshlet Local -> Global Buffer)
        global_meshlet_vertex_indices.extend_from_slice(&meshlet.vertices);
        
        // 拷贝该 Meshlet 的三角形索引 (Meshlet Local Indices)
        // meshlet.indices is [[u8; 3]; N], we need flat [u8]
        // We only push the valid triangles (up to triangle_count)
        for i in 0..meshlet.triangle_count {
            let tri = meshlet.indices[i as usize];
            global_meshlet_primitive_indices.push(tri[0]);
            global_meshlet_primitive_indices.push(tri[1]);
            global_meshlet_primitive_indices.push(tri[2]);
            // Pad to 4 bytes? VisBuffer shader reads u8x4 packed in u32?
            // "primitive_indices_packed: uint[]" -> 4 indices per uint
            // My shader logic:
            // uint packed_tri_indices = primitive_indices_packed[(cluster.triangle_offset + triID * 3 + vertInTri) / 4];
            // This assumes a continuous stream of indices.
            // BUT: u8 alignment is fine in ByteAddressBuffer if we use u8 load (which GLSL/SPIR-V doesn't really support easily without extension).
            // Shader says: layout(std430) buffer ... uint primitive_indices_packed[];
            // So on the CPU side, we must ensure we write valid bytes that align to uints?
            // Actually, `Vec<u8>` is fine if we cast it to `[u32]` later or just upload as bytes.
            // But for alignment, let's keep it simple: just push u8s.
            // VisBuffer shader logic `(offset + index) / 4` handles the packing interpretation.
        }
        
        // We need to ensure the primitive indices for this cluster are padding-friendly if the next cluster starts?
        // Shader uses `cluster.triangle_offset` which is an index into the u8 stream.
        // `(offset + local_idx) / 4` works fine regardless of alignment as long as the buffer is big enough.
        
        // However, `meshlet.indices` has fixed size 126. We only iterate up to triangle_count.

        // 打包 Cluster 数据
        let cluster = ClusterPacked {
            center_radius: [bounds.center[0], bounds.center[1], bounds.center[2], bounds.radius],
            vertex_offset,
            triangle_offset,
            vertex_count: meshlet.vertices.len() as u8,
            triangle_count: meshlet.triangle_count as u8,
            _pad1: 0,
            error_metric: 0.01, 
        };
        
        global_clusters.push(cluster);
    }

    // 确保整个 primitive 缓冲区是对齐到 4 字节的 (uint)
    while global_meshlet_primitive_indices.len() % 4 != 0 {
        global_meshlet_primitive_indices.push(0);
    }

    println!("Processor: Generated {} clusters from {} triangles.", global_clusters.len(), indices.len() / 3);

    AdaptrixFlatAsset {
        clusters: global_clusters,
        vertices,
        meshlet_vertex_indices: global_meshlet_vertex_indices,
        meshlet_primitive_indices: global_meshlet_primitive_indices,
    }
}