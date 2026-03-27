# rust-options

> A blazing fast equity derivatives pricing engine written in Rust. Sub-microsecond Black-Scholes, massively parallel Monte Carlo, real-time Greeks, and a full trading simulator with strategy backtesting and PnL tracking.

## Benchmarks

| Benchmark | Target | Actual |
|---|---|---|
| Black-Scholes + all Greeks | ~150ns | — |
| Implied vol (Newton-Raphson) | ~800ns | — |
| Monte Carlo 100K paths | ~12ms | — |
| Binomial tree 200 steps | ~50us | — |
| Vol surface (500 BS calls) | ~75us | — |

Run benchmarks:
```bash
cargo bench -p pricer
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
- [ ] Normal CDF/PDF (Abramowitz & Stegun)
- [ ] xorshift64 PRNG + Box-Muller
- [ ] Black-Scholes pricing (European, Merton extension)
- [ ] Analytical Greeks (delta, gamma, theta, vega, rho)
- [ ] Implied volatility solver (Newton-Raphson + bisection)
- [ ] Monte Carlo engine (GBM, antithetic variates)
- [ ] Binomial tree (CRR, American options)
- [ ] Benchmarks
- [ ] Full test suite

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
