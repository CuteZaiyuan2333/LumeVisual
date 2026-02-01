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

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Generate a fullscreen triangle
    var out: VertexOutput;
    let x = f32(i32(vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(vertex_index) % 2) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

fn get_barycentrics(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denom = d00 * d11 - d01 * d01;
    if (abs(denom) < 1e-10) { return vec3<f32>(1.0, 0.0, 0.0); }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    return vec3<f32>(u, v, w);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let vis_data = textureLoad(vis_buffer, pixel, 0);
    
    // vis_data.x is depth, vis_data.y is ID
    let id = vis_data.y;
    if (id == 0u) {
        return vec4<f32>(0.05, 0.05, 0.07, 1.0); // Dark background
    }
    
    let cluster_id = id >> 10u;
    let triangle_id = id & 0x3FFu;
    
    let cluster = clusters[cluster_id];
    let i0 = indices[cluster.triangle_offset + triangle_id * 3u + 0u];
    let i1 = indices[cluster.triangle_offset + triangle_id * 3u + 1u];
    let i2 = indices[cluster.triangle_offset + triangle_id * 3u + 2u];
    
    let v0 = vertices[cluster.vertex_offset + i0];
    let v1 = vertices[cluster.vertex_offset + i1];
    let v2 = vertices[cluster.vertex_offset + i2];
    
    let depth = bitcast<f32>(vis_data.x);
    let ndc = vec4<f32>(
        (in.position.x / view.viewport_size.x) * 2.0 - 1.0,
        (1.0 - in.position.y / view.viewport_size.y) * 2.0 - 1.0,
        depth,
        1.0
    );
    let world_pos_h = view.inv_view_proj * ndc;
    let world_pos = world_pos_h.xyz / world_pos_h.w;
    
    let bary = get_barycentrics(world_pos, v0.position, v1.position, v2.position);
    
    let normal = normalize(v0.normal * bary.x + v1.normal * bary.y + v2.normal * bary.z);
    
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let diff = max(dot(normal, light_dir), 0.0);
    
    return vec4<f32>(vec3<f32>(diff), 1.0);
}