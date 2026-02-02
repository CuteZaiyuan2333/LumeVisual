use anyhow::{Context, Result};
use lume_adaptrix::{AdaptrixFlatAsset, processor::process_mesh};
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
    let (positions, normals, uvs, indices) = load_obj_flat(input_path)?;
    let adaptrix_asset = process_mesh(&positions, &normals, &uvs, &indices);

    save_adaptrix_asset(&adaptrix_asset, output_path)?;
    println!("Saved to {}", output_path);

    Ok(())
}

fn load_obj_flat(path: &str) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>, Vec<u32>)> {
    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::GPU_LOAD_OPTIONS,
    ).with_context(|| format!("Failed to load OBJ file: {}", path))?;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut index_offset = 0;

    for model in models {
        let mesh = &model.mesh;
        positions.extend_from_slice(&mesh.positions);
        normals.extend_from_slice(&mesh.normals);
        uvs.extend_from_slice(&mesh.texcoords);
        
        for &index in &mesh.indices {
            indices.push(index + index_offset);
        }
        index_offset += (mesh.positions.len() / 3) as u32;
    }

    Ok((positions, normals, uvs, indices))
}

fn save_adaptrix_asset(asset: &AdaptrixFlatAsset, path: &str) -> Result<()> {
    use std::fs::File;
    use std::io::BufWriter;

    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, asset).context("Failed to serialize asset")?;

    Ok(())
}