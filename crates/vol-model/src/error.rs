use thiserror::Error;

#[derive(Debug, Error)]
pub enum VolModelError {
    #[error("Failed to load ONNX model: {0}")]
    ModelLoad(String),

    #[error("Failed to load normalization stats: {0}")]
    StatsLoad(String),

    #[error("Inference failed: {0}")]
    Inference(String),

    #[error("Invalid input shape")]
    InvalidInput,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}
