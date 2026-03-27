// Black-Scholes pricing for European options with Merton extension (continuous dividends).
// Pure math — zero allocations in the hot path.

use crate::error::PricerError;
use crate::types::{OptionContract, PricingResult};

// TODO: implement validate(contract) -> Result<(), PricerError>
// TODO: implement d1(s, k, t, r, q, sigma) -> f64
// TODO: implement d2(d1, sigma, t) -> f64
// TODO: implement price(contract) -> Result<f64, PricerError>
// TODO: implement delta(contract) -> Result<f64, PricerError>
// TODO: implement gamma(contract) -> Result<f64, PricerError>
// TODO: implement theta(contract) -> Result<f64, PricerError>
// TODO: implement vega(contract) -> Result<f64, PricerError>
// TODO: implement rho(contract) -> Result<f64, PricerError>
// TODO: implement black_scholes(contract) -> Result<PricingResult, PricerError>
//       ^ computes all Greeks in one pass, reusing d1/d2

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: known-value test — BS call S=100, K=100, T=1, r=0.05, sigma=0.2 => ~10.4506
    // TODO: put-call parity — C - P = S*exp(-qT) - K*exp(-rT)
    // TODO: greek symmetry — put delta = call delta - exp(-qT)
    // TODO: edge cases — ATM, deep ITM, deep OTM, near-expiry, zero vol
    // TODO: error cases — negative vol, negative time should return Err
}
