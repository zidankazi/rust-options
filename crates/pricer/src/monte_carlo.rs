// Monte Carlo pricing engine.
// GBM path simulation, antithetic variates, bump-and-reprice Greeks.
// Uses custom xorshift64 PRNG + Box-Muller — no rand crate.
  
use crate::types::{MonteCarloConfig, OptionContract, OptionType, PricingResult};
use crate::error::PricerError;
use crate::rng::Xorshift64;

// Runs the core MC simulation and returns just the price, no Greeks.
// Separated so bump-and-reprice can call it without infinite recursion.
fn simulate_price(contract: &OptionContract, config: &MonteCarloConfig) -> f64 {
    let s = contract.s;
    let k = contract.k;
    let t = contract.t;
    let r = contract.r;
    let q = contract.q();
    let sigma = contract.sigma;

    let dt = t / config.num_steps as f64; // time per step
    let drift = (r - q - 0.5 * sigma * sigma) * dt; // deterministic drift per step
    let vol_step = sigma * dt.sqrt(); // volatility scaling per step
    let discount = (-r * t).exp(); // discount factor

    let seed = config.seed.unwrap_or(12345);
    let mut rng = Xorshift64::new(seed);
    let mut total_payoff = 0.0;

    for _ in 0..(config.num_paths / 2) {
        let mut s1 = s; // normal path (uses Z)
        let mut s2 = s; // antithetic path (uses -Z)

        // Simulate GBM path (Geometric Brownian Motion)
        for _ in 0..config.num_steps {
            let (z, _) = rng.next_normal_pair();
            s1 *= (drift + vol_step * z).exp();
            s2 *= (drift - vol_step * z).exp();
        }

        // Calculate payoff for both paths
        let payoff1 = match contract.option_type {
            OptionType::Call => (s1 - k).max(0.0),
            OptionType::Put => (k - s1).max(0.0),
        };
        let payoff2 = match contract.option_type {
            OptionType::Call => (s2 - k).max(0.0),
            OptionType::Put => (k - s2).max(0.0),
        };

        total_payoff += (payoff1 + payoff2) / 2.0;
    }

    let num_pairs = (config.num_paths / 2) as f64;
    discount * total_payoff / num_pairs
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

    // Same seed = same result. Different seed = different result.
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
        assert_eq!(a, b);
    }
}
