import type { ExpirationsResponse, ChainResponse, PricingResult, StockQuote, SparklineData } from './types';

export async function fetchExpirations(symbol: string): Promise<ExpirationsResponse> {
  const resp = await fetch(`/api/expirations?symbol=${symbol}`);
  if (!resp.ok) throw new Error(await resp.text());
  return resp.json();
}

export async function fetchChain(symbol: string, expiry: number): Promise<ChainResponse> {
  const resp = await fetch(`/api/chain?symbol=${symbol}&expiry=${expiry}`);
  if (!resp.ok) throw new Error(await resp.text());
  return resp.json();
}

export async function fetchPrice(params: {
  s: number;
  k: number;
  t: number;
  r: number;
  sigma: number;
  type: string;
}): Promise<PricingResult> {
  const qs = new URLSearchParams(
    Object.entries(params).map(([k, v]) => [k, String(v)])
  );
  const resp = await fetch(`/api/price?${qs}`);
  if (!resp.ok) throw new Error(await resp.text());
  return resp.json();
}

export async function fetchQuotes(symbols: string[]): Promise<StockQuote[]> {
  const resp = await fetch(`/api/quotes?symbols=${symbols.join(',')}`);
  if (!resp.ok) throw new Error(await resp.text());
  return resp.json();
}

export async function fetchSparklines(symbols: string[]): Promise<SparklineData[]> {
  const resp = await fetch(`/api/sparklines?symbols=${symbols.join(',')}`);
  if (!resp.ok) throw new Error(await resp.text());
  return resp.json();
}
