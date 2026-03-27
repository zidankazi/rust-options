// Monte Carlo pricing engine.
// GBM path simulation, antithetic variates, bump-and-reprice Greeks.
// Uses custom xorshift64 PRNG + Box-Muller — no rand crate.

use crate::error::PricerError;
use crate::types::{MonteCarloConfig, OptionContract, PricingResult};

// TODO: implement monte_carlo_price(contract, config) -> Result<PricingResult, PricerError>
//       - Geometric Brownian Motion path generation
//       - Antithetic variates for variance reduction
//       - Delta via bump-and-reprice (dS = 0.01 * S)
//       - Target: 100K paths in under 15ms

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: MC convergence — MC price approaches BS price as paths increase
}
