// SVI (Stochastic Volatility Inspired) smile calibration.
// Fits a 5-parameter curve to a single expiry's worth of market IVs.
// Gives you smooth, arbitrage-aware IV at any strike.

use crate::error::PricerError;

// The five knobs — output of calibration, input to interpolation.
pub struct SviParams {
    pub a: f64,     // level — shifts the whole curve up or down
    pub b: f64,     // steepness — how aggressively the wings rise (>= 0)
    pub rho: f64,   // tilt — which wing is steeper (-1 to 1), same idea as stock-vol correlation
    pub m: f64,     // center — where the bottom of the smile sits
    pub sigma: f64, // roundness — sharp V vs smooth U at the bottom (> 0)
}

// One market observation to fit against.
pub struct SviPoint {
    pub k: f64, // log-moneyness: ln(K / F) where F = forward price
    pub w: f64, // total implied variance: iv^2 * T
}

// Evaluates the SVI curve at a given log-moneyness.
// Returns total implied variance w(k).
// The shape is a tilted hyperbola — rho tilts it, m centers it,
// sigma rounds the bottom, b scales it, a shifts it up.
pub fn svi_variance(params: &SviParams, k: f64) -> f64 {
    let x = k - params.m; // log-moneyness relative to center
    params.a + params.b * (params.rho * x + (x * x + params.sigma * params.sigma).sqrt()) // total variance
}

// Converts SVI total variance back to an implied volatility.
// total variance w = iv^2 * T, so iv = sqrt(w / T).
pub fn svi_iv(params: &SviParams, k: f64, t: f64) -> Result<f64, PricerError> {
    if t <= 0.0 {
        return Err(PricerError::InvalidInput(
            "Time to expiry must be positive".to_string(),
        ));
    }
    let w = svi_variance(params, k); // total implied variance
    if w < 0.0 {
        return Err(PricerError::NumericalInstability);
    }
    Ok((w / t).sqrt()) // iv = sqrt(w / T)
}

// Calibrates SVI parameters to market observations.
// Takes market (log-moneyness, total variance) points and finds
// the five knobs that best fit the data.
// Uses Nelder-Mead — no derivatives needed, works well for 5 params.
pub fn svi_calibrate(points: &[SviPoint]) -> Result<SviParams, PricerError> {
    if points.len() < 5 {
        return Err(PricerError::InvalidInput(
            "Need at least 5 points to fit 5 parameters".to_string(),
        ));
    }

    // objective: sum of squared errors between SVI curve and market data
    let objective = |p: &[f64; 5]| -> f64 {
        // enforce parameter bounds — return huge cost if violated
        let b = p[1];
        let rho = p[2];
        let sigma = p[4];
        if b < 0.0 || rho <= -1.0 || rho >= 1.0 || sigma <= 0.0 {
            return 1e18;
        }

        let params = SviParams {
            a: p[0],
            b,
            rho,
            m: p[3],
            sigma,
        };
        points
            .iter()
            .map(|pt| {
                let diff = svi_variance(&params, pt.k) - pt.w;
                diff * diff
            })
            .sum()
    };

    // initial guess — estimated from the data
    let w_mean: f64 = points.iter().map(|p| p.w).sum::<f64>() / points.len() as f64;
    let k_mean: f64 = points.iter().map(|p| p.k).sum::<f64>() / points.len() as f64;
    let initial: [f64; 5] = [
        w_mean, // a — start near the average total variance
        0.1,    // b — moderate steepness
        -0.3,   // rho — typical equity skew
        k_mean, // m — center near the average log-moneyness
        0.1,    // sigma — moderate roundness
    ];

    let best = nelder_mead(&objective, &initial, 10000, 1e-12);

    Ok(SviParams {
        a: best[0],
        b: best[1],
        rho: best[2],
        m: best[3],
        sigma: best[4],
    })
}

