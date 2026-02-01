struct Cluster {
    vertex_offset: u32,
    triangle_offset: u32,
    vertex_count: u32,
    triangle_count: u32,
    bounding_sphere: vec4<f32>,
    error_metric: f32,
    parent_error: f32,
    pad0: f32,
    pad1: f32,
};

struct AdaptrixVertex {
    px: f32, py: f32, pz: f32,
    nx: f32, ny: f32, nz: f32,
    u: f32, v: f32,
};

struct View {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
    viewport_size: vec2<f32>,
    error_threshold: f32,
};

@group(0) @binding(0) var<storage, read> clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read> vertices: array<AdaptrixVertex>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read> visible_clusters: array<u32>;

@group(1) @binding(0) var<uniform> view: View;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) cluster_id: u32,
    @location(1) @interpolate(flat) triangle_id: u32,
};

@vertex
fn main(@builtin(instance_index) instance_idx: u32, @builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    // 从可见集群列表中获取 ID
    let cluster_id = visible_clusters[instance_idx];
    
    if (cluster_id >= arrayLength(&clusters)) {
        return dummy_output();
    }

    let cluster = clusters[cluster_id];
    let triangle_id = vertex_idx / 3u;
    let local_v_idx = vertex_idx % 3u;
    
    // 如果三角形序号超出该集群实际大小，彻底抛弃
    if (triangle_id >= cluster.triangle_count) {
        return dummy_output();
    }
    
    let index_idx = cluster.triangle_offset + triangle_id * 3u + local_v_idx;
    let v_idx = cluster.vertex_offset + indices[index_idx];
    let vertex = vertices[v_idx];
    
    var out: VertexOutput;
    // 直接投影，不再手动翻转 Y
    out.position = view.view_proj * vec4<f32>(vertex.px, vertex.py, vertex.pz, 1.0);
    out.cluster_id = cluster_id;
    out.triangle_id = triangle_id;
    return out;
}

fn dummy_output() -> VertexOutput {
    var out: VertexOutput;
    // 强制设为 NaN 或 远平面外
    out.position = vec4<f32>(0.0, 0.0, 2.0, 0.0); 
    return out;
}