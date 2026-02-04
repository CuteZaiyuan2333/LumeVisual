// cull.wgsl - Hierarchical Occlusion Culling
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

struct DrawArgs { vertex_count: u32, instance_count: atomic<u32>, first_vertex: u32, first_instance: u32 }
struct DispatchIndirect { x: atomic<u32>, y: u32, z: u32 }

struct View {
    view_proj_0: vec4<f32>, view_proj_1: vec4<f32>, view_proj_2: vec4<f32>, view_proj_3: vec4<f32>,
    inv_view_proj_0: vec4<f32>, inv_view_proj_1: vec4<f32>, inv_view_proj_2: vec4<f32>, inv_view_proj_3: vec4<f32>,
    camera_pos_and_threshold: vec4<f32>, viewport_size: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> all_clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read_write> hw_visible_clusters: array<u32>;
@group(0) @binding(5) var<storage, read_write> hw_draw_args: DrawArgs; 
@group(0) @binding(2) var<storage, read_write> sw_visible_clusters: array<u32>;
@group(0) @binding(3) var<storage, read_write> sw_dispatch_args: DispatchIndirect;
@group(0) @binding(6) var<storage, read> cluster_children: array<u32>;

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var hzb: texture_2d<f32>; // 采样 HZB

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_idx = id.x;
    if (cluster_idx >= arrayLength(&all_clusters)) { return; }

    let cluster = all_clusters[cluster_idx];
    let camera_pos = view.camera_pos_and_threshold.xyz;
    let threshold_px = view.camera_pos_and_threshold.w; 

    // 1. 视锥剔除
    let center = cluster.center_radius.xyz;
    let radius = cluster.center_radius.w;
    let m = transpose(mat4x4<f32>(view.view_proj_0, view.view_proj_1, view.view_proj_2, view.view_proj_3));
    var planes: array<vec4<f32>, 6>;
    planes[0] = m[3] + m[0]; planes[1] = m[3] - m[0]; 
    planes[2] = m[3] + m[1]; planes[3] = m[3] - m[1];
    planes[4] = m[2];        planes[5] = m[3] - m[2];

    for (var i = 0; i < 6; i = i + 1) {
        var plane = planes[i];
        if (dot(vec4<f32>(center, 1.0), plane) < -radius * length(plane.xyz)) { return; }
    }

    // 2. Nanite LOD 选择
    let dist = max(length(center - camera_pos) - radius, 0.0001);
    let screen_factor = view.viewport_size.y / (2.0 * 0.414);
    let error_px = (cluster.lod_error * screen_factor) / dist;
    let parent_error_px = (cluster.parent_error * screen_factor) / dist;
    let is_leaf = cluster.parent_error > 9e9; 
    
    if (error_px <= threshold_px && (parent_error_px > threshold_px || is_leaf)) {
        // 3. HZB 遮挡剔除 (简单实现)
        // 投影包围球到屏幕空间
        let clip_pos = m * vec4<f32>(center, 1.0);
        let screen_radius = (radius * screen_factor) / max(clip_pos.w, 0.0001);
        let screen_pos = (clip_pos.xy / clip_pos.w) * 0.5 + 0.5;
        let depth = clip_pos.z / clip_pos.w;

        // 这里我们简化：如果中心点深度明显大于 HZB，则剔除
        // 真正的 Nanite 会测试整个 BBox 区域
        let hzb_size = vec2<f32>(textureDimensions(hzb, 0));
        let hzb_depth = textureLoad(hzb, vec2<i32>(screen_pos * hzb_size), 0).x;
        if (depth > hzb_depth + 0.001) { return; }

        let max_extent = screen_radius * 2.0;
        if (max_extent < 12.0) { // 调小 SW 阈值提升性能
            let slot = atomicAdd(&sw_dispatch_args.x, 1u);
            if (slot < arrayLength(&sw_visible_clusters)) { sw_visible_clusters[slot] = cluster_idx; }
        } else {
            let slot = atomicAdd(&hw_draw_args.instance_count, 1u);
            if (slot < arrayLength(&hw_visible_clusters)) { hw_visible_clusters[slot] = cluster_idx; }
        }
    }
}