// Nelder-Mead (downhill simplex) optimizer for 5 parameters.
// No derivatives needed — just evaluates the objective function at
// different points and crawls downhill by reflecting/expanding/contracting
// the worst point in the simplex.
fn nelder_mead(
    objective: &dyn Fn(&[f64; 5]) -> f64,
    initial: &[f64; 5],
    max_iter: usize,
    tol: f64,
) -> [f64; 5] {
    const N: usize = 5; // number of parameters
    const NP: usize = 6; // simplex has N+1 vertices

    // standard Nelder-Mead coefficients
    let alpha = 1.0; // reflection — try the opposite direction of the worst point
    let gamma = 2.0; // expansion — if reflection was great, go even further
    let rho = 0.5;   // contraction — if reflection was bad, try halfway
    let sigma = 0.5; // shrink — if nothing works, pull everything toward the best point

    // build initial simplex: start point + 5 perturbations (one per dimension)
    let mut simplex: [[f64; N]; NP] = [*initial; NP];
    for i in 0..N {
        let step = if initial[i].abs() > 1e-8 {
            initial[i] * 0.05 // 5% perturbation
        } else {
            0.00025 // small absolute step if near zero
        };
        simplex[i + 1][i] += step;
    }

    // evaluate objective at each vertex
    let mut costs: [f64; NP] = [0.0; NP];
    for i in 0..NP {
        costs[i] = objective(&simplex[i]);
    }

    for _ in 0..max_iter {
        // sort vertices by cost (best first, worst last)
        let mut order: [usize; NP] = [0, 1, 2, 3, 4, 5];
        order.sort_by(|&a, &b| costs[a].partial_cmp(&costs[b]).unwrap());
        let mut sorted_simplex = [[0.0; N]; NP];
        let mut sorted_costs = [0.0; NP];
        for i in 0..NP {
            sorted_simplex[i] = simplex[order[i]];
            sorted_costs[i] = costs[order[i]];
        }
        simplex = sorted_simplex;
        costs = sorted_costs;

        // check convergence: are all costs close enough?
        let spread = costs[NP - 1] - costs[0];
        if spread < tol {
            break;
        }

        // centroid of all points except the worst
        let mut centroid = [0.0; N];
        for i in 0..(NP - 1) {
            for j in 0..N {
                centroid[j] += simplex[i][j];
            }
        }
        for j in 0..N {
            centroid[j] /= (NP - 1) as f64;
        }

        let worst = simplex[NP - 1]; // worst point (highest cost)

        // 1. reflection — mirror the worst point through the centroid
        let mut reflected = [0.0; N];
        for j in 0..N {
            reflected[j] = centroid[j] + alpha * (centroid[j] - worst[j]);
        }
        let reflected_cost = objective(&reflected);

        if reflected_cost < costs[NP - 2] && reflected_cost >= costs[0] {
            // reflection is better than second-worst but not best — accept it
            simplex[NP - 1] = reflected;
            costs[NP - 1] = reflected_cost;
            continue;
        }

        if reflected_cost < costs[0] {
            // reflection is the new best — try expanding even further
            let mut expanded = [0.0; N];
            for j in 0..N {
                expanded[j] = centroid[j] + gamma * (reflected[j] - centroid[j]);
            }
            let expanded_cost = objective(&expanded);

            if expanded_cost < reflected_cost {
                simplex[NP - 1] = expanded; // expansion was even better
                costs[NP - 1] = expanded_cost;
            } else {
                simplex[NP - 1] = reflected; // reflection was good enough
                costs[NP - 1] = reflected_cost;
            }
            continue;
        }

        // reflection was worse than second-worst — try contraction
        let mut contracted = [0.0; N];
        for j in 0..N {
            contracted[j] = centroid[j] + rho * (worst[j] - centroid[j]);
        }
        let contracted_cost = objective(&contracted);

        if contracted_cost < costs[NP - 1] {
            // contraction improved on worst — accept it
            simplex[NP - 1] = contracted;
            costs[NP - 1] = contracted_cost;
            continue;
        }

        // nothing worked — shrink everything toward the best point
        let best = simplex[0];
        for i in 1..NP {
            for j in 0..N {
                simplex[i][j] = best[j] + sigma * (simplex[i][j] - best[j]);
            }
            costs[i] = objective(&simplex[i]);
        }
    }

    simplex[0] // return best vertex
}

#[cfg(test)]
mod tests {
    use super::*;

    // helper: build a set of points from known SVI params,
    // so we can test that calibration recovers them
    fn make_points(params: &SviParams, ks: &[f64]) -> Vec<SviPoint> {
        ks.iter()
            .map(|&k| SviPoint {
                k,
                w: svi_variance(params, k),
            })
            .collect()
    }

