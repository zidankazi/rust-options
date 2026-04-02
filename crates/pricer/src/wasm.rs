// WASM bindings — thin wrappers that expose the pricer to JavaScript.
// wasm-bindgen can't pass Rust structs across the boundary, so each function
// takes flat args (f64, &str) and returns a JS object via serde.

use wasm_bindgen::prelude::*;
use crate::types::{ExerciseStyle, OptionContract, OptionType};

// Converts "call"/"put" string from JS into our OptionType enum.
// wasm-bindgen doesn't support Rust enums with variants, so we use strings at the boundary.
fn parse_option_type(s: &str) -> Result<OptionType, JsError> {
    match s.to_lowercase().as_str() {
        "call" => Ok(OptionType::Call),
        "put" => Ok(OptionType::Put),
        _ => Err(JsError::new(&format!(
            "invalid option type: '{}' (expected 'call' or 'put')", s
        ))),
    }
}

// Converts "european"/"american" string from JS into ExerciseStyle.
fn parse_exercise_style(s: &str) -> Result<ExerciseStyle, JsError> {
    match s.to_lowercase().as_str() {
        "european" => Ok(ExerciseStyle::European),
        "american" => Ok(ExerciseStyle::American),
        _ => Err(JsError::new(&format!(
            "invalid exercise style: '{}' (expected 'european' or 'american')", s
        ))),
    }
}

// Builds an OptionContract from flat args.
// Every WASM-exposed function needs to do this, so we centralize it here.
fn build_contract(
    s: f64,
    k: f64,
    t: f64,
    r: f64,
    sigma: f64,
    q: f64,
    option_type: &str,
    exercise_style: ExerciseStyle,
) -> Result<OptionContract, JsError> {
    Ok(OptionContract {
        s,
        k,
        t,
        r,
        sigma,
        option_type: parse_option_type(option_type)?,
        exercise_style,
        q: if q == 0.0 { None } else { Some(q) }, // 0.0 means no dividend
    })
}

// Black-Scholes pricing — the analytical formula for European options.
// Returns a JS object: { price, delta, gamma, theta, vega, rho, implied_volatility }
#[wasm_bindgen]
pub fn bs_price(
    s: f64,           // spot price
    k: f64,           // strike price
    t: f64,           // time to expiry (years)
    r: f64,           // risk-free rate
    sigma: f64,       // volatility
    q: f64,           // dividend yield (0.0 if none)
    option_type: &str, // "call" or "put"
) -> Result<JsValue, JsError> {
    let contract = build_contract(s, k, t, r, sigma, q, option_type, ExerciseStyle::European)?;
    let result = crate::black_scholes::black_scholes(&contract)
        .map_err(|e| JsError::new(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsError::new(&e.to_string()))
}

// Implied volatility solver — finds the sigma that makes BS match the market price.
// Returns a single f64 (the solved IV), not a full PricingResult.
#[wasm_bindgen]
pub fn iv_solve(
    market_price: f64,
    s: f64,
    k: f64,
    t: f64,
    r: f64,
    q: f64,
    option_type: &str,
) -> Result<f64, JsError> {
    // sigma here is just the initial guess — the solver overrides it internally
    let contract = build_contract(s, k, t, r, 0.2, q, option_type, ExerciseStyle::European)?;
    crate::implied_volatility::implied_volatility(market_price, &contract)
        .map_err(|e| JsError::new(&e.to_string()))
}

// Binomial tree pricing — handles both American and European exercise.
// This is the only pricer that supports early exercise (American options).
// Returns a JS object: { price, delta, gamma, theta, vega, rho, implied_volatility }
#[wasm_bindgen]
pub fn tree_price(
    s: f64,
    k: f64,
    t: f64,
    r: f64,
    sigma: f64,
    q: f64,
    option_type: &str,     // "call" or "put"
    exercise_style: &str,  // "european" or "american"
    steps: usize,          // tree depth — 200 is a good default
) -> Result<JsValue, JsError> {
    let style = parse_exercise_style(exercise_style)?;
    let contract = build_contract(s, k, t, r, sigma, q, option_type, style)?;
    let result = crate::binomial::binomial_price(&contract, steps)
        .map_err(|e| JsError::new(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsError::new(&e.to_string()))
}
