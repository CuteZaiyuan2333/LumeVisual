use std::fmt;

#[derive(Debug)]
pub enum LumeError {
    InstanceCreationFailed(String),
    DeviceCreationFailed(String),
    SurfaceCreationFailed(String),
    ResourceCreationFailed(String),
    PipelineCreationFailed(String),
    ShaderCompilationFailed(String),
    SubmissionFailed(String),
    BackendError(String),
    OutOfMemory,
    Generic(&'static str),
}

impl fmt::Display for LumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LumeError::InstanceCreationFailed(msg) => write!(f, "Instance Creation Failed: {}", msg),
            LumeError::DeviceCreationFailed(msg) => write!(f, "Device Creation Failed: {}", msg),
            LumeError::SurfaceCreationFailed(msg) => write!(f, "Surface Creation Failed: {}", msg),
            LumeError::ResourceCreationFailed(msg) => write!(f, "Resource Creation Failed: {}", msg),
            LumeError::PipelineCreationFailed(msg) => write!(f, "Pipeline Creation Failed: {}", msg),
            LumeError::ShaderCompilationFailed(msg) => write!(f, "Shader Compilation Failed: {}", msg),
            LumeError::SubmissionFailed(msg) => write!(f, "Submission Failed: {}", msg),
            LumeError::BackendError(msg) => write!(f, "Backend Error: {}", msg),
            LumeError::OutOfMemory => write!(f, "Out of Memory"),
            LumeError::Generic(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for LumeError {}

pub type LumeResult<T> = Result<T, LumeError>;
