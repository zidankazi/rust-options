pub mod error;
pub mod types;
pub mod normal;
pub mod rng;
pub mod black_scholes;
pub mod implied_volatility;
pub mod binomial;
// pub mod heston; // uncomment when heston.rs is finished

// Monte Carlo uses rayon (OS threads) which doesn't exist in WASM.
// The whole module is skipped when compiling for wasm32.
#[cfg(not(target_arch = "wasm32"))]
pub mod monte_carlo;

// WASM bindings — only compiled when building with `wasm-pack build --features wasm`
#[cfg(feature = "wasm")]
pub mod wasm;

pub use error::PricerError;
pub use types::{ExerciseStyle, MonteCarloConfig, OptionContract, OptionType, PricingResult};
