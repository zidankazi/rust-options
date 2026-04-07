"""
Processes raw option chain data into SVI training dataset.

Takes the raw JSON from fetch_options.py and produces a CSV where each row
is one (date, expiry) slice with:
  - market features (spot price, days to expiry, etc.)
  - fitted SVI parameters (a, b, rho, m, sigma) as training labels

Usage:
    source data/.venv/bin/activate
    python data/build_dataset.py
"""

import json
import math
import datetime
import pandas as pd
from pathlib import Path
from scipy.optimize import minimize

DATA_DIR = Path(__file__).resolve().parent
RAW_DIR = DATA_DIR / "raw"
CONTRACTS_DIR = RAW_DIR / "contracts"
OUTPUT_FILE = DATA_DIR / "svi_dataset.csv"

RISK_FREE_RATE = 0.05  # rough average over the period
DIV_YIELD = 0.013      # SPY dividend yield (~1.3%)


# ── Black-Scholes + IV solver (minimal Python versions) ─────────────────────

def norm_cdf(x: float) -> float:
    """Standard normal CDF — Abramowitz & Stegun, same as the Rust version."""
    a1, a2, a3, a4, a5 = 0.254829592, -0.284496736, 1.421413741, -1.453152027, 1.061405429
    p = 0.3275911
    sign = 1.0 if x >= 0 else -1.0
    x = abs(x)
    t = 1.0 / (1.0 + p * x)
    y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * math.exp(-x * x / 2.0)
    return 0.5 * (1.0 + sign * y)


def bs_price(s: float, k: float, t: float, r: float, q: float, sigma: float, is_call: bool) -> float:
    """Black-Scholes price for a European option."""
    if t <= 0 or sigma <= 0:
        return 0.0
    d1 = (math.log(s / k) + (r - q + 0.5 * sigma * sigma) * t) / (sigma * math.sqrt(t))
    d2 = d1 - sigma * math.sqrt(t)
    if is_call:
        return s * math.exp(-q * t) * norm_cdf(d1) - k * math.exp(-r * t) * norm_cdf(d2)
    else:
        return k * math.exp(-r * t) * norm_cdf(-d2) - s * math.exp(-q * t) * norm_cdf(-d1)


def bs_vega(s: float, k: float, t: float, r: float, q: float, sigma: float) -> float:
    """Vega — derivative of BS price with respect to sigma."""
    if t <= 0 or sigma <= 0:
        return 0.0
    d1 = (math.log(s / k) + (r - q + 0.5 * sigma * sigma) * t) / (sigma * math.sqrt(t))
    return s * math.exp(-q * t) * math.exp(-d1 * d1 / 2.0) / math.sqrt(2 * math.pi) * math.sqrt(t)


def implied_vol(market_price: float, s: float, k: float, t: float, r: float, q: float, is_call: bool) -> float | None:
    """Newton-Raphson IV solver with bisection fallback. Returns None if it can't converge."""
    if market_price <= 0 or t <= 0:
        return None

    # newton-raphson
    sigma = 0.2
    for _ in range(100):
        price = bs_price(s, k, t, r, q, sigma, is_call)
        diff = price - market_price
        if abs(diff) < 1e-8:
            return sigma
        vega = bs_vega(s, k, t, r, q, sigma)
        if abs(vega) < 1e-10:
            break
        new_sigma = sigma - diff / vega
        if new_sigma <= 0 or new_sigma > 5.0:
            break
        sigma = new_sigma

    # bisection fallback
    lo, hi = 0.001, 5.0
    for _ in range(100):
        mid = (lo + hi) / 2.0
        price = bs_price(s, k, t, r, q, mid, is_call)
        diff = price - market_price
        if abs(diff) < 1e-8:
            return mid
        if diff > 0:
            hi = mid
        else:
            lo = mid

    return None


# ── SVI calibration (Python version of the Rust code) ──────────────────────

def svi_variance(params: list, k: float) -> float:
    """SVI total variance at log-moneyness k."""
    a, b, rho, m, sigma = params
    x = k - m
    return a + b * (rho * x + math.sqrt(x * x + sigma * sigma))


def svi_calibrate(ks: list[float], ws: list[float]) -> dict | None:
    """Fit SVI params to (log-moneyness, total variance) points.
    Returns dict with {a, b, rho, m, sigma} or None if fitting fails."""
    if len(ks) < 5:
        return None

    def objective(p):
        a, b, rho, m, sigma = p
        # penalty for constraint violations
        if b < 0 or sigma <= 0 or rho <= -1 or rho >= 1:
            return 1e18
        total = 0.0
        for k, w in zip(ks, ws):
            diff = svi_variance(p, k) - w
            total += diff * diff
        return total

    w_mean = sum(ws) / len(ws)
    k_mean = sum(ks) / len(ks)
    x0 = [w_mean, 0.1, -0.3, k_mean, 0.1]

    result = minimize(objective, x0, method="Nelder-Mead",
                      options={"maxiter": 10000, "xatol": 1e-10, "fatol": 1e-12})

    if not result.success and result.fun > 1e10:
        return None

    a, b, rho, m, sigma = result.x
    return {"a": a, "b": b, "rho": rho, "m": m, "sigma": sigma}


# ── Data loading ────────────────────────────────────────────────────────────

def load_spot_prices() -> dict[str, float]:
    """Load SPY daily close prices. Returns {date_str: close_price}."""
    spot_file = RAW_DIR / "spy_daily.json"
    if not spot_file.exists():
        raise FileNotFoundError(f"Run fetch_options.py first: {spot_file}")

    with open(spot_file) as f:
        bars = json.load(f)

    prices = {}
    for bar in bars:
        # timestamp is in milliseconds
        ts = bar["t"] / 1000
        date_str = datetime.date.fromtimestamp(ts).isoformat()
        prices[date_str] = bar["c"]  # close price
    return prices


