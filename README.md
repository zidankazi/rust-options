# rust-options

> A blazing-fast equity derivatives pricing engine written in Rust. Sub-microsecond Black-Scholes, 1,370x-optimized parallel Monte Carlo, analytical Greeks, and binomial trees for American options. All math from scratch with zero external math dependencies.

## Benchmarks

| Benchmark | Python (NumPy/SciPy) | Rust | Speedup |
|---|---|---|---|
| Black-Scholes + all Greeks | ~10-50μs | **~19ns** | **~2,000x** |
| Implied vol (Newton-Raphson) | ~100-500μs | **~50ns** | **~5,000x** |
| Monte Carlo 100K paths | ~5-15s | **~600μs** | **~16,000x** |
| Binomial tree 200 steps | ~1-5ms | **~11μs** | **~200x** |
| Vol surface (500 BS calls) | ~5-25ms | **~10μs** | **~1,500x** |

All benchmarks measured with [Criterion.rs](https://github.com/bheisler/criterion.rs). Python times are typical for equivalent implementations using NumPy, SciPy, and `scipy.stats.norm`. Run them yourself:

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

At 53 million BS prices per second, the math is never the bottleneck. The network is. That's what matters when you're pricing thousands of contracts in real time for trading, risk dashboards, or backtesting.

## Architecture

```
rust-options/
├── crates/
│   ├── pricer/        ← BS, MC, binomial, IV solver, Greeks (pure math, zero deps)
│   ├── market-data/   ← Yahoo Finance client, live option chains + quotes
│   ├── risk/          ← portfolio risk analytics (planned)
│   ├── strategy/      ← trade strategies & backtesting (planned)
│   └── web/
│       ├── src/       ← Axum REST API serving market data + pricer
│       └── frontend/  ← TypeScript + Vite (market overview, chain viewer, pricer)
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
- [x] Yahoo Finance integration (option chains, quotes, sparklines)
- [x] Automatic IV + Greeks computation on live market prices
- [x] Parallel quote fetching

### risk
- [ ] Not started

### strategy
- [ ] Not started

### web
- [x] Axum REST API
- [x] TypeScript frontend (Vite)
- [x] Market overview landing page with sparkline charts
- [x] Live option chain viewer with Greeks
- [x] Black-Scholes pricing calculator with payoff diagram
- [ ] Strategy builder
- [ ] Portfolio tracking
- [ ] Backtesting

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
