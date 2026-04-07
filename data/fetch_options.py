"""
Fetches historical SPY option chain data from Massive (formerly Polygon.io).
Free tier: 5 requests/min, so this runs slow on purpose.

Strategy:
  1. Find quarterly expiry dates (search around 3rd Friday of Mar/Jun/Sep/Dec)
  2. List OTM contracts near the money for each expiry
  3. Fetch daily OHLCV bars for each contract
  4. Fetch SPY underlying daily bars

Resumable — skips contracts that already have a file.

Usage:
    source data/.venv/bin/activate
    python data/fetch_options.py
"""

import os
import json
import time
import datetime
import requests
from pathlib import Path
from collections import Counter
from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parent.parent / ".env")
API_KEY = os.getenv("MASSIVE_API_KEY")
if not API_KEY:
    raise RuntimeError("MASSIVE_API_KEY not found in .env")

RAW_DIR = Path(__file__).resolve().parent / "raw"
RAW_DIR.mkdir(exist_ok=True)
CONTRACTS_DIR = RAW_DIR / "contracts"
CONTRACTS_DIR.mkdir(exist_ok=True)

BASE_URL = "https://api.polygon.io"
TICKER = "SPY"
START_DATE = "2023-04-01"
END_DATE = "2026-04-04"
RATE_LIMIT_DELAY = 13
SPY_MID = 520.0  # rough midpoint for strike filtering

request_count = 0


def api_get(url: str, params: dict | None = None) -> dict | None:
    global request_count
    if params is None:
        params = {}
    if "apiKey" not in params and "apiKey" not in url:
        params["apiKey"] = API_KEY

    while True:
        resp = requests.get(url, params=params)
        request_count += 1

        if resp.status_code == 429:
            print("  rate limited, waiting 60s...")
            time.sleep(60)
            continue

        if resp.status_code != 200:
            print(f"  error {resp.status_code}: {resp.text[:200]}")
            return None

        time.sleep(RATE_LIMIT_DELAY)
        return resp.json()


def fetch_all_pages(url: str, params: dict) -> list:
    all_results = []
    while url:
        data = api_get(url, params)
        if data is None:
            break
        all_results.extend(data.get("results", []))
        next_url = data.get("next_url")
        if next_url:
            url = next_url
            params = {"apiKey": API_KEY}
        else:
            url = None
    return all_results


def sanitize(ticker: str) -> str:
    return ticker.replace(":", "_")


def third_friday(year: int, month: int) -> datetime.date:
    """Find the 3rd Friday of a given month."""
    d = datetime.date(year, month, 1)
    fridays = 0
    while True:
        if d.weekday() == 4:
            fridays += 1
            if fridays == 3:
                return d
        d += datetime.timedelta(days=1)


def quarterly_months() -> list[tuple[int, int]]:
    """Generate (year, month) pairs for quarterly expiries in our date range."""
    months = []
    for year in [2023, 2024, 2025, 2026]:
        for month in [3, 6, 9, 12]:
            tf = third_friday(year, month)
            start = datetime.date.fromisoformat(START_DATE)
            end = datetime.date.fromisoformat(END_DATE)
            if start <= tf <= end:
                months.append((year, month))
    return months


# ── Phase 1: find expiry dates and list contracts ───────────────────────────

def fetch_contract_list() -> list[dict]:
    contracts_file = RAW_DIR / "contracts_list.json"

    if contracts_file.exists():
        with open(contracts_file) as f:
            contracts = json.load(f)
        print(f"Phase 1: loaded {len(contracts)} contracts from cache")
        return contracts

    print("Phase 1: finding quarterly expiries and listing contracts...")
    all_contracts = []
    quarters = quarterly_months()

    for year, month in quarters:
        tf = third_friday(year, month)
        # search a window around the 3rd friday to find the actual expiry
        window_start = (tf - datetime.timedelta(days=3)).isoformat()
        window_end = (tf + datetime.timedelta(days=3)).isoformat()

        print(f"  {year}-{month:02d} (around {tf})...", end=" ")

        # fetch both expired and active contracts to cover the full range
        contracts = fetch_all_pages(
            f"{BASE_URL}/v3/reference/options/contracts",
            {
                "underlying_ticker": TICKER,
                "expiration_date.gte": window_start,
                "expiration_date.lte": window_end,
                "expired": "true",
                "limit": 1000,
                "apiKey": API_KEY,
            },
        )
        active = fetch_all_pages(
            f"{BASE_URL}/v3/reference/options/contracts",
            {
                "underlying_ticker": TICKER,
                "expiration_date.gte": window_start,
                "expiration_date.lte": window_end,
                "expired": "false",
                "limit": 1000,
                "apiKey": API_KEY,
            },
        )
        # merge, dedup by ticker
        seen = {c["ticker"] for c in contracts}
        for c in active:
            if c["ticker"] not in seen:
                contracts.append(c)
                seen.add(c["ticker"])

        if not contracts:
            print("no contracts found")
            continue

        # pick the expiry date with the most contracts (that's the quarterly)
        expiry_counts = Counter(c["expiration_date"] for c in contracts)
        best_expiry = max(expiry_counts, key=expiry_counts.get)

        # filter to that expiry, OTM, near money
        filtered = []
        for c in contracts:
            if c["expiration_date"] != best_expiry:
                continue
            strike = c["strike_price"]
            if strike < SPY_MID * 0.85 or strike > SPY_MID * 1.15:
                continue
            if c["contract_type"] == "call" and strike >= SPY_MID:
                filtered.append(c)
            elif c["contract_type"] == "put" and strike <= SPY_MID:
                filtered.append(c)

        all_contracts.extend(filtered)
        calls = sum(1 for c in filtered if c["contract_type"] == "call")
        puts = sum(1 for c in filtered if c["contract_type"] == "put")
        print(f"expiry={best_expiry}, {len(filtered)} OTM ({calls}C/{puts}P)")

    with open(contracts_file, "w") as f:
        json.dump(all_contracts, f)

    expiries = sorted(set(c["expiration_date"] for c in all_contracts))
    print(f"  total: {len(all_contracts)} contracts across {len(expiries)} expiries")
    return all_contracts


