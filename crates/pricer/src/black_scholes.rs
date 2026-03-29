// Black-Scholes pricing for European options with Merton extension (continuous dividends).
// Pure math — zero allocations in the hot path.

use crate::error::PricerError;
use crate::normal::{norm_cdf as cdf, norm_pdf as pdf};
use crate::types::{OptionContract, OptionType, PricingResult};

// Validates contract inputs before pricing.
// Rejects nonsensical values (negative prices, zero time, etc.) that would -
// - produce NaN/Inf in the formulas rather than meaningful results.
fn validate(contract: &OptionContract) -> Result<(), PricerError> {
    if contract.s <= 0.0 {
        return Err(PricerError::InvalidInput(
            "Stock price must be positive".to_string(),
        ));
    }
    if contract.k <= 0.0 {
        return Err(PricerError::InvalidInput(
            "Strike price must be positive".to_string(),
        ));
    }
    if contract.t <= 0.0 {
        return Err(PricerError::InvalidInput(
            "Time to expiration must be positive".to_string(),
        ));
    }
    if contract.r < 0.0 {
        return Err(PricerError::InvalidInput(
            "Risk-free rate must be non-negative".to_string(),
        ));
    }
    if contract.q() < 0.0 {
        // use q() instead of q because contract.q is a Option<f64>, not an f64. .q() unwraps the value safely.
        return Err(PricerError::InvalidInput(
            "Dividend yield must be non-negative".to_string(),
        ));
    }
    if contract.sigma <= 0.0 {
        return Err(PricerError::InvalidInput(
            "Volatility must be positive".to_string(),
        ));
    }
    Ok(())
}

