// Binomial tree pricing — supports American and European options.
// Cox-Ross-Rubinstein parameterization.

use crate::error::PricerError;
use crate::types::{OptionContract, PricingResult};

// TODO: implement binomial_price(contract, steps) -> Result<PricingResult, PricerError>
//       - CRR: u = exp(sigma*sqrt(dt)), d = 1/u, p = (exp((r-q)*dt) - d) / (u - d)
//       - Early exercise check at each node for American options
//       - Delta from first tree step
//       - Default 200 steps

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: binomial convergence — binomial approaches BS as steps increase (European)
    // TODO: American put price > European put price (early exercise premium)
}
