struct Cluster {
    center_radius: vec4<f32>,
    vertex_offset: u32,
    triangle_offset: u32,
    counts: u32,
    lod_error: f32,
    parent_error: f32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
}

struct AdaptrixVertex {
    px: f32, py: f32, pz: f32,
    nx: f32, ny: f32, nz: f32,
    u: f32, v: f32,
}

struct View {
    view_proj_0: vec4<f32>,
    view_proj_1: vec4<f32>,
    view_proj_2: vec4<f32>,
    view_proj_3: vec4<f32>,
    inv_view_proj_0: vec4<f32>,
    inv_view_proj_1: vec4<f32>,
    inv_view_proj_2: vec4<f32>,
    inv_view_proj_3: vec4<f32>,
    camera_pos_and_threshold: vec4<f32>,
    viewport_size: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read> vertices: array<AdaptrixVertex>;
@group(0) @binding(2) var<storage, read> meshlet_vertex_indices: array<u32>;
@group(0) @binding(3) var<storage, read> visible_clusters: array<u32>;
@group(0) @binding(4) var<storage, read> primitive_indices: array<u32>;

@group(1) @binding(0) var<uniform> view: View;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) cluster_id: u32,
    @location(1) @interpolate(flat) triangle_id: u32,
}

@vertex
fn main(@builtin(instance_index) instance_idx: u32, @builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    let view_proj = mat4x4<f32>(view.view_proj_0, view.view_proj_1, view.view_proj_2, view.view_proj_3);

    // 此时硬件已通过 DrawIndirect 过滤了 instance_idx
    let cluster_id = visible_clusters[instance_idx];
    let cluster = clusters[cluster_id];
    
    let triangle_count = (cluster.counts >> 8u) & 0xFFu;
    let triangle_id = vertex_idx / 3u;
    
    // 尽管有 DrawIndirect，但 meshlet 内部的 vertex_idx 仍然可能超出当前 meshlet 的三角形数
    if (triangle_id >= triangle_count) {
        return dummy_output();
    }
    
    let tri_byte_offset = cluster.triangle_offset + vertex_idx;
    let word_idx = tri_byte_offset / 4u;
    let byte_in_word = tri_byte_offset % 4u;
    let packed_word = primitive_indices[word_idx];
    let local_v_idx = (packed_word >> (byte_in_word * 8u)) & 0xFFu;

    let global_v_idx = meshlet_vertex_indices[cluster.vertex_offset + local_v_idx];
    let vertex = vertices[global_v_idx];
    
    var out: VertexOutput;
    out.position = view_proj * vec4<f32>(vertex.px, vertex.py, vertex.pz, 1.0);
    out.cluster_id = cluster_id;
    out.triangle_id = triangle_id;
    return out;
}

fn dummy_output() -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(0.0, 0.0, 2.0, 1.0); 
    return out;
}
