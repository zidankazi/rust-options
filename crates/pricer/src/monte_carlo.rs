// Monte Carlo pricing engine.
// GBM path simulation, antithetic variates, bump-and-reprice Greeks.
// Uses custom xorshift64 PRNG + Box-Muller — no rand crate.
  
use crate::types::{MonteCarloConfig, OptionContract, OptionType, PricingResult};
use crate::error::PricerError;
use crate::rng::Xorshift64;
use rayon::prelude::*;

// Runs the core MC simulation and returns just the price, no Greeks.
// Separated so bump-and-reprice can call it without infinite recursion.
fn simulate_price(contract: &OptionContract, config: &MonteCarloConfig) -> f64 {
    let s = contract.s; // initial stock price
    let k = contract.k; // strike price
    let t = contract.t; // time to expiration (in years)
    let r = contract.r; // risk-free rate (annualized)
    let q = contract.q(); // dividend yield (annualized)
    let sigma = contract.sigma; // volatility (annualized)

    let drift = (r - q - 0.5 * sigma * sigma) * t; // total drift over full time horizon
    let vol_sqrt_t = sigma * t.sqrt(); // volatility scaled by sqrt(T)
    let discount = (-r * t).exp(); // discount factor

    let seed = config.seed.unwrap_or(12345); // seed for reproducibility
    let num_pairs = config.num_paths / 2; // number of pairs of paths

    let total_payoff: f64 = (0..num_pairs) // iterate over pairs of paths
        .into_par_iter() // parallel iteration
        .map(|i| {
            let mut rng = Xorshift64::new(seed + i as u64); // create new PRNG for each pair
            let (z1, _) = rng.next_normal_pair(); // generate pair of standard normals

            let s1 = s * (drift + vol_sqrt_t * z1).exp(); // simulate path 1
            let s2 = s * (drift - vol_sqrt_t * z1).exp(); // simulate path 2

            let payoff1 = match contract.option_type {
                OptionType::Call => (s1 - k).max(0.0), // call payoff (S - K) if S > K else 0
                OptionType::Put => (k - s1).max(0.0), // put payoff (K - S) if K > S else 0
            };
            let payoff2 = match contract.option_type {
                OptionType::Call => (s2 - k).max(0.0), // call payoff (S - K) if S > K else 0
                OptionType::Put => (k - s2).max(0.0), // put payoff (K - S) if K > S else 0
            };

            (payoff1 + payoff2) / 2.0 // average payoff of the pair
        })
        .sum();

    discount * total_payoff / num_pairs as f64 // discount the average payoff
}

// Monte Carlo pricer — simulates the price, then computes delta via bump-and-reprice.
pub fn monte_carlo_price(
    contract: &OptionContract,
    config: &MonteCarloConfig,
) -> Result<PricingResult, PricerError> {
    let price = simulate_price(contract, config);

    // Delta via bump-and-reprice
    // No formula for delta in MC, so we measure it directly:
    // nudge the stock price up 1%, reprice, nudge down 1%, reprice,
    // delta = (price_up - price_down) / total_nudge_size
    let bump = 0.01 * contract.s; // 1% of stock price
    let mut bumped = *contract;
    bumped.s = contract.s + bump;
    let price_up = simulate_price(&bumped, config); // reprice with nudged-up stock
    bumped.s = contract.s - bump;
    let price_down = simulate_price(&bumped, config); // reprice with nudged-down stock
    let delta = (price_up - price_down) / (2.0 * bump); // slope = rise / run

    Ok(PricingResult {
        price,
        delta,
        gamma: 0.0,     // not computed for MC
        theta: 0.0,     // not computed for MC
        vega: 0.0,      // not computed for MC
        rho: 0.0,       // not computed for MC
        implied_volatility: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::black_scholes::black_scholes;
    use crate::types::ExerciseStyle;

    fn test_contract(option_type: OptionType) -> OptionContract {
        OptionContract {
            s: 100.0,
            k: 100.0,
            t: 1.0,
            r: 0.05,
            sigma: 0.2,
            option_type,
            exercise_style: ExerciseStyle::European,
            q: None,
        }
    }

    // MC price should converge to BS price with enough paths.
    // BS gives the exact answer; MC should get close with 200K paths.
    #[test]
    fn converges_to_bs_call() {
        let contract = test_contract(OptionType::Call);
        let bs_price = black_scholes(&contract).unwrap().price;
        let config = MonteCarloConfig {
            num_paths: 200_000,
            num_steps: 252,
            seed: Some(42),
        };
        let mc = monte_carlo_price(&contract, &config).unwrap();
        assert!((mc.price - bs_price).abs() < 0.15);
    }

    #[test]
    fn converges_to_bs_put() {
        let contract = test_contract(OptionType::Put);
        let bs_price = black_scholes(&contract).unwrap().price;
        let config = MonteCarloConfig {
            num_paths: 200_000,
            num_steps: 252,
            seed: Some(42),
        };
        let mc = monte_carlo_price(&contract, &config).unwrap();
        assert!((mc.price - bs_price).abs() < 0.15);
    }

    // Delta should be in a reasonable range for an ATM call (~0.5-0.6)
    #[test]
    fn delta_reasonable() {
        let contract = test_contract(OptionType::Call);
        let config = MonteCarloConfig {
            num_paths: 100_000,
            num_steps: 252,
            seed: Some(99),
        };
        let mc = monte_carlo_price(&contract, &config).unwrap();
        assert!(mc.delta > 0.4 && mc.delta < 0.8);
    }

    // Same seed should give very close results. Not bit-exact because
    // parallel summation order can vary, but within floating-point noise.
    #[test]
    fn reproducible_with_seed() {
        let contract = test_contract(OptionType::Call);
        let config = MonteCarloConfig {
            num_paths: 10_000,
            num_steps: 252,
            seed: Some(777),
        };
        let a = monte_carlo_price(&contract, &config).unwrap().price;
        let b = monte_carlo_price(&contract, &config).unwrap().price;
        assert!((a - b).abs() < 1e-10);
    }
}
