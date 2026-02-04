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
@group(0) @binding(3) var<storage, read> primitive_indices: array<u32>; 

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var vis_buffer: texture_2d<u32>; 
@group(1) @binding(2) var<storage, read> sw_id_buffer: array<atomic<u32>>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

fn hash(n: u32) -> f32 {
    let m = n * 1103515245u + 12345u;
    return f32(m & 0x7FFFFFFFu) / f32(0x7FFFFFFFu);
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let vis_data = textureLoad(vis_buffer, pixel, 0);
    
    var id = vis_data.y;
    var is_sw = false;
    if (id == 0u) {
        // SW补洞层：仅当HW没有写入id时才回退
        let w = u32(view.viewport_size.x);
        let idx = u32(pixel.y) * w + u32(pixel.x);
        let combined = atomicLoad(&sw_id_buffer[idx]);
        let id_u16 = combined & 0xFFFFu;
        if (id_u16 != 0u) {
            // Reconstruct ID from the 16-bit packed format
            // payload_id = ((cluster_id + 1u) << 6u) | (t & 0x3Fu)
            let cluster_id_plus_1 = id_u16 >> 6u;
            let triangle_id = id_u16 & 0x3Fu;
            id = (cluster_id_plus_1 << 10u) | triangle_id;
            is_sw = true;
        }
    }
    if (id == 0u) {
        return vec4<f32>(0.02, 0.02, 0.03, 1.0); // 背景色
    }
    
    let cluster_id = (id >> 10u) - 1u;
    let triangle_id = id & 0x3FFu;
    let cluster = clusters[cluster_id];

    var local_v_indices: array<u32, 3>;
    for (var i = 0u; i < 3u; i = i + 1u) {
        let tri_byte_offset = cluster.triangle_offset + triangle_id * 3u + i;
        let word_idx = tri_byte_offset / 4u;
        let byte_in_word = tri_byte_offset % 4u;
        let packed_word = primitive_indices[word_idx];
        local_v_indices[i] = (packed_word >> (byte_in_word * 8u)) & 0xFFu;
    }

    let i0 = meshlet_vertex_indices[cluster.vertex_offset + local_v_indices[0]];
    let i1 = meshlet_vertex_indices[cluster.vertex_offset + local_v_indices[1]];
    let i2 = meshlet_vertex_indices[cluster.vertex_offset + local_v_indices[2]];
    
    let v0 = vertices[i0];
    let v1 = vertices[i1];
    let v2 = vertices[i2];
    
    let p0 = vec3(v0.px, v0.py, v0.pz);
    let p1 = vec3(v1.px, v1.py, v1.pz);
    let p2 = vec3(v2.px, v2.py, v2.pz);
    
    // --- 仿 Nanite 鲁棒法线重建 ---
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    var raw_normal = cross(edge1, edge2);
    let len_sq = dot(raw_normal, raw_normal);
    
    var normal: vec3<f32>;
    if (len_sq < 1e-12) {
        normal = vec3<f32>(0.0, 1.0, 0.0); // 极小三角形保底
    } else {
        normal = raw_normal * inverseSqrt(len_sq);
    }
    
    let r = hash(cluster_id);
    let g = hash(cluster_id + 1u);
    let b = hash(cluster_id + 2u);
    
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 2.0));
    let diff = max(abs(dot(normal, light_dir)), 0.2); // 双面光照，防止翻转导致的黑色
    
    return vec4<f32>(vec3(r, g, b) * diff, 1.0);
}
