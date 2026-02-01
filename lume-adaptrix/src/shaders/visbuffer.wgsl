struct Cluster {
    vertex_offset: u32,
    triangle_offset: u32,
    vertex_count: u32,
    triangle_count: u32,
    bounding_sphere: vec4<f32>,
    error_metric: f32,
    parent_error: f32,
};

struct AdaptrixVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
};

struct View {
    view_proj: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
    viewport_size: vec2<f32>,
    error_threshold: f32,
};

@group(0) @binding(0) var<storage, read> clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read> vertices: array<AdaptrixVertex>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read> visible_clusters: array<u32>;

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var vis_buffer: texture_storage_2d<rg32uint, write>; 
// Note: r64uint might not be supported everywhere, R32G32 might be safer but harder to atomic

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) cluster_id: u32,
    @location(1) @interpolate(flat) triangle_id: u32,
};

@vertex
fn vs_main(@builtin(instance_index) instance_idx: u32, @builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    let cluster_idx = visible_clusters[instance_idx];
    let cluster = clusters[cluster_idx];
    
    let triangle_id = vertex_idx / 3u;
    let local_v_idx = vertex_idx % 3u;
    
    let index_idx = cluster.triangle_offset + triangle_id * 3u + local_v_idx;
    let v_idx = cluster.vertex_offset + indices[index_idx];
    let vertex = vertices[v_idx];
    
    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(vertex.position, 1.0);
    out.cluster_id = cluster_idx;
    out.triangle_id = triangle_id;
    return out;
}

// In standard hardware rasterization, we'd just output to a texture.
// But Adaptrix/Nanite uses a VisBuffer where we want to store Instance/Cluster/Triangle.
// If we use textureAtomicMax, we need to pack depth and IDs.

@fragment
fn fs_main(in: VertexOutput) {
    let depth = bitcast<u32>(in.position.z);
    let id = (in.cluster_id << 10u) | (in.triangle_id & 0x3FFu);
    textureStore(vis_buffer, vec2<i32>(in.position.xy), vec4<u32>(depth, id, 0u, 0u));
}
