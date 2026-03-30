# rust-options

> A blazing-fast equity derivatives pricing engine written in Rust. Sub-microsecond Black-Scholes, 1,370x-optimized parallel Monte Carlo, analytical Greeks, and binomial trees for American options. All math from scratch with zero external math dependencies.

## Benchmarks

| Benchmark | Time | Throughput |
|---|---|---|
| Black-Scholes + all Greeks | **~19ns** | ~53M prices/sec |
| Implied vol (Newton-Raphson) | **~50ns** | ~20M solves/sec |
| Monte Carlo 100K paths | **~600μs** | ~167M paths/sec |
| Binomial tree 200 steps | **~11μs** | ~90K trees/sec |
| Vol surface (500 BS calls) | **~10μs** | 50K surfaces/sec |

Run benchmarks:
```bash
cargo bench -p pricer
```

### Why Rust?

Most options pricing code lives in Python with NumPy and SciPy. That works for learning, but Python pays a cost on every function call, loop iteration, and memory allocation. Rust compiles down to native machine code with no garbage collector, so tight math loops run at hardware speed.

Here's the same Black-Scholes formula in both languages. Same math, same inputs:

```python
# Python: ~10-50μs per call
from scipy.stats import norm
import numpy as np

def black_scholes(S, K, T, r, sigma):
    d1 = (np.log(S/K) + (r + 0.5*sigma**2)*T) / (sigma*np.sqrt(T))
    d2 = d1 - sigma*np.sqrt(T)
    return S*norm.cdf(d1) - K*np.exp(-r*T)*norm.cdf(d2)
```

```rust
// Rust: ~19ns per call (same formula, ~2000x faster)
// Normal CDF/PDF built from scratch using Abramowitz & Stegun approximation.
// No external math libraries, just f64 arithmetic.
pub fn black_scholes(contract: &OptionContract) -> Result<PricingResult, PricerError> {
    let d1 = ((contract.s / contract.k).ln()
        + (r - q + 0.5 * sigma * sigma) * t)
        / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();
    // ... price + all five Greeks in one pass
}
```

| Operation | Python (NumPy/SciPy) | Rust | Speedup |
|---|---|---|---|
| Single BS price + Greeks | ~10-50μs | ~19ns | ~500-2500x |
| IV solve (Newton-Raphson) | ~100-500μs | ~50ns | ~2000-10000x |
| 500-call vol surface | ~5-25ms | ~10μs | ~500-2500x |
| Monte Carlo 100K paths | ~5-15s | ~600μs | ~8000-25000x |

At 53 million BS prices per second, the math is never the bottleneck. The network is. That's what matters when you're pricing thousands of contracts in real time for trading, risk dashboards, or backtesting.

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
