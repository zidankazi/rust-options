// Implied volatility solver.
// Newton-Raphson with vega as derivative, bisection fallback.

use crate::black_scholes::black_scholes;
use crate::error::PricerError;
use crate::types::OptionContract;

// Finds the volatility that makes the Black-Scholes price match the market price.
// Uses Newton-Raphson with vega as derivative, falls back to bisection.
pub fn implied_volatility(
    market_price: f64,
    contract: &OptionContract,
) -> Result<f64, PricerError> {
    let max_iter = 100;
    let tol = 1e-8;

    // Newton-Raphson
    let mut sigma = 0.2; // 20% annualized volatility is a rough average for U.S. equities
    for _ in 0..max_iter {
        let mut c = *contract; // dereference and copy contract
        c.sigma = sigma; // set sigma to current guess

        let result = black_scholes(&c)?;
        let diff = result.price - market_price;

        // Found it (Close enough)
        if diff.abs() < tol {
            return Ok(sigma);
        }

        // Break if vega is near zero (Won't work with Newton-Raphson)
        if result.vega.abs() < 1e-10 {
            break;
        }

        // Calculate new sigma
        let new_sigma = sigma - diff/result.vega;

        // Break if new sigma is out of bounds
        if new_sigma < 0.0 || new_sigma > 5.0 {
            break;
        }

        // Update sigma
        sigma = new_sigma; 
    }

    // Bisection fallback (basically just brute force/binary search)
    let mut lo = 0.001;
    let mut hi = 5.0;

      for _ in 0..max_iter {
          let mid = (lo + hi) / 2.0;
          let mut c = *contract;
          c.sigma = mid;
          let result = black_scholes(&c)?;

          let diff = result.price - market_price;

          if diff.abs() < tol {
              return Ok(mid);
          }

          // if BS price is too high, sigma is too high — search lower half
          // if BS price is too low, sigma is too low — search upper half
          if diff > 0.0 {
              hi = mid;
          } else {
              lo = mid;
          }
      }

      Err(PricerError::ConvergenceFailure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ExerciseStyle, OptionType};

    fn test_contract(sigma: f64, option_type: OptionType) -> OptionContract {
        OptionContract {
            s: 100.0,
            k: 100.0,
            t: 1.0,
            r: 0.05,
            sigma,
            option_type,
            exercise_style: ExerciseStyle::European,
            q: None,
        }
    }

    // Round-trip: price an option with known sigma, then recover sigma from the price.
    // If you get back what you put in, the solver works.
    #[test]
    fn round_trip_call() {
        let contract = test_contract(0.2, OptionType::Call);
        let price = black_scholes(&contract).unwrap().price;
        let iv = implied_volatility(price, &contract).unwrap();
        assert!((iv - 0.2).abs() < 1e-6);
    }

    #[test]
    fn round_trip_put() {
        let contract = test_contract(0.2, OptionType::Put);
        let price = black_scholes(&contract).unwrap().price;
        let iv = implied_volatility(price, &contract).unwrap();
        assert!((iv - 0.2).abs() < 1e-6);
    }

    // High vol: sigma = 0.8 (80%). Makes sure it works outside the "normal" range.
    #[test]
    fn round_trip_high_vol() {
        let contract = test_contract(0.8, OptionType::Call);
        let price = black_scholes(&contract).unwrap().price;
        let iv = implied_volatility(price, &contract).unwrap();
        assert!((iv - 0.8).abs() < 1e-6);
    }

    // Low vol: sigma = 0.05 (5%). Near the lower bound — tests Newton's initial guess being far off.
    #[test]
    fn round_trip_low_vol() {
        let contract = test_contract(0.05, OptionType::Call);
        let price = black_scholes(&contract).unwrap().price;
        let iv = implied_volatility(price, &contract).unwrap();
        assert!((iv - 0.05).abs() < 1e-6);
    }

    // Deep ITM: stock at 150, strike at 100. Vega is small here — tests bisection fallback.
    #[test]
    fn deep_itm() {
        let mut contract = test_contract(0.3, OptionType::Call);
        contract.s = 150.0;
        let price = black_scholes(&contract).unwrap().price;
        let iv = implied_volatility(price, &contract).unwrap();
        assert!((iv - 0.3).abs() < 1e-4);
    }

    // Deep OTM: stock at 50, strike at 100. Price is tiny, vega is tiny.
    #[test]
    fn deep_otm() {
        let mut contract = test_contract(0.3, OptionType::Call);
        contract.s = 50.0;
        let price = black_scholes(&contract).unwrap().price;
        let iv = implied_volatility(price, &contract).unwrap();
        assert!((iv - 0.3).abs() < 1e-4);
    }
}