# ── Phase 2: underlying spot prices ─────────────────────────────────────────

def fetch_underlying() -> list[dict]:
    spot_file = RAW_DIR / "spy_daily.json"

    if spot_file.exists():
        with open(spot_file) as f:
            data = json.load(f)
        print(f"Phase 2: loaded {len(data)} SPY daily bars from cache")
        return data

    print("Phase 2: fetching SPY daily bars...")
    data = api_get(
        f"{BASE_URL}/v2/aggs/ticker/{TICKER}/range/1/day/{START_DATE}/{END_DATE}",
        {"adjusted": "true", "sort": "asc", "limit": 5000, "apiKey": API_KEY},
    )

    bars = data.get("results", []) if data else []
    with open(spot_file, "w") as f:
        json.dump(bars, f)

    print(f"  got {len(bars)} daily bars")
    return bars


# ── Phase 3: option contract bars ───────────────────────────────────────────

def fetch_contract_bars(contracts: list[dict]):
    already = sum(
        1 for c in contracts
        if (CONTRACTS_DIR / f"{sanitize(c['ticker'])}.json").exists()
    )
    remaining = len(contracts) - already

    print(f"Phase 3: fetching daily bars for {len(contracts)} contracts")
    print(f"  already fetched: {already}")
    print(f"  remaining: {remaining}")

    if remaining > 0:
        est_min = remaining * RATE_LIMIT_DELAY / 60
        print(f"  estimated time: ~{est_min:.0f} min (~{est_min / 60:.1f} hrs)")
    else:
        print("  all done!")
        return

    fetched = 0
    for contract in contracts:
        ticker = contract["ticker"]
        safe_name = sanitize(ticker)
        outfile = CONTRACTS_DIR / f"{safe_name}.json"

        if outfile.exists():
            continue

        exp = contract["expiration_date"]
        strike = contract["strike_price"]
        ctype = contract["contract_type"]
        print(
            f"  [{already + fetched + 1}/{len(contracts)}] "
            f"{ctype[0].upper()} {strike} exp {exp}...",
            end=" ",
        )

        data = api_get(
            f"{BASE_URL}/v2/aggs/ticker/{ticker}/range/1/day/{START_DATE}/{END_DATE}",
            {"adjusted": "true", "sort": "asc", "limit": 5000, "apiKey": API_KEY},
        )

        bars = data.get("results", []) if data else []
        result = {
            "ticker": ticker,
            "underlying": TICKER,
            "contract_type": ctype,
            "strike_price": strike,
            "expiration_date": exp,
            "bars": bars,
        }

        with open(outfile, "w") as f:
            json.dump(result, f)

        print(f"{len(bars)} bars")
        fetched += 1

    print(f"  done. fetched {fetched} new contracts.")


# ── Main ────────────────────────────────────────────────────────────────────

def main():
    print("=== SPY Options Data Pipeline ===")
    print(f"Date range: {START_DATE} to {END_DATE}")
    print(f"Rate limit: ~{60 // RATE_LIMIT_DELAY} req/min")
    print()

    contracts = fetch_contract_list()
    if not contracts:
        print("No contracts found. Check your API key and date range.")
        return
    print()

    fetch_underlying()
    print()

    fetch_contract_bars(contracts)
    print()

    print(f"=== Pipeline complete. Total API requests: {request_count} ===")
    print(f"Raw data in: {RAW_DIR}")
    print(f"Next step: python data/build_dataset.py")


if __name__ == "__main__":
    main()