def load_contracts() -> list[dict]:
    """Load all contract bar files."""
    contracts = []
    for path in sorted(CONTRACTS_DIR.glob("*.json")):
        with open(path) as f:
            data = json.load(f)
        if data.get("bars"):
            contracts.append(data)
    return contracts


# ── Main pipeline ───────────────────────────────────────────────────────────

def main():
    print("=== Building SVI Training Dataset ===")
    print()

    # load data
    print("Loading spot prices...", end=" ")
    spot_prices = load_spot_prices()
    print(f"{len(spot_prices)} days")

    print("Loading contract bars...", end=" ")
    contracts = load_contracts()
    print(f"{len(contracts)} contracts")
    print()

    if not contracts:
        print("No contract data found. Run fetch_options.py first.")
        return

    # build a flat table: one row per (date, contract) observation
    print("Building observations table...")
    observations = []

    for contract in contracts:
        ticker = contract["ticker"]
        strike = contract["strike_price"]
        expiry = contract["expiration_date"]
        is_call = contract["contract_type"] == "call"
        exp_date = datetime.date.fromisoformat(expiry)

        for bar in contract["bars"]:
            ts = bar["t"] / 1000
            date_str = datetime.date.fromtimestamp(ts).isoformat()
            trade_date = datetime.date.fromisoformat(date_str)

            # skip if no spot price for this day
            if date_str not in spot_prices:
                continue

            # skip if past expiry
            if trade_date >= exp_date:
                continue

            spot = spot_prices[date_str]
            opt_close = bar["c"]  # option close price
            volume = bar.get("v", 0)

            # skip low volume — noisy prices
            if volume < 10:
                continue

            # time to expiry in years
            days_to_exp = (exp_date - trade_date).days
            t = days_to_exp / 365.0
            if t <= 0:
                continue

            # compute IV
            iv = implied_vol(opt_close, spot, strike, t, RISK_FREE_RATE, DIV_YIELD, is_call)
            if iv is None or iv < 0.01 or iv > 3.0:
                continue  # bad solve, skip

            # SVI coordinates
            forward = spot * math.exp((RISK_FREE_RATE - DIV_YIELD) * t)
            k = math.log(strike / forward)  # log-moneyness
            w = iv * iv * t                  # total variance

            observations.append({
                "date": date_str,
                "expiry": expiry,
                "spot": spot,
                "strike": strike,
                "is_call": is_call,
                "days_to_exp": days_to_exp,
                "t": t,
                "option_price": opt_close,
                "volume": volume,
                "iv": iv,
                "forward": forward,
                "k": k,
                "w": w,
            })

    print(f"  {len(observations)} valid observations")
    print()

    if not observations:
        print("No valid observations. Check your data.")
        return

    # group by (date, expiry) and calibrate SVI
    print("Calibrating SVI for each (date, expiry) slice...")
    df = pd.DataFrame(observations)
    groups = df.groupby(["date", "expiry"])

    rows = []
    success = 0
    fail = 0

    for (date, expiry), group in groups:
        ks = group["k"].tolist()
        ws = group["w"].tolist()

        # need enough points for a meaningful fit
        if len(ks) < 6:
            fail += 1
            continue

        params = svi_calibrate(ks, ws)
        if params is None:
            fail += 1
            continue

        # compute fit quality — root mean squared error
        residuals = [svi_variance(list(params.values()), k) - w for k, w in zip(ks, ws)]
        rmse = math.sqrt(sum(r * r for r in residuals) / len(residuals))

        # skip bad fits — high error or degenerate parameters
        if rmse > 0.005:
            fail += 1
            continue
        if not (-1.0 < params["rho"] < 1.0):
            fail += 1
            continue
        if params["b"] < 0 or params["b"] > 5.0:
            fail += 1
            continue
        if params["sigma"] <= 0 or params["sigma"] > 2.0:
            fail += 1
            continue
        if abs(params["m"]) > 2.0:
            fail += 1
            continue
        if abs(params["a"]) > 1.0:
            fail += 1
            continue

        rows.append({
            "date": date,
            "expiry": expiry,
            "spot": group["spot"].iloc[0],
            "days_to_exp": group["days_to_exp"].iloc[0],
            "t": group["t"].iloc[0],
            "forward": group["forward"].iloc[0],
            "n_contracts": len(ks),
            "svi_a": params["a"],
            "svi_b": params["b"],
            "svi_rho": params["rho"],
            "svi_m": params["m"],
            "svi_sigma": params["sigma"],
            "fit_rmse": rmse,
        })
        success += 1

    print(f"  calibrated: {success}")
    print(f"  failed/skipped: {fail}")
    print()

    if not rows:
        print("No successful calibrations. Check your data quality.")
        return

    # save
    result_df = pd.DataFrame(rows)
    result_df.to_csv(OUTPUT_FILE, index=False)
    print(f"Saved {len(result_df)} rows to {OUTPUT_FILE}")

    # summary stats
    print()
    print("Dataset summary:")
    print(f"  Date range: {result_df['date'].min()} to {result_df['date'].max()}")
    print(f"  Expiries: {result_df['expiry'].nunique()}")
    print(f"  Total slices: {len(result_df)}")
    print(f"  Avg contracts per slice: {result_df['n_contracts'].mean():.1f}")
    print(f"  Avg fit RMSE: {result_df['fit_rmse'].mean():.6f}")
    print()
    print("SVI parameter ranges:")
    for col in ["svi_a", "svi_b", "svi_rho", "svi_m", "svi_sigma"]:
        print(f"  {col}: [{result_df[col].min():.4f}, {result_df[col].max():.4f}]")


if __name__ == "__main__":
    main()
