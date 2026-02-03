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



            // --- 1. 视锥剔除 (参考 Gribb-Hartmann 算法 & UE5 逻辑) ---



            let center = cluster.center_radius.xyz;



            let radius = cluster.center_radius.w;



            



            // 构建 mat4 并转置，以便通过索引获取“行”



            let m = transpose(mat4x4<f32>(



                view.view_proj_0,



                view.view_proj_1,



                view.view_proj_2,



                view.view_proj_3



            ));



        



            // 根据投影矩阵的行提取 6 个视锥体平面



            // 平面方程：dot(plane.xyz, pos) + plane.w = 0



            // 如果结果 < 0，则点在平面外



            var planes: array<vec4<f32>, 6>;



            planes[0] = m[3] + m[0]; // Left:   w + x >= 0



            planes[1] = m[3] - m[0]; // Right:  w - x >= 0



            planes[2] = m[3] + m[1]; // Bottom: w + y >= 0



            planes[3] = m[3] - m[1]; // Top:    w - y >= 0



            planes[4] = m[2];        // Near:   z >= 0 (Vulkan 专用)



            planes[5] = m[3] - m[2]; // Far:    w - z >= 0 (Vulkan 专用)



        



            for (var i = 0; i < 6; i = i + 1) {



                var plane = planes[i];



                // 归一化法线部分



                let length_inv = 1.0 / length(plane.xyz);



                plane = plane * length_inv; 



                



                // 球体视锥剔除判定



                if (dot(vec4<f32>(center, 1.0), plane) < -radius) {



                    return; // 集群在平面外，直接丢弃



                }



            }



        

    



    



    // --- 2. Nanite LOD 选择 (参考 UE5 核心逻辑) ---
    // 计算边界球到相机的最短距离
    let dist = max(length(center - camera_pos) - radius, 0.0001);
    
    // 投影公式：error * screen_height / (2.0 * dist * tan(half_fov))
    // 其中 0.414 是 tan(45deg / 2) 的近似值
    let screen_factor = view.viewport_size.y / (2.0 * 0.414);
    
    // 将几何误差投影到屏幕空间（像素）
    let error_px = (cluster.lod_error * screen_factor) / dist;
    let parent_error_px = (cluster.parent_error * screen_factor) / dist;

    // --- 核心判定条件 ---
    // 判定逻辑：当前集群的误差在阈值内（足够精细），且其父集群的误差超出阈值（父级不够精细）
    // 这在 DAG 中形成了一个唯一的“切割面（Cut）”，确保每个像素只被覆盖一次
    // 增加一个小 epsilon (1e-4) 处理浮点边界情况
    let is_leaf = cluster.parent_error > 9e9; // 根节点或未处理节点
    
    if (error_px <= threshold_px && (parent_error_px > threshold_px || is_leaf)) {
        let slot = atomicAdd(&draw_args.instance_count, 1u);
        if (slot < arrayLength(&visible_clusters)) {
            visible_clusters[slot] = cluster_idx;
        }
    }
}
