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

    let threshold_px = view.camera_pos_and_threshold.w; 



        let center = cluster.center_radius.xyz;
    let radius = cluster.center_radius.w;

        // --- 1. 视锥剔除 (针对 Vulkan Z[0, 1] 优化) ---

        let m0 = view.view_proj_0;

        let m1 = view.view_proj_1;

        let m2 = view.view_proj_2;

        let m3 = view.view_proj_3;

        

        // 提取 6 个视锥体平面

        var planes = array<vec4<f32>, 6>(

            m3 + m0, // Left

            m3 - m0, // Right

            m3 + m1, // Bottom

            m3 - m1, // Top

            m2,      // Near (Vulkan 深度从 0 开始，所以直接用 m2)

            m3 - m2  // Far

        );

    

        for (var i = 0; i < 6; i = i + 1) {

            var plane = planes[i];

            let length_inv = 1.0 / length(plane.xyz);

            plane = plane * length_inv; // 归一化

            if (dot(vec4<f32>(center, 1.0), plane) < -radius) {

                return; // 集群完全在视锥体外

            }

        }

    



    



    // --- 2. Nanite LOD 选择 ---

    // 计算到边界球的最短距离

    let dist = max(length(center - camera_pos) - radius, 0.0001);

    

    // 投影误差公式 (简化版)

    let screen_factor = view.viewport_size.y / (2.0 * 0.414);

    let error_px = (cluster.lod_error * screen_factor) / dist;

    let parent_error_px = (cluster.parent_error * screen_factor) / dist;



    // 只有当当前节点够细，且父节点不够细时，才渲染它

    if (error_px <= threshold_px && parent_error_px > (threshold_px + 0.0001)) {

        let slot = atomicAdd(&draw_args.instance_count, 1u);

        if (slot < arrayLength(&visible_clusters)) {

            visible_clusters[slot] = cluster_idx;

        }

    }

}
