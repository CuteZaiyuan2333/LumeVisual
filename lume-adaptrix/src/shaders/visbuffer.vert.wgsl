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

// 关键修正：使用平铺的 f32 确保 32 字节对齐
struct AdaptrixVertex {
    px: f32, py: f32, pz: f32,
    nx: f32, ny: f32, nz: f32,
    u: f32, v: f32,
}

struct DrawArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

struct View {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    pad0: f32,
    error_threshold: f32,
    viewport_size: vec2<f32>,
    pad1: f32,
}

@group(0) @binding(0) var<storage, read> clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read> vertices: array<AdaptrixVertex>;
@group(0) @binding(2) var<storage, read> meshlet_vertex_indices: array<u32>;
@group(0) @binding(3) var<storage, read> visible_clusters: array<u32>;
@group(0) @binding(4) var<storage, read> primitive_indices: array<u32>;
@group(0) @binding(5) var<storage, read> draw_args: DrawArgs;

@group(1) @binding(0) var<uniform> view: View;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) cluster_id: u32,
    @location(1) @interpolate(flat) triangle_id: u32,
}

@vertex
fn main(@builtin(instance_index) instance_idx: u32, @builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    if (instance_idx >= draw_args.instance_count) {
        return dummy_output();
    }

    let cluster_id = visible_clusters[instance_idx];
    let cluster = clusters[cluster_id];
    
    let triangle_count = (cluster.counts >> 8u) & 0xFFu;
    let triangle_id = vertex_idx / 3u;
    
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
    // 显式构建 vec3，确保不受对齐干扰
    out.position = view.view_proj * vec4<f32>(vertex.px, vertex.py, vertex.pz, 1.0);
    out.cluster_id = cluster_id;
    out.triangle_id = triangle_id;
    return out;
}

fn dummy_output() -> VertexOutput {
    var out: VertexOutput;
    // w 设为 1.0，防止除零，z 设为 2.0 确保在远平面外被剔除
    out.position = vec4<f32>(0.0, 0.0, 2.0, 1.0); 
    return out;
}
