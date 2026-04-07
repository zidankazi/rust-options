// Vol model crate — loads a trained transformer from ONNX and predicts
// SVI vol surface parameters from a 30-day market window.
// The model was trained in PyTorch (see data/vol_model/) and exported to ONNX.

pub mod error;
pub mod model;
pub mod types;

pub use error::VolModelError;
pub use model::VolModel;
pub use types::{MarketWindow, SviPrediction};

pub const WINDOW_SIZE: usize = 30;
pub const NUM_FEATURES: usize = 5;