// Computes price and all Greeks in one pass, reusing intermediate values.
// This is the main entry point for Black-Scholes pricing.
pub fn black_scholes(contract: &OptionContract) -> Result<PricingResult, PricerError> {
    validate(contract)?;

    let s = contract.s;
    let k = contract.k;
    let t = contract.t;
    let r = contract.r;
    let q = contract.q(); // must use .q() since q is an Option<f64>, not an f64, so the value must be safely unwrapped
    let sigma = contract.sigma;

    // d1: how many std devs (spread) the stock is above the strike, adjusted for drift and time
    let d1 = ((s / k).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
    // d2: d1 shifted down by total volatility over the period
    let d2 = d1 - sigma * t.sqrt();

    // cache values that show up in multiple formulas
    let cdf_d1 = cdf(d1); // N(d1) — probability-weighted stock exposure
    let cdf_d2 = cdf(d2); // N(d2) — roughly probability of expiring ITM
    let pdf_d1 = pdf(d1); // n(d1) — bell curve height at d1, used for gamma/vega/theta
    let exp_qt = (-q * t).exp(); // dividend discount factor
    let exp_rt = (-r * t).exp(); // risk-free discount factor
    let sqrt_t = t.sqrt();

    // price, delta, theta, rho differ for calls vs puts — compute in one match block
    // price: expected value of what you get minus what you pay, discounted to today
    // delta: how much the option price moves per $1 move in the stock
    // theta: how much value the option loses per day from time passing (almost always negative)
    //        three terms: volatility decay + interest cost/gain + dividend cost/gain
    // rho: how much the price changes per unit change in interest rate
    let (price, delta, theta, rho) = match contract.option_type {
        OptionType::Call => {
            let price = s * exp_qt * cdf_d1 - k * exp_rt * cdf_d2;
            let delta = exp_qt * cdf_d1;
            let theta = (-s * exp_qt * pdf_d1 * sigma / (2.0 * sqrt_t))
                - r * k * exp_rt * cdf_d2
                + q * s * exp_qt * cdf_d1;
            let rho = k * t * exp_rt * cdf_d2;
            (price, delta, theta, rho)
        }
        OptionType::Put => {
            let price = k * exp_rt * (1.0 - cdf_d2) - s * exp_qt * (1.0 - cdf_d1);
            let delta = exp_qt * (cdf_d1 - 1.0);
            let theta = (-s * exp_qt * pdf_d1 * sigma / (2.0 * sqrt_t))
                + r * k * exp_rt * (1.0 - cdf_d2)
                - q * s * exp_qt * (1.0 - cdf_d1);
            let rho = -k * t * exp_rt * (1.0 - cdf_d2);
            (price, delta, theta, rho)
        }
    };

    // gamma: how fast delta changes when the stock moves — same for calls and puts
    let gamma = exp_qt * pdf_d1 / (s * sigma * sqrt_t);

    // vega: how much the price changes per unit change in volatility — same for calls and puts
    let vega = s * exp_qt * pdf_d1 * sqrt_t;

    Ok(PricingResult {
        price,
        delta,
        gamma,
        theta,
        vega,
        rho,
        implied_volatility: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ExerciseStyle;

    // helper: standard ATM contract used by most tests
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

    // Known-value test: does the formula produce the right number?
    // The classic BS test case. If this passes, the core math is correct.
    #[test]
    fn call_price_known_value() {
        let result = black_scholes(&test_contract(OptionType::Call)).unwrap();
        assert!((result.price - 10.4506).abs() < 0.01);
    }

    // Put-call parity: C - P = S*exp(-qT) - K*exp(-rT).
    // Call price minus put price must always equal the current stock price minus the discounted strike price.
    // Arbitrage law. If this fails, one of the formulas has a bug.
    #[test]
    fn put_call_parity() {
        let call = black_scholes(&test_contract(OptionType::Call)).unwrap();
        let put = black_scholes(&test_contract(OptionType::Put)).unwrap();
        let s = 100.0_f64;
        let k = 100.0_f64;
        let r = 0.05_f64;
        let t = 1.0_f64;
        // q = 0, so exp(-qT) = 1.0
        let expected = s - k * (-r * t).exp();
        assert!((call.price - put.price - expected).abs() < 1e-6);
    }

    // Greek symmetry: put delta = call delta - exp(-qT)
    // Follows from put-call parity. If delta is wrong for either side, this catches it.
    #[test]
    fn delta_symmetry() {
        let call = black_scholes(&test_contract(OptionType::Call)).unwrap();
        let put = black_scholes(&test_contract(OptionType::Put)).unwrap();
        // q = 0, so exp(-qT) = 1.0
        assert!((put.delta - (call.delta - 1.0)).abs() < 1e-6);
    }

    // Gamma and vega should be identical for call and put with same inputs.
    #[test]
    fn gamma_vega_same_for_call_and_put() {
        let call = black_scholes(&test_contract(OptionType::Call)).unwrap();
        let put = black_scholes(&test_contract(OptionType::Put)).unwrap();
        assert!((call.gamma - put.gamma).abs() < 1e-10);
        assert!((call.vega - put.vega).abs() < 1e-10);
    }

    // Deep ITM call: price ≈ S - K*exp(-rT), delta ≈ 1.0
    // When the stock is way above the strike, the option is basically the stock.
    #[test]
    fn deep_itm_call() {
        let mut c = test_contract(OptionType::Call);
        c.s = 200.0; // stock at 200, strike at 100
        let result = black_scholes(&c).unwrap();
        let intrinsic = 200.0 - 100.0 * (-0.05_f64).exp();
        assert!((result.price - intrinsic).abs() < 1.0);
        assert!(result.delta > 0.99);
    }

    // Deep OTM call: price ≈ 0, delta ≈ 0
    // Stock is way below the strike — almost no chance of expiring ITM.
    #[test]
    fn deep_otm_call() {
        let mut c = test_contract(OptionType::Call);
        c.s = 10.0; // stock at 10, strike at 100
        let result = black_scholes(&c).unwrap();
        assert!(result.price < 0.01);
        assert!(result.delta < 0.01);
    }

    // Error cases: bad inputs should return Err, not NaN or panic
    #[test]
    fn rejects_negative_vol() {
        let mut c = test_contract(OptionType::Call);
        c.sigma = -0.2;
        assert!(black_scholes(&c).is_err());
    }

    #[test]
    fn rejects_negative_time() {
        let mut c = test_contract(OptionType::Call);
        c.t = -1.0;
        assert!(black_scholes(&c).is_err());
    }

    #[test]
    fn rejects_zero_spot() {
        let mut c = test_contract(OptionType::Call);
        c.s = 0.0;
        assert!(black_scholes(&c).is_err());
    }
}
