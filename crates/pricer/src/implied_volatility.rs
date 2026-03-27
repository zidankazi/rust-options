// Implied volatility solver.
// Newton-Raphson with vega as derivative, bisection fallback.

use crate::error::PricerError;
use crate::types::OptionContract;

// TODO: implement implied_volatility(market_price, contract) -> Result<f64, PricerError>
//       - Newton-Raphson: sigma_{n+1} = sigma_n - (bs_price - market_price) / vega
//       - Tolerance: 1e-8, max 100 iterations
//       - Fallback to bisection if Newton diverges or vega ~ 0
//       - Handle edge cases: deep ITM/OTM, near-expiry

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: IV round-trip — price an option, extract IV, verify it matches original sigma
    // TODO: edge cases — deep ITM, deep OTM, near-expiry
}
