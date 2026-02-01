use anyhow::{Context, Result};
use lume_adaptrix::{AdaptrixMesh, AdaptrixVertex, Cluster};
use meshopt::{build_meshlets, compute_meshlet_bounds, VertexDataAdapter};
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
    let mesh = load_obj(input_path)?;
    let adaptrix_mesh = process_mesh(mesh)?;

    save_adaptrix_mesh(&adaptrix_mesh, output_path)?;
    println!("Saved to {}", output_path);

    Ok(())
}

struct RawMesh {
    vertices: Vec<AdaptrixVertex>,
    indices: Vec<u32>,
}

fn load_obj(path: &str) -> Result<RawMesh> {
    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::GPU_LOAD_OPTIONS,
    ).with_context(|| format!("Failed to load OBJ file: {}", path))?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut index_offset = 0;

    for model in models {
        let mesh = &model.mesh;
        for i in 0..mesh.positions.len() / 3 {
            vertices.push(AdaptrixVertex {
                position: [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ],
                normal: if !mesh.normals.is_empty() {
                    [
                        mesh.normals[i * 3],
                        mesh.normals[i * 3 + 1],
                        mesh.normals[i * 3 + 2],
                    ]
                } else {
                    [0.0, 1.0, 0.0]
                },
                uv: if !mesh.texcoords.is_empty() {
                    [
                        mesh.texcoords[i * 2],
                        mesh.texcoords[i * 2 + 1],
                    ]
                } else {
                    [0.0, 0.0]
                },
            });
        }
        for &index in &mesh.indices {
            indices.push(index + index_offset);
        }
        index_offset = vertices.len() as u32;
    }

    Ok(RawMesh { vertices, indices })
}

fn process_mesh(raw: RawMesh) -> Result<AdaptrixMesh> {
    let vertex_stride = std::mem::size_of::<AdaptrixVertex>();
    let vertex_data = bytemuck::cast_slice(&raw.vertices);
    let adapter = VertexDataAdapter::new(vertex_data, vertex_stride, 0).unwrap();

    let meshlets = build_meshlets(&raw.indices, &adapter, 128, 256, 0.0);

    let mut clusters = Vec::new();
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();

    for m in meshlets.iter() {
        let bounds = compute_meshlet_bounds(m, &adapter);
        
        let cluster = Cluster {
            vertex_offset: all_vertices.len() as u32,
            triangle_offset: all_indices.len() as u32,
            vertex_count: m.vertices.len() as u32,
            triangle_count: (m.triangles.len() / 3) as u32,
            bounding_sphere: [bounds.center[0], bounds.center[1], bounds.center[2], bounds.radius].into(),
            error_metric: 0.0,
            parent_error: 1e10,
            _padding: [0.0; 2],
        };

        clusters.push(cluster);

        for &v_idx in m.vertices {
            all_vertices.push(raw.vertices[v_idx as usize]);
        }
        for &t_idx in m.triangles {
            all_indices.push(t_idx as u32);
        }
    }

    Ok(AdaptrixMesh {
        clusters,
        vertices: all_vertices,
        indices: all_indices,
    })
}

fn save_adaptrix_mesh(mesh: &AdaptrixMesh, path: &str) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path)?;
    
    file.write_all(b"LAD ")?;
    file.write_all(&1u32.to_le_bytes())?;
    file.write_all(&(mesh.clusters.len() as u32).to_le_bytes())?;
    file.write_all(&(mesh.vertices.len() as u32).to_le_bytes())?;
    file.write_all(&(mesh.indices.len() as u32).to_le_bytes())?;

    file.write_all(bytemuck::cast_slice(&mesh.clusters))?;
    file.write_all(bytemuck::cast_slice(&mesh.vertices))?;
    file.write_all(bytemuck::cast_slice(&mesh.indices))?;

    Ok(())
}