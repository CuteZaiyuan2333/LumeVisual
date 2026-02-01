struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) cluster_id: u32,
    @location(1) @interpolate(flat) triangle_id: u32,
};

struct FragmentOutput {
    @location(0) vis_data: vec2<u32>,
};

@fragment
fn main(in: VertexOutput) -> FragmentOutput {
    let depth = bitcast<u32>(in.position.z);
    let id = (in.cluster_id << 10u) | (in.triangle_id & 0x3FFu);
    
    var out: FragmentOutput;
    out.vis_data = vec2<u32>(depth, id);
    return out;
}