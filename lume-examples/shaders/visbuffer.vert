#version 450

struct Vertex {
    float px, py, pz;
    float nx, ny, nz;
    float u, v;
};

struct Cluster {
    vec4 center_radius;
    uint vertex_offset;
    uint triangle_offset;
    uint counts;
    float lod_error;
    float parent_error;
    uint _pad0;
    uint _pad1;
    uint _pad2;
};

layout(std430, set = 0, binding = 0) readonly buffer ClusterBuffer { Cluster clusters[]; };
layout(std430, set = 0, binding = 1) readonly buffer VertexBuffer { Vertex vertices[]; };
layout(std430, set = 0, binding = 2) readonly buffer MeshletVertexIndices { uint vertex_indices[]; };
layout(std430, set = 0, binding = 3) readonly buffer MeshletPrimitiveIndices { uint primitive_indices_packed[]; };
layout(std430, set = 0, binding = 5) readonly buffer VisibleClusters { uint visible_indices[]; };

layout(set = 0, binding = 4) uniform Uniforms {
    mat4 mvp;
} ubo;

layout(location = 0) out flat uint outClusterID;
layout(location = 1) out flat uint outPrimitiveID;

void main() {
    // 现在 gl_InstanceIndex 代表可见集群列表中的索引
    uint clusterID = visible_indices[gl_InstanceIndex];
    
    uint triID = gl_VertexIndex / 3;
    uint vertInTri = gl_VertexIndex % 3;

    Cluster cluster = clusters[clusterID];
    // 保护：虽然剔除逻辑应该处理好，但此处作为兜底
    if (triID >= ((cluster.counts >> 8) & 0xFF)) {
        gl_Position = vec4(0.0, 0.0, 0.0, 0.0);
        return;
    }

    uint idx_ptr = cluster.triangle_offset + triID * 3 + vertInTri;
    uint packed_tri_indices = primitive_indices_packed[idx_ptr / 4];
    uint shift = (idx_ptr % 4) * 8;
    uint local_vert_idx = (packed_tri_indices >> shift) & 0xFF;

    uint global_vert_idx = vertex_indices[cluster.vertex_offset + local_vert_idx];
    Vertex v = vertices[global_vert_idx];

    gl_Position = ubo.mvp * vec4(v.px, v.py, v.pz, 1.0);
    outClusterID = clusterID;
    outPrimitiveID = triID;
}
