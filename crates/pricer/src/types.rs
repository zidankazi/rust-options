use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionType {
    Call,
    Put,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExerciseStyle {
    European,
    American,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OptionContract {
    /// Spot price of the underlying
    pub s: f64,
    /// Strike price
    pub k: f64,
    /// Time to expiry in years
    pub t: f64,
    /// Risk-free interest rate
    pub r: f64,
    /// Volatility (annualized)
    pub sigma: f64,
    /// Option type (Call or Put)
    pub option_type: OptionType,
    /// Exercise style (European or American)
    pub exercise_style: ExerciseStyle,
    /// Optional continuous dividend yield
    pub q: Option<f64>,
}

impl OptionContract {
    /// Returns the dividend yield, defaulting to 0.0 if not set.
    pub fn q(&self) -> f64 {
        self.q.unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PricingResult {
    pub price: f64,
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
    pub implied_volatility: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct MonteCarloConfig {
    pub num_paths: usize,
    pub num_steps: usize,
    pub seed: Option<u64>,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            num_paths: 100_000,
            num_steps: 252,
            seed: None,
        }
    }
}
