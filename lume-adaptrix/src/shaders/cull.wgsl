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
@group(0) @binding(1) var<storage, read_write> visible_clusters: array<u32>;
@group(0) @binding(5) var<storage, read_write> draw_args: DrawArgs; 

@group(1) @binding(0) var<uniform> view: View;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_idx = id.x;
    if (cluster_idx >= arrayLength(&all_clusters)) { return; }

    let cluster = all_clusters[cluster_idx];
    let camera_pos = view.camera_pos_and_threshold.xyz;
    let threshold_px = view.camera_pos_and_threshold.w; // 这里的阈值现在代表“像素”

    // 计算到边界球的最近距离
    let dist = max(length(cluster.center_radius.xyz - camera_pos) - cluster.center_radius.w, 0.0001);
    
    // --- 仿 Nanite 投影误差公式 ---
    // ProjectedError = (WorldError * ViewportHeight) / (2.0 * dist * tan(FOV/2))
    // 假设 tan(FOV/2) 为 0.414 (对应 45度)
    let screen_factor = view.viewport_size.y / (2.0 * 0.414);
    let error_px = (cluster.lod_error * screen_factor) / dist;
    let parent_error_px = (cluster.parent_error * screen_factor) / dist;

    // Nanite Cut 判定
    if (error_px <= threshold_px && parent_error_px > threshold_px) {
        let slot = atomicAdd(&draw_args.instance_count, 1u);
        if (slot < arrayLength(&visible_clusters)) {
            visible_clusters[slot] = cluster_idx;
        }
    }
}
