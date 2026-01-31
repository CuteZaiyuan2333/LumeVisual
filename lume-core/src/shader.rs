use naga::front::glsl;
use naga::back::spv;

pub enum ShaderSource<'a> {
    Glsl {
        source: &'a str,
        stage: naga::ShaderStage,
        defines: naga::FastHashMap<String, String>,
    },
    Wgsl(&'a str),
}

pub fn compile_shader(source: ShaderSource) -> Result<Vec<u32>, String> {
    let module = match source {
        ShaderSource::Wgsl(src) => {
            naga::front::wgsl::Frontend::new().parse(src)
                .map_err(|e| format!("WGSL parse error: {:?}", e))?
        }
        ShaderSource::Glsl { source, stage, defines } => {
            let mut parser = glsl::Frontend::default();
            let options = glsl::Options {
                stage,
                defines,
            };
            parser.parse(&options, source)
                .map_err(|e| format!("GLSL parse error: {:?}", e))?
        }
    };

    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| format!("Naga validation error: {:?}", e))?;

    let write_options = spv::Options::default();
    let spv = spv::write_vec(&module, &info, &write_options, None)
        .map_err(|e| format!("SPIR-V write error: {:?}", e))?;

    Ok(spv)
}
