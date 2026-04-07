use serde::{Deserialize, Serialize};

use crate::{NUM_FEATURES, WINDOW_SIZE};

// 30 days of 5 features each — matches the Python training format.
// Feature order: [log_return, realized_vol, spot_scaled, t, days_to_exp_scaled]
#[derive(Debug, Clone)]
pub struct MarketWindow {
    pub data: [[f32; NUM_FEATURES]; WINDOW_SIZE],
}

impl MarketWindow {
    pub fn new(data: [[f32; NUM_FEATURES]; WINDOW_SIZE]) -> Self {
        Self { data }
    }

    // flatten into a row-major [WINDOW_SIZE * NUM_FEATURES] vector — what ndarray expects
    pub fn as_flat(&self) -> Vec<f32> {
        let mut flat = Vec::with_capacity(WINDOW_SIZE * NUM_FEATURES);
        for day in &self.data {
            flat.extend_from_slice(day);
        }
        flat
    }
}

// Same 5 knobs as pricer::svi::SviParams, but in the vol-model crate
// so callers don't need a dependency on pricer just to hold predictions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SviPrediction {
    pub a: f64,
    pub b: f64,
    pub rho: f64,
    pub m: f64,
    pub sigma: f64,
}

impl SviPrediction {
    // convert to pricer::svi::SviParams for use with the pricer crate's svi_variance / svi_iv
    pub fn to_pricer_params(&self) -> pricer::svi::SviParams {
        pricer::svi::SviParams {
            a: self.a,
            b: self.b,
            rho: self.rho,
            m: self.m,
            sigma: self.sigma,
        }
    }
}

// Normalization stats loaded from norm_stats.json — must match what Python used.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NormStats {
    pub feat_mean: Vec<f32>,
    pub feat_std: Vec<f32>,
    pub label_mean: Vec<f32>,
    pub label_std: Vec<f32>,
    pub window_size: usize,
    pub num_features: usize,
}
