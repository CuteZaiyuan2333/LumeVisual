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

    // 核心修复：cluster_id + 1u，避免产生 0 号 ID

    let id = ((in.cluster_id + 1u) << 10u) | (in.triangle_id & 0x3FFu);

    

    var out: FragmentOutput;

    out.vis_data = vec2<u32>(depth, id);

    return out;

}
