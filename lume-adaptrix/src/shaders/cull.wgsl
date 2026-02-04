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
@group(0) @binding(1) var<storage, read_write> hw_visible_clusters: array<u32>;
@group(0) @binding(5) var<storage, read_write> hw_draw_args: DrawArgs; 
@group(0) @binding(2) var<storage, read_write> sw_visible_clusters: array<u32>;
@group(0) @binding(3) var<storage, read_write> sw_dispatch_args: DispatchIndirect;

@group(1) @binding(0) var<uniform> view: View;

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
        // --- 3. Hybrid Split (HW main + SW补洞层) ---
        // SW仅用于屏幕投影极小的cluster（避免过多像素开销），并在Resolve中作为HW id==0 的回退。
        // 注意：SW渲染pass必须在帧图中真实执行，否则会出现镂空。
        let view_pos = mat4x4<f32>(view.view_proj_0, view.view_proj_1, view.view_proj_2, view.view_proj_3) * vec4<f32>(center, 1.0);
        let d = abs(view_pos.w);
        let screen_radius = (radius / max(d, 0.0001)) * view.viewport_size.y;
        let max_extent = screen_radius * 2.0;

        if (max_extent < 20.0) {
            let slot = atomicAdd(&sw_dispatch_args.x, 1u);
            if (slot < arrayLength(&sw_visible_clusters)) {
                // We don't store triangles here, just cluster index
                // but the soft_raster needs to know which cluster to process.
                sw_visible_clusters[slot] = cluster_idx;
            }
        } else {
            let slot = atomicAdd(&hw_draw_args.instance_count, 1u);
            if (slot < arrayLength(&hw_visible_clusters)) {
                // HW rendering uses the full 32-bit ID since it has RG32Uint target
                hw_visible_clusters[slot] = cluster_idx;
            }
        }
    }
}
