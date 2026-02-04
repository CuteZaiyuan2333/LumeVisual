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

struct DrawArgs {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_vertex: u32,
    first_instance: u32,
}

struct DispatchIndirect {
    x: atomic<u32>,
    y: u32,
    z: u32,
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

@group(0) @binding(0) var<storage, read> all_clusters: array<Cluster>;
@group(0) @binding(3) var<storage, read_write> hw_visible_clusters: array<u32>;
@group(0) @binding(5) var<storage, read_write> hw_draw_args: DrawArgs; 
@group(0) @binding(6) var<storage, read_write> sw_visible_clusters: array<u32>;
@group(0) @binding(7) var<storage, read_write> sw_dispatch_args: DispatchIndirect;

@group(1) @binding(0) var<uniform> view: View;

fn project_sphere_rect(center: vec3<f32>, radius: f32, view_proj: mat4x4<f32>) -> vec4<f32> {
    let view_pos = view_proj * vec4<f32>(center, 1.0);
    let d = abs(view_pos.w);
    let screen_radius = (radius / max(d, 0.0001)) * view.viewport_size.y; 
    let box_size = screen_radius * 2.0;
    return vec4<f32>(box_size, box_size, 0.0, 0.0);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_idx = id.x;
    if (cluster_idx >= arrayLength(&all_clusters)) { return; }

    let cluster = all_clusters[cluster_idx];
    let camera_pos = view.camera_pos_and_threshold.xyz;
    let threshold_px = view.camera_pos_and_threshold.w; 

    // --- 1. 视锥剔除 ---
    let center = cluster.center_radius.xyz;
    let radius = cluster.center_radius.w;

    let m = transpose(mat4x4<f32>(
        view.view_proj_0, view.view_proj_1, view.view_proj_2, view.view_proj_3
    ));

    var planes: array<vec4<f32>, 6>;
    planes[0] = m[3] + m[0]; // Left
    planes[1] = m[3] - m[0]; // Right
    planes[2] = m[3] + m[1]; // Bottom
    planes[3] = m[3] - m[1]; // Top
    planes[4] = m[2];        // Near
    planes[5] = m[3] - m[2]; // Far

    for (var i = 0; i < 6; i = i + 1) {
        var plane = planes[i];
        let length_inv = 1.0 / length(plane.xyz);
        plane = plane * length_inv; 
        if (dot(vec4<f32>(center, 1.0), plane) < -radius) {
            return;
        }
    }

    // --- 2. Nanite LOD 选择 ---
    let dist = max(length(center - camera_pos) - radius, 0.0001);
    let screen_factor = view.viewport_size.y / (2.0 * 0.414);
    
    let error_px = (cluster.lod_error * screen_factor) / dist;
    let parent_error_px = (cluster.parent_error * screen_factor) / dist;

    let is_leaf = cluster.parent_error > 9e9; 
    
    if (error_px <= threshold_px && (parent_error_px > threshold_px || is_leaf)) {
        // --- 3. Hybrid Rasterization Split ---
        let rect = project_sphere_rect(center, radius, transpose(m));
        let max_extent = max(rect.x, rect.y);

        // 临时禁用软光栅以验证 HW 路径
        if (false && max_extent < 32.0) {
            let slot = atomicAdd(&sw_dispatch_args.x, 1u);
            if (slot < arrayLength(&sw_visible_clusters)) {
                sw_visible_clusters[slot] = cluster_idx;
            }
        } else {
            let slot = atomicAdd(&hw_draw_args.instance_count, 1u);
            if (slot < arrayLength(&hw_visible_clusters)) {
                hw_visible_clusters[slot] = cluster_idx;
            }
        }
    }
}
