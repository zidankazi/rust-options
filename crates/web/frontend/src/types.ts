export interface PricingResult {
  price: number;
  delta: number;
  gamma: number;
  theta: number;
  vega: number;
  rho: number;
  implied_volatility: number | null;
}

export interface OptionChainEntry {
  contract_symbol: string;
  option_type: 'Call' | 'Put';
  strike: number;
  expiration: number;
  time_to_expiry: number;
  last_price: number;
  bid: number;
  ask: number;
  mid_price: number;
  volume: number;
  open_interest: number;
  in_the_money: boolean;
  implied_volatility: number | null;
  greeks: PricingResult | null;
}

export interface SparklineData {
  symbol: string;
  prices: number[];
}

export interface StockQuote {
  symbol: string;
  name: string;
  price: number;
  change: number;
  change_percent: number;
}

export interface BenchmarkResult {
  iterations: number;
  total_ns: number;
  per_call_ns: number;
  calls_per_second: number;
  python_estimate_ms: number;
}

export interface VolSurfacePoint {
  strike: number;
  expiry_days: number;
  iv: number;
}

export interface VolSurfaceResponse {
  symbol: string;
  spot_price: number;
  points: VolSurfacePoint[];
}

export interface ExpirationsResponse {
  symbol: string;
  spot_price: number;
  expirations: number[];
}

export interface ChainResponse {
  symbol: string;
  spot_price: number;
  entries: OptionChainEntry[];
}
