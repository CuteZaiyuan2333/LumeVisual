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
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
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
@group(0) @binding(2) var<storage, read> indices: array<u32>;

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var vis_buffer_hw: texture_2d<u32>;
@group(1) @binding(2) var depth_buffer_hw: texture_depth_2d;
@group(1) @binding(3) var vis_buffer_sw: texture_2d<u32>; // Read from storage texture as sampled/texture_2d if usage allows, else storage

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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
    
    // 1. Read HW
    let id_hw = textureLoad(vis_buffer_hw, pixel, 0).r;
    let z_hw = textureLoad(depth_buffer_hw, pixel, 0); // 0.0 = near (in standard), or 0=far in reversed? Assuming standard 0..1

    // 2. Read SW
    let packed_sw = textureLoad(vis_buffer_sw, pixel, 0).r;
    let depth_sw_u = packed_sw >> 12u;
    let id_sw = packed_sw & 0xFFFu;
    
    var z_sw = 1.0;
    if (packed_sw != 0u) {
        z_sw = 1.0 - f32(depth_sw_u) / 1048575.0;
    }

    // 3. Compare (Standard Depth: Less is Closer)
    var final_id = 0u;
    var final_z = 1.0;
    
    if (z_hw < z_sw) {
        final_id = id_hw;
        final_z = z_hw;
    } else {
        final_id = id_sw;
        final_z = z_sw;
    }

    if (final_id == 0u) {
         return vec4<f32>(0.05, 0.05, 0.07, 1.0);
    }
    
    // Visualize ID as color
    let r = f32((final_id * 17u) % 255u) / 255.0;
    let g = f32((final_id * 43u) % 255u) / 255.0;
    let b = f32((final_id * 97u) % 255u) / 255.0;
    
    return vec4<f32>(r, g, b, 1.0);
}