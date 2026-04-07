// VolModel — loads an ONNX transformer and runs inference on market windows.

use std::path::Path;

use ndarray::Array3;
use ort::session::Session;
use ort::value::Value;

use crate::error::VolModelError;
use crate::types::{MarketWindow, NormStats, SviPrediction};
use crate::{NUM_FEATURES, WINDOW_SIZE};

pub struct VolModel {
    session: Session,
    feat_mean: [f32; NUM_FEATURES],
    feat_std: [f32; NUM_FEATURES],
    label_mean: [f32; 5],
    label_std: [f32; 5],
}

impl VolModel {
    // Load the ONNX model and its normalization stats from disk.
    // Expects: onnx_path points to vol_model.onnx, stats_path to norm_stats.json
    pub fn load(onnx_path: &Path, stats_path: &Path) -> Result<Self, VolModelError> {
        let session = Session::builder()
            .map_err(|e| VolModelError::ModelLoad(e.to_string()))?
            .commit_from_file(onnx_path)
            .map_err(|e| VolModelError::ModelLoad(e.to_string()))?;

        let stats_json = std::fs::read_to_string(stats_path)?;
        let stats: NormStats = serde_json::from_str(&stats_json)?;

        // sanity check — make sure the exported stats match our compiled constants
        if stats.window_size != WINDOW_SIZE || stats.num_features != NUM_FEATURES {
            return Err(VolModelError::StatsLoad(format!(
                "shape mismatch: expected {WINDOW_SIZE}x{NUM_FEATURES}, \
                 got {}x{}",
                stats.window_size, stats.num_features
            )));
        }

        let feat_mean = vec_to_array::<NUM_FEATURES>(&stats.feat_mean)?;
        let feat_std = vec_to_array::<NUM_FEATURES>(&stats.feat_std)?;
        let label_mean = vec_to_array::<5>(&stats.label_mean)?;
        let label_std = vec_to_array::<5>(&stats.label_std)?;

        Ok(Self {
            session,
            feat_mean,
            feat_std,
            label_mean,
            label_std,
        })
    }

    // Run inference on a single market window. Returns the predicted SVI params.
    pub fn predict(&mut self, window: &MarketWindow) -> Result<SviPrediction, VolModelError> {
        // 1. normalize the input features (same math the Python dataset used)
        let mut normalized = [[0.0f32; NUM_FEATURES]; WINDOW_SIZE];
        for day in 0..WINDOW_SIZE {
            for feat in 0..NUM_FEATURES {
                normalized[day][feat] =
                    (window.data[day][feat] - self.feat_mean[feat]) / self.feat_std[feat];
            }
        }

        // 2. build the input tensor: shape [1, 30, 5] with batch dim of 1
        let mut flat = Vec::with_capacity(WINDOW_SIZE * NUM_FEATURES);
        for day in &normalized {
            flat.extend_from_slice(day);
        }
        let input_array = Array3::from_shape_vec((1, WINDOW_SIZE, NUM_FEATURES), flat)
            .map_err(|_| VolModelError::InvalidInput)?;

        let input_tensor = Value::from_array(input_array)
            .map_err(|e| VolModelError::Inference(e.to_string()))?;

        // 3. run the session — the input name must match what we exported ("market_window")
        let outputs = self
            .session
            .run(ort::inputs!["market_window" => input_tensor])
            .map_err(|e| VolModelError::Inference(e.to_string()))?;

        // 4. extract the output tensor: shape [1, 5]
        let output = outputs
            .get("svi_params")
            .ok_or_else(|| VolModelError::Inference("missing svi_params output".into()))?;
        let (_shape, data) = output
            .try_extract_tensor::<f32>()
            .map_err(|e| VolModelError::Inference(e.to_string()))?;

        if data.len() != 5 {
            return Err(VolModelError::Inference(format!(
                "expected 5 outputs, got {}",
                data.len()
            )));
        }

        // 5. denormalize — reverse the (x - mean) / std we did during training
        let mut denorm = [0.0f32; 5];
        for i in 0..5 {
            denorm[i] = data[i] * self.label_std[i] + self.label_mean[i];
        }

        // 6. package as SviPrediction
        Ok(SviPrediction {
            a: denorm[0] as f64,
            b: denorm[1] as f64,
            rho: denorm[2] as f64,
            m: denorm[3] as f64,
            sigma: denorm[4] as f64,
        })
    }
}

fn vec_to_array<const N: usize>(v: &[f32]) -> Result<[f32; N], VolModelError> {
    if v.len() != N {
        return Err(VolModelError::StatsLoad(format!(
            "expected {N} values, got {}",
            v.len()
        )));
    }
    let mut arr = [0.0f32; N];
    arr.copy_from_slice(v);
    Ok(arr)
}
