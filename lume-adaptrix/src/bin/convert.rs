use std::path::PathBuf;
use std::fs::File;
use std::io::{Write, BufWriter};
use lume_adaptrix::processor::process_mesh;
use tobj;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: lume-convert <input.obj> <output.lad>");
        return;
    }

    let input_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(&args[2]);

    println!("Loading model: {:?}", input_path);

    let load_options = tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ..Default::default()
    };

    let (models, _materials) = tobj::load_obj(&input_path, &load_options)
        .expect("Failed to load OBJ file");

    if models.is_empty() {
        println!("No models found in file.");
        return;
    }

    // 简单起见，目前只合并处理第一个 mesh
    // 实际项目中应该支持多个 Mesh 或者 Scene Graph
    let mesh = &models[0].mesh;

    println!("Processing mesh: {} ({} triangles)", models[0].name, mesh.indices.len() / 3);
    
    // 转换 tobj 数据格式到 process_mesh 期望的 &[f32]
    // tobj positions 也是 flat Vec<f32>
    let asset = process_mesh(
        &mesh.positions, 
        &mesh.normals, 
        &mesh.texcoords, 
        &mesh.indices
    );

    println!("Saving adaptrix asset to: {:?}", output_path);
    
    let file = File::create(output_path).expect("Failed to create output file");
    let mut writer = BufWriter::new(file);
    
    let encoded = bincode::serialize(&asset).expect("Failed to serialize asset");
    writer.write_all(&encoded).expect("Failed to write to file");

    println!("Done! Size: {:.2} MB", encoded.len() as f64 / 1024.0 / 1024.0);
}
