struct Cluster {
    center_radius: vec4<f32>,
    vertex_offset: u32,
    triangle_offset: u32,
    vertex_count: u32,
    triangle_count: u32, // 这里在 Rust 中是 u8x2+u16，WGSL 按 u32 读取需要小心位域
    lod_error: f32,
    parent_error: f32,
}

struct ViewUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
    viewport_size: vec2<f32>,
    error_threshold: f32, // 通常设为 1.0 (代表 1 像素误差)
}

@group(0) @binding(0) var<storage, read> all_clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read_write> visible_clusters: array<u32>;
@group(0) @binding(2) var<storage, read_write> draw_args: vec4<u32>; // [count, instanceCount, first, base]

@group(1) @binding(0) var<uniform> view: ViewUniform;

fn get_projected_error(center: vec3<f32>, radius: f32, error: f32) -> f32 {
    // 简化的投影误差计算
    // 实际 Nanite 会计算边界球到摄像机的最近距离
    let dist = max(length(center - view.inv_view_proj[3].xyz) - radius, 0.0001);
    
    // 核心公式：误差投影到屏幕上的像素大小
    // 假设 FOV 是 45度，tan(FOV/2) 约等于 0.414
    let projected_error = (error * view.viewport_size.y) / (2.0 * dist * 0.414);
    return projected_error;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_idx = id.x;
    if (cluster_idx >= arrayLength(&all_clusters)) { return; }

    let cluster = all_clusters[cluster_idx];
    
    // 1. 视锥剔除 (Frustum Culling)
    // TODO: 使用 cluster.center_radius 进行判定，这里暂略
    
    // 2. Nanite LOD 选择核心逻辑
    let current_err = get_projected_error(cluster.center_radius.xyz, cluster.center_radius.w, cluster.lod_error);
    let parent_err = get_projected_error(cluster.center_radius.xyz, cluster.center_radius.w, cluster.parent_error);

    // 关键判定条件：
    // 当前误差足够小（看不出来） 且 父节点误差太大（必须细分）
    // 或者它是根节点 (parent_error 很大)
    var is_visible = false;
    if (current_err <= view.error_threshold && parent_err > view.error_threshold) {
        is_visible = true;
    }

    if (is_visible) {
        let slot = atomicAdd(&draw_args[1], 1u); // 增加 instanceCount
        visible_clusters[slot] = cluster_idx;
    }
}