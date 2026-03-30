// Binomial tree pricing — supports American and European options.
// Cox-Ross-Rubinstein (CRR) parameterization.
// The only pricer that handles early exercise (American options).

use crate::error::PricerError;
use crate::types::{ExerciseStyle, OptionContract, OptionType, PricingResult};

// Prices an option by building a tree of all possible stock prices,
// then working backwards from expiry to today.
pub fn binomial_price(contract: &OptionContract, steps: usize) -> Result<PricingResult, PricerError> {
    let s = contract.s; // initial stock price
    let k = contract.k; // strike price
    let t = contract.t; // time to expiration (in years)
    let r = contract.r; // risk-free rate (annualized)
    let q = contract.q();  // dividend yield (annualized)
    let sigma = contract.sigma; // volatility (annualized)

    // CRR parameters — define the shape of the tree
    let dt = t / steps as f64;                     // time per step
    let u = (sigma * dt.sqrt()).exp();             // up factor — how much the stock jumps up each step
    let d = 1.0 / u;                               // down factor — always the inverse of up
    let p = (((r - q) * dt).exp() - d) / (u - d);  // probability of going up (risk-neutral) 
    let disc = (-r * dt).exp();                    // discount factor per step

    // Forward pass: compute payoffs at every possible expiry price
    // After `steps` steps there are steps+1 possible ending prices
    let u_over_d = u * u; // u/d = u/(1/u) = u^2 — ratio between consecutive spots
    let mut values = vec![0.0; steps + 1]; // values[i] = option value at node i
    let mut spot = s * d.powi(steps as i32); // start at all-down: s * d^steps
    for i in 0..=steps {
        values[i] = match contract.option_type {
            OptionType::Call => (spot - k).max(0.0), // call payoff = max(spot - strike, 0)
            OptionType::Put => (k - spot).max(0.0), // put payoff = max(strike - spot, 0)
        };
        spot *= u_over_d; // next node's spot = previous * u/d
    }

    // Backward pass: work from expiry back to today, one step at a time
    for step in (0..steps).rev() { // Iterate in reverse
        let mut spot = s * d.powi(step as i32); // start at all-down for this step
        for i in 0..=step {
            // Continuation value: discounted weighted average of up and down futures
            values[i] = disc * (p * values[i + 1] + (1.0 - p) * values[i]);

            // American options: check if exercising now beats waiting
            if contract.exercise_style == ExerciseStyle::American {
                let exercise = match contract.option_type {
                    OptionType::Call => (spot - k).max(0.0),
                    OptionType::Put => (k - spot).max(0.0),
                };
                values[i] = values[i].max(exercise);
            }
            spot *= u_over_d; // next node's spot
        }

        // After processing step 1, save the two values needed for delta before they get overwritten on the final iteration
        // To calculate delta, we need the calculated option values at the very first branches of the tree (v_up and v_down)
        if step == 1 { // Second to last step
            let v_up = values[1];   // option value if stock goes up first step
            let v_down = values[0]; // option value if stock goes down first step

            // Finish the last backward step (step 0) manually
            values[0] = disc * (p * values[1] + (1.0 - p) * values[0]);
            if contract.exercise_style == ExerciseStyle::American {
                let exercise = match contract.option_type {
                    OptionType::Call => (s - k).max(0.0),
                    OptionType::Put => (k - s).max(0.0),
                };
                values[0] = values[0].max(exercise);
            }

            let price = values[0];

            // Delta: how much the option value changes between the up and down nodes
            // Same idea as bump-and-reprice, but the tree gives it naturally
            let delta = (v_up - v_down) / (s * u - s * d);

            return Ok(PricingResult {
                price,
                delta,
                gamma: 0.0,
                theta: 0.0,
                vega: 0.0,
                rho: 0.0,
                implied_volatility: None,
            });
        }
    }

    // Fallback for steps <= 1
    Ok(PricingResult {
        price: values[0],
        delta: 0.0,
        gamma: 0.0,
        theta: 0.0,
        vega: 0.0,
        rho: 0.0,
        implied_volatility: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::black_scholes::black_scholes;

    // Helper function to create a European call or put option contract
    fn euro_contract(option_type: OptionType) -> OptionContract {
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

    // European binomial should converge to BS as steps increase.
    // At 200 steps it should be within a few cents.
    #[test]
    fn converges_to_bs_call() {
        let contract = euro_contract(OptionType::Call);
        let bs = black_scholes(&contract).unwrap().price;
        let bin = binomial_price(&contract, 200).unwrap().price;
        assert!((bin - bs).abs() < 0.05);
    }

    #[test]
    fn converges_to_bs_put() {
        let contract = euro_contract(OptionType::Put);
        let bs = black_scholes(&contract).unwrap().price;
        let bin = binomial_price(&contract, 200).unwrap().price;
        assert!((bin - bs).abs() < 0.05);
    }

    // American put should be worth MORE than European put (early exercise has value).
    // This is the whole reason the tree exists.
    #[test]
    fn american_put_exceeds_european() {
        let mut american = euro_contract(OptionType::Put);
        american.exercise_style = ExerciseStyle::American;
        let euro_price = binomial_price(&euro_contract(OptionType::Put), 200).unwrap().price;
        let amer_price = binomial_price(&american, 200).unwrap().price;
        assert!(amer_price >= euro_price);
    }

    // Delta should be reasonable for an ATM call (~0.5-0.6)
    #[test]
    fn delta_reasonable() {
        let contract = euro_contract(OptionType::Call);
        let result = binomial_price(&contract, 200).unwrap();
        assert!(result.delta > 0.4 && result.delta < 0.8);
    }

    // More steps = more accurate. 500 steps should be closer to BS than 50 steps.
    #[test]
    fn more_steps_more_accurate() {
        let contract = euro_contract(OptionType::Call);
        let bs = black_scholes(&contract).unwrap().price;
        let bin_50 = binomial_price(&contract, 50).unwrap().price;
        let bin_500 = binomial_price(&contract, 500).unwrap().price;
        assert!((bin_500 - bs).abs() < (bin_50 - bs).abs());
    }
}
