// resolve.frag.wgsl - 32-bit Dual Buffer Fallback
struct Cluster {
    center_radius: vec4<f32>,
    vertex_offset: u32,
    triangle_offset: u32,
    counts: u32,
    lod_error: f32,
    parent_error: f32,
    child_count: u32,
    child_base: u32,
    pad0: u32,
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
@group(0) @binding(3) var<storage, read> primitive_indices: array<u32>; 

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var vis_buffer: texture_2d<u32>; 
@group(1) @binding(2) var<storage, read> sw_id_buffer: array<atomic<u32>>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

fn hash_to_light_color(id: u32) -> vec3<f32> {
    let r = f32((id * 1103515245u + 12345u) & 0x7FFFFFFFu) / f32(0x7FFFFFFFu);
    let g = f32((id * 2147483647u + 45678u) & 0x7FFFFFFFu) / f32(0x7FFFFFFFu);
    let b = f32((id * 134775813u + 91011u) & 0x7FFFFFFFu) / f32(0x7FFFFFFFu);
    return mix(vec3<f32>(r, g, b), vec3<f32>(1.0), 0.5);
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let hw_vis = textureLoad(vis_buffer, pixel, 0);
    
    var cluster_id = 0u;
    var triangle_id = 0u;
    var found = false;

    if (hw_vis.y != 0u) {
        cluster_id = (hw_vis.y >> 10u) - 1u;
        triangle_id = hw_vis.y & 0x3FFu;
        found = true;
    } else {
        let w = u32(view.viewport_size.x);
        let idx = u32(pixel.y) * w + u32(pixel.x);
        let vis_id_plus_1 = atomicLoad(&sw_id_buffer[idx]);
        if (vis_id_plus_1 != 0u) {
            let vis_id = vis_id_plus_1 - 1u;
            cluster_id = vis_id >> 10u;
            triangle_id = vis_id & 0x3FFu;
            found = true;
        }
    }
    
    if (!found) {
        return vec4<f32>(0.02, 0.02, 0.03, 1.0); 
    }
    
    let cluster = clusters[cluster_id];
    var lv: array<u32, 3>;
    let tri_off = cluster.triangle_offset + triangle_id * 3u;
    for (var i = 0u; i < 3u; i = i + 1u) {
        let off = tri_off + i;
        lv[i] = (primitive_indices[off / 4u] >> ((off % 4u) * 8u)) & 0xFFu;
    }

    let i0 = meshlet_vertex_indices[cluster.vertex_offset + lv[0]];
    let i1 = meshlet_vertex_indices[cluster.vertex_offset + lv[1]];
    let i2 = meshlet_vertex_indices[cluster.vertex_offset + lv[2]];
    
    let p0 = vec3(vertices[i0].px, vertices[i0].py, vertices[i0].pz);
    let p1 = vec3(vertices[i1].px, vertices[i1].py, vertices[i1].pz);
    let p2 = vec3(vertices[i2].px, vertices[i2].py, vertices[i2].pz);
    
    let normal = normalize(cross(p1 - p0, p2 - p0));
    let diff = max(dot(normal, normalize(vec3(0.5, 1.0, 0.5))), 0.0) * 0.6 + 0.4;
    return vec4<f32>(hash_to_light_color(cluster_id) * diff, 1.0);
}
