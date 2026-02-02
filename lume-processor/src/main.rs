use anyhow::{Context, Result};
use lume_adaptrix::{AdaptrixFlatAsset, processor::process_mesh, AdaptrixAsset};
use std::env;

fn main() -> Result<()> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: lume-processor <input.obj> <output.lad>");
        return Ok(());
    }

    let input_path = &args[1];
    let output_path = &args[2];

    println!("Processing {}...", input_path);
    let start_total = std::time::Instant::now();
    
    let (positions, normals, uvs, indices) = load_obj_flat(input_path)?;
    println!("Model loaded and welded in {:.2}s", start_total.elapsed().as_secs_f32());
    
    let build_start = std::time::Instant::now();
    let adaptrix_asset = process_mesh(&positions, &normals, &uvs, &indices);
    println!("Nanite Build complete in {:.2}s", build_start.elapsed().as_secs_f32());

    let save_start = std::time::Instant::now();
    save_adaptrix_asset(&adaptrix_asset, output_path)?;
    println!("Saved to {} in {:.2}s", output_path, save_start.elapsed().as_secs_f32());
    println!("Total execution time: {:.2}s", start_total.elapsed().as_secs_f32());

    Ok(())
}

fn load_obj_flat(path: &str) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>, Vec<u32>)> {
    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::GPU_LOAD_OPTIONS,
    ).with_context(|| format!("Failed to load OBJ file: {}", path))?;

    let mut raw_positions = Vec::new();
    let mut raw_normals = Vec::new();
    let mut raw_uvs = Vec::new();
    let mut raw_indices = Vec::new();
    let mut index_offset = 0;

    for model in models {
        let mesh = &model.mesh;
        raw_positions.extend_from_slice(&mesh.positions);
        raw_normals.extend_from_slice(&mesh.normals);
        raw_uvs.extend_from_slice(&mesh.texcoords);
        
        for &index in &mesh.indices {
            raw_indices.push(index + index_offset);
        }
        index_offset += (mesh.positions.len() / 3) as u32;
    }

    // 使用 meshopt 进行顶点焊接
    use meshopt::{generate_vertex_remap, remap_vertex_buffer, remap_index_buffer};
    
    #[repr(C)]
    #[derive(Clone, Copy, PartialEq, Default)]
    struct FullVertex {
        p: [f32; 3],
        n: [f32; 3],
        u: [f32; 2],
    }

    let mut vertices = Vec::with_capacity(raw_positions.len() / 3);
    for i in 0..raw_positions.len() / 3 {
        vertices.push(FullVertex {
            p: [raw_positions[i*3], raw_positions[i*3+1], raw_positions[i*3+2]],
            n: [0.0, 0.0, 0.0],
            u: [0.0, 0.0],
        });
    }

    let (vertex_count, remap) = generate_vertex_remap(&vertices, Some(&raw_indices));
    let final_vertices = remap_vertex_buffer(&vertices, vertex_count, &remap);
    let final_indices = remap_index_buffer(Some(&raw_indices), vertex_count, &remap);

    // 中心化和缩放逻辑
    let mut min_p = [f32::MAX; 3];
    let mut max_p = [f32::MIN; 3];
    for v in &final_vertices {
        for i in 0..3 {
            min_p[i] = min_p[i].min(v.p[i]);
            max_p[i] = max_p[i].max(v.p[i]);
        }
    }
    let center = [(min_p[0] + max_p[0]) / 2.0, (min_p[1] + max_p[1]) / 2.0, (min_p[2] + max_p[2]) / 2.0];
    let size = [max_p[0] - min_p[0], max_p[1] - min_p[1], max_p[2] - min_p[2]];
    let max_dim = size[0].max(size[1]).max(size[2]);
    let scale = if max_dim > 0.0 { 2.0 / max_dim } else { 1.0 }; 

    let mut positions = Vec::with_capacity(vertex_count * 3);
    let mut normals = Vec::with_capacity(vertex_count * 3);
    let mut uvs = Vec::with_capacity(vertex_count * 2);

    for v in final_vertices {
        positions.push((v.p[0] - center[0]) * scale);
        positions.push((v.p[1] - center[1]) * scale);
        positions.push((v.p[2] - center[2]) * scale);
        normals.extend_from_slice(&v.n);
        uvs.extend_from_slice(&v.u);
    }

    Ok((positions, normals, uvs, final_indices))
}

fn save_adaptrix_asset(asset: &AdaptrixFlatAsset, path: &str) -> Result<()> {
    AdaptrixAsset::save_to_file(asset, path)
}