    // at k = m, the tilt term is zero and the hyperbola term = sigma.
    // so w(m) = a + b * sigma.
    #[test]
    fn variance_at_center_is_a_plus_b_sigma() {
        let params = SviParams {
            a: 0.04,
            b: 0.2,
            rho: -0.3,
            m: 0.0,
            sigma: 0.1,
        };
        let w = svi_variance(&params, 0.0);
        let expected = 0.04 + 0.2 * 0.1; // a + b * sigma
        assert!(
            (w - expected).abs() < 1e-12,
            "At center: got {w}, expected {expected}"
        );
    }

    // with no tilt, the smile should be symmetric around m
    #[test]
    fn symmetric_when_rho_is_zero() {
        let params = SviParams {
            a: 0.04,
            b: 0.2,
            rho: 0.0,
            m: 0.0,
            sigma: 0.1,
        };
        let left = svi_variance(&params, -0.2);
        let right = svi_variance(&params, 0.2);
        assert!(
            (left - right).abs() < 1e-12,
            "Should be symmetric: left={left}, right={right}"
        );
    }

    // negative rho = equity-style skew = left wing rises faster
    #[test]
    fn left_wing_steeper_when_rho_negative() {
        let params = SviParams {
            a: 0.04,
            b: 0.2,
            rho: -0.5,
            m: 0.0,
            sigma: 0.1,
        };
        let far_left = svi_variance(&params, -1.0);
        let far_right = svi_variance(&params, 1.0);
        assert!(
            far_left > far_right,
            "Left wing should be higher: left={far_left}, right={far_right}"
        );
    }

    // iv = sqrt(w / T), so iv^2 * T should give back w
    #[test]
    fn iv_conversion_roundtrips() {
        let params = SviParams {
            a: 0.04,
            b: 0.2,
            rho: -0.3,
            m: 0.0,
            sigma: 0.1,
        };
        let t = 0.5;
        let k = 0.1;
        let iv = svi_iv(&params, k, t).unwrap();
        let w_roundtrip = iv * iv * t;
        let w_direct = svi_variance(&params, k);
        assert!(
            (w_roundtrip - w_direct).abs() < 1e-12,
            "Roundtrip failed: {w_roundtrip} vs {w_direct}"
        );
    }

    // t must be positive — can't compute iv for zero or negative time
    #[test]
    fn rejects_non_positive_time() {
        let params = SviParams {
            a: 0.04,
            b: 0.2,
            rho: -0.3,
            m: 0.0,
            sigma: 0.1,
        };
        assert!(svi_iv(&params, 0.0, 0.0).is_err());
        assert!(svi_iv(&params, 0.0, -1.0).is_err());
    }

    // generate points from known params, calibrate, check we recover them
    #[test]
    fn recovers_known_params() {
        let true_params = SviParams {
            a: 0.04,
            b: 0.2,
            rho: -0.3,
            m: 0.01,
            sigma: 0.1,
        };
        let ks: Vec<f64> = (-10..=10).map(|i| i as f64 * 0.05).collect();
        let points = make_points(&true_params, &ks);
        let fitted = svi_calibrate(&points).unwrap();

        assert!((fitted.a - true_params.a).abs() < 1e-4, "a: {} vs {}", fitted.a, true_params.a);
        assert!((fitted.b - true_params.b).abs() < 1e-4, "b: {} vs {}", fitted.b, true_params.b);
        assert!((fitted.rho - true_params.rho).abs() < 1e-4, "rho: {} vs {}", fitted.rho, true_params.rho);
        assert!((fitted.m - true_params.m).abs() < 1e-4, "m: {} vs {}", fitted.m, true_params.m);
        assert!((fitted.sigma - true_params.sigma).abs() < 1e-4, "sigma: {} vs {}", fitted.sigma, true_params.sigma);
    }

    // 4 points isn't enough to fit 5 parameters
    #[test]
    fn rejects_too_few_points() {
        let points = vec![
            SviPoint { k: 0.0, w: 0.04 },
            SviPoint { k: 0.1, w: 0.05 },
        ];
        assert!(svi_calibrate(&points).is_err());
    }
}
