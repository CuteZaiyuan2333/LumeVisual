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
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    error_threshold: f32,
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> all_clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read_write> visible_clusters: array<u32>;
@group(0) @binding(2) var<storage, read_write> draw_args: DrawArgs; 

@group(1) @binding(0) var<uniform> view: View;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_idx = id.x;
    if (cluster_idx >= arrayLength(&all_clusters)) { return; }

    let cluster = all_clusters[cluster_idx];
    
    // 投影误差计算
    let dist = max(length(cluster.center_radius.xyz - view.camera_pos) - cluster.center_radius.w, 0.0001);
    let current_err = (cluster.lod_error * view.viewport_size.y) / (2.0 * dist * 0.414);
    let parent_err = (cluster.parent_error * view.viewport_size.y) / (2.0 * dist * 0.414);

    // Nanite Cut 判定
    if (current_err <= view.error_threshold && parent_err > view.error_threshold) {
        let slot = atomicAdd(&draw_args.instance_count, 1u);
        if (slot < arrayLength(&visible_clusters)) {
            visible_clusters[slot] = cluster_idx;
        }
    }
}