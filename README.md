# rust-options

> A blazing-fast equity derivatives pricing engine written in Rust. Sub-microsecond Black-Scholes, 236x-optimized Monte Carlo, analytical Greeks, and binomial trees for American options — all from scratch with zero external math dependencies.

## Benchmarks

| Benchmark | Time | Throughput |
|---|---|---|
| Black-Scholes + all Greeks | **~19ns** | ~53M prices/sec |
| Implied vol (Newton-Raphson) | **~50ns** | ~20M solves/sec |
| Monte Carlo 100K paths | **~3.5ms** | ~29M paths/sec |
| Binomial tree 200 steps | **~11μs** | ~90K trees/sec |
| Vol surface (500 BS calls) | **~10μs** | 50K surfaces/sec |

Run benchmarks:
```bash
cargo bench -p pricer
```

### Optimization Journey

The Monte Carlo engine went through several rounds of optimization, each targeting a specific bottleneck:

| Optimization | MC Time | Speedup | What changed |
|---|---|---|---|
| Baseline | 826 ms | — | Naive implementation: step-by-step GBM, Box-Muller, wasted normals |
| Reuse spare normals + log-space accumulation | 224 ms | 3.7x | Use both Box-Muller outputs; accumulate sums, one `exp()` per path instead of 252 |
| Marsaglia polar method | 187 ms | 4.4x | Replace `cos()`/`sin()` with rejection sampling |
| Closed-form GBM | **3.5 ms** | **236x** | European options don't need path stepping — one normal per path, no inner loop |

**Total: 826ms → 3.5ms (236x faster).** Same math, same accuracy, same 100K paths with antithetic variates. Every optimization was verified against Black-Scholes analytical prices.

### Why Rust?

The same Black-Scholes pricer in Python (using NumPy/SciPy) takes ~10-50μs per call. This Rust implementation runs in **~19ns** — roughly **500-2500x faster**.

| Operation | Python (NumPy) | Rust | Speedup |
|---|---|---|---|
| Single BS price + Greeks | ~10-50μs | ~19ns | ~500-2500x |
| IV solve (Newton-Raphson) | ~100-500μs | ~50ns | ~2000-10000x |
| 500-call vol surface | ~5-25ms | ~10μs | ~500-2500x |

Python pricers are teaching tools. This is a production-grade engine. The difference matters when you need to price thousands of options in real time — for live trading, risk dashboards, or strategy backtesting.

```python
# Python equivalent — same math, ~2000x slower
from scipy.stats import norm
import numpy as np

def black_scholes(S, K, T, r, sigma):
    d1 = (np.log(S/K) + (r + 0.5*sigma**2)*T) / (sigma*np.sqrt(T))
    d2 = d1 - sigma*np.sqrt(T)
    return S*norm.cdf(d1) - K*np.exp(-r*T)*norm.cdf(d2)
```

## Architecture

```
rust-options/
├── crates/
│   ├── pricer/        ← options pricing math (BS, MC, binomial, IV)
│   ├── market-data/   ← real-time & historical data ingestion
│   ├── risk/          ← portfolio risk analytics (VaR, stress testing)
│   ├── strategy/      ← trade strategies & backtesting
│   └── web/           ← Axum REST API & WASM target
```

## Features

### pricer
- [x] Project scaffolding & types
- [x] Normal CDF/PDF (Abramowitz & Stegun)
- [x] xorshift64 PRNG + Marsaglia polar method
- [x] Black-Scholes pricing (European, Merton extension)
- [x] Analytical Greeks (delta, gamma, theta, vega, rho)
- [x] Implied volatility solver (Newton-Raphson + bisection)
- [x] Monte Carlo engine (GBM, antithetic variates)
- [x] Binomial tree (CRR, American options)
- [x] Benchmarks
- [x] Full test suite

### market-data
- [ ] Not started

### risk
- [ ] Not started

### strategy
- [ ] Not started

### web
- [ ] Not started

## Getting Started

```bash
# Build everything
cargo build

# Run tests
cargo test -p pricer

# Run benchmarks
cargo bench -p pricer
```

## Design Principles

- **Speed is a feature.** Sub-microsecond pricing, zero allocations in hot paths.
- **No unnecessary dependencies.** Normal CDF, PRNG, and RNG implemented from scratch for WASM portability.
- **Pure math in the pricer.** No IO, no async, no side effects. Inputs in, prices out.
- **Correct first, then fast.** Validated against known analytical results before optimizing.
