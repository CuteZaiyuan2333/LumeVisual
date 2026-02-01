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

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var vis_buffer: texture_2d<u32>; 

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// 简单的哈希函数，用于给集群涂色
fn hash(n: u32) -> f32 {
    let m = n * 1103515245u + 12345u;
    return f32(m & 0x7FFFFFFFu) / f32(0x7FFFFFFFu);
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let vis_data = textureLoad(vis_buffer, pixel, 0);
    
    let id = vis_data.y;
    if (id == 0u) {
        return vec4<f32>(0.05, 0.05, 0.07, 1.0);
    }
    
    let cluster_id = id >> 10u;
    let triangle_id = id & 0x3FFu;
    
    // --- 调试模式：显示集群颜色 ---
    let r = hash(cluster_id);
    let g = hash(cluster_id + 1u);
    let b = hash(cluster_id + 2u);
    
    // 加入简单的法线光照来增加立体感
    let cluster = clusters[cluster_id];
    let i0 = indices[cluster.triangle_offset + triangle_id * 3u + 0u];
    let i1 = indices[cluster.triangle_offset + triangle_id * 3u + 1u];
    let i2 = indices[cluster.triangle_offset + triangle_id * 3u + 2u];
    let v0 = vertices[cluster.vertex_offset + i0];
    let v1 = vertices[cluster.vertex_offset + i1];
    let v2 = vertices[cluster.vertex_offset + i2];
    
    let p0 = vec3(v0.px, v0.py, v0.pz);
    let p1 = vec3(v1.px, v1.py, v1.pz);
    let p2 = vec3(v2.px, v2.py, v2.pz);
    let normal = normalize(cross(p1 - p0, p2 - p0));
    
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 2.0));
    let diff = max(dot(normal, light_dir), 0.3);
    
    return vec4<f32>(vec3(r, g, b) * diff, 1.0);
}