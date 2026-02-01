struct Cluster {
    vertex_offset: u32,
    triangle_offset: u32,
    vertex_count: u32,
    triangle_count: u32,
    bounding_sphere: vec4<f32>,
    error_metric: f32,
    parent_error: f32,
};

struct MeshInstance {
    world_from_local: mat4x4<f32>,
    cluster_base: u32,
    cluster_count: u32,
};

struct View {
    view_proj: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
    viewport_size: vec2<f32>,
    error_threshold: f32,
};

@group(0) @binding(0) var<storage, read> clusters: array<Cluster>;
@group(0) @binding(1) var<storage, read> instances: array<MeshInstance>;
@group(0) @binding(2) var<storage, read_write> visible_clusters: array<u32>;
@group(0) @binding(3) var<storage, read_write> visible_count: atomic<u32>;

@group(1) @binding(0) var<uniform> view: View;

fn sphere_in_frustum(sphere: vec4<f32>) -> bool {
    for (var i = 0; i < 6; i = i + 1) {
        if (dot(view.frustum[i].xyz, sphere.xyz) + view.frustum[i].w < -sphere.w) {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cluster_idx = global_id.x;
    // For simplicity, assume one instance for now or a global cluster list
    // In a real system, we'd have a nested culling or a flat list of potential clusters
    
    if (cluster_idx >= arrayLength(&clusters)) {
        return;
    }

    let cluster = clusters[cluster_idx];
    
    // Frustum culling
    if (!sphere_in_frustum(cluster.bounding_sphere)) {
        return;
    }

    // LOD selection (simplified)
    // In Nanite/Adaptrix, we select clusters based on screen-space error
    // For this prototype, we'll just accept everything if error_metric == 0
    // and if error_metric > 0, we check if it's small enough.
    
    // TODO: Implement more complex LOD selection
    
    let idx = atomicAdd(&visible_count, 1u);
    visible_clusters[idx] = cluster_idx;
}
