use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pricer::{ExerciseStyle, MonteCarloConfig, OptionContract, OptionType};

/// Standard test contract: ATM call, 1 year, 20% vol
fn test_contract() -> OptionContract {
    OptionContract {
        s: 100.0,
        k: 100.0,
        t: 1.0,
        r: 0.05,
        sigma: 0.2,
        option_type: OptionType::Call,
        exercise_style: ExerciseStyle::European,
        q: None,
    }
}

fn bench_black_scholes(c: &mut Criterion) {
    let contract = test_contract();
    c.bench_function("black_scholes_full", |b| {
        b.iter(|| {
            // TODO: call black_scholes(black_box(contract))
            black_box(contract);
        })
    });
}

fn bench_implied_volatility(c: &mut Criterion) {
    let contract = test_contract();
    let _market_price = 10.4506; // approximate BS price for this contract
    c.bench_function("implied_volatility", |b| {
        b.iter(|| {
            // TODO: call implied_volatility(black_box(market_price), black_box(contract))
            black_box(contract);
        })
    });
}

fn bench_monte_carlo(c: &mut Criterion) {
    let contract = test_contract();
    let config = MonteCarloConfig {
        num_paths: 100_000,
        num_steps: 252,
        seed: Some(42),
    };
    c.bench_function("monte_carlo_100k", |b| {
        b.iter(|| {
            // TODO: call monte_carlo_price(black_box(contract), black_box(config))
            black_box((contract, config));
        })
    });
}

fn bench_binomial(c: &mut Criterion) {
    let contract = test_contract();
    c.bench_function("binomial_200_steps", |b| {
        b.iter(|| {
            // TODO: call binomial_price(black_box(contract), 200)
            black_box(contract);
        })
    });
}

fn bench_vol_surface(c: &mut Criterion) {
    // 500 BS calls across a strike/expiry grid
    let strikes: Vec<f64> = (80..=120).map(|k| k as f64).collect(); // 41 strikes
    let expiries = [0.08, 0.17, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 5.0]; // 12 expiries

    c.bench_function("vol_surface_500_calls", |b| {
        b.iter(|| {
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
                    // TODO: call black_scholes(black_box(contract))
                    black_box(contract);
                }
            }
        })
    });
}

criterion_group!(
    benches,
    bench_black_scholes,
    bench_implied_volatility,
    bench_monte_carlo,
    bench_binomial,
    bench_vol_surface,
);
criterion_main!(benches);
