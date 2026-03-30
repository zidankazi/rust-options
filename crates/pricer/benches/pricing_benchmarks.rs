// Benchmark suite for measuring the execution speed of all pricing models.
// Uses Criterion.rs to run statistical analysis and measure time down to the nanosecond.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pricer::{ExerciseStyle, MonteCarloConfig, OptionContract, OptionType};

/// Standard test contract: ATM call, 1 year, 20% vol
fn test_contract() -> OptionContract {
    OptionContract {
        s: 100.0, // initial stock price
        k: 100.0, // strike price
        t: 1.0,   // time to expiration (1 year)
        r: 0.05,  // 5% risk-free rate
        sigma: 0.2, // 20% annualized volatility
        option_type: OptionType::Call,
        exercise_style: ExerciseStyle::European,
        q: None,
    }
}

// Benchmarks the blazing-fast analytical Black-Scholes formula.
// Because it's a closed-form math equation without loops, it should be the fastest pricer.
fn bench_black_scholes(c: &mut Criterion) {
    let contract = test_contract();
    c.bench_function("black_scholes_full", |b| {
        // b.iter is the tight loop where criterion measures execution time
        // black_box hides the variable from the compiler's optimizer so it actually performs the calculation
        b.iter(|| pricer::black_scholes::black_scholes(black_box(&contract)))
    });
}

// Benchmarks the Newton-Raphson implied volatility solver.
// Runs the Black-Scholes formula repeatedly until it homes in on the target price.
fn bench_implied_volatility(c: &mut Criterion) {
    let contract = test_contract();
    let market_price = 10.4506; // Pre-calculated target price
    c.bench_function("implied_volatility", |b| {
        b.iter(|| {
            pricer::implied_volatility::implied_volatility(
                black_box(market_price),
                black_box(&contract),
            )
        })
    });
}

// Benchmarks the Monte Carlo pricing engine.
// Expected to be the slowest model since it simulates hundreds of thousands of random paths.
fn bench_monte_carlo(c: &mut Criterion) {
    let contract = test_contract();
    let config = MonteCarloConfig {
        num_paths: 100_000, // Number of simulated futures
        num_steps: 252,     // Number of trading days per future
        seed: Some(42),     // Fixed seed for deterministic benchmarking
    };
    c.bench_function("monte_carlo_100k", |b| {
        b.iter(|| {
            pricer::monte_carlo::monte_carlo_price(black_box(&contract), black_box(&config))
        })
    });
}

// Benchmarks the Binomial Tree pricer.
// Execution time scales quadratically with the number of steps (O(N^2)).
fn bench_binomial(c: &mut Criterion) {
    let contract = test_contract();
    c.bench_function("binomial_200_steps", |b| {
        b.iter(|| pricer::binomial::binomial_price(black_box(&contract), black_box(200)))
    });
}

// Benchmarks computing a full volatility surface grid in one go.
// Helpful for understanding worst-case performance when rendering a UI or graph.
fn bench_vol_surface(c: &mut Criterion) {
    // 500 BS calls across a strike/expiry grid
    let strikes: Vec<f64> = (80..=120).map(|k| k as f64).collect(); // 41 strikes
    let expiries = [0.08, 0.17, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 5.0]; // 12 expiries

    c.bench_function("vol_surface_500_calls", |b| {
        b.iter(|| {
            // Calculate a Black-Scholes price for every combination of strike and expiry
            for &k in &strikes {
                for &t in &expiries {
                    let contract = OptionContract {
                        s: 100.0,
                        k,
                        t,
                        r: 0.05,
                        sigma: 0.2,
                        option_type: OptionType::Call,
                        exercise_style: ExerciseStyle::European,
                        q: None,
                    };
                    black_box(pricer::black_scholes::black_scholes(black_box(&contract)));
                }
            }
        })
    });
}

// Bundles all individual benchmarks so they can be run together
criterion_group!(
    benches,
    bench_black_scholes,
    bench_implied_volatility,
    bench_monte_carlo,
    bench_binomial,
    bench_vol_surface,
);
// Generates the main() function required to run the `cargo bench` executable
criterion_main!(benches);
