use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use market_data::YahooClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

// --- /api/expirations?symbol=AAPL ---

#[derive(Deserialize)]
pub struct SymbolQuery {
    symbol: String,
}

#[derive(Serialize)]
pub struct ExpirationsResponse {
    symbol: String,
    spot_price: f64,
    expirations: Vec<i64>,
}

pub async fn expirations(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SymbolQuery>,
) -> Result<Json<ExpirationsResponse>, (StatusCode, String)> {
    let client = get_client(&state).await?;
    let (spot, exps) = client
        .get_expirations(&params.symbol)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(Json(ExpirationsResponse {
        symbol: params.symbol,
        spot_price: spot,
        expirations: exps,
    }))
}

// --- /api/chain?symbol=AAPL&expiry=1774828800 ---

#[derive(Deserialize)]
pub struct ChainQuery {
    symbol: String,
    expiry: i64,
}

#[derive(Serialize)]
pub struct ChainResponse {
    symbol: String,
    spot_price: f64,
    entries: Vec<market_data::OptionChainEntry>,
}

pub async fn chain(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ChainQuery>,
) -> Result<Json<ChainResponse>, (StatusCode, String)> {
    let client = get_client(&state).await?;

    let (spot, _) = client
        .get_expirations(&params.symbol)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let r = 0.0425;
    let entries = client
        .get_chain_for_expiry(&params.symbol, params.expiry, spot, r)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(Json(ChainResponse {
        symbol: params.symbol,
        spot_price: spot,
        entries,
    }))
}

// --- /api/price?s=100&k=100&t=1&r=0.05&sigma=0.2&type=call ---

#[derive(Deserialize)]
pub struct PriceQuery {
    s: f64,
    k: f64,
    t: f64,
    r: f64,
    sigma: f64,
    #[serde(rename = "type")]
    option_type: String,
}

pub async fn price(
    Query(params): Query<PriceQuery>,
) -> Result<Json<pricer::PricingResult>, (StatusCode, String)> {
    let option_type = match params.option_type.to_lowercase().as_str() {
        "call" => pricer::OptionType::Call,
        "put" => pricer::OptionType::Put,
        _ => return Err((StatusCode::BAD_REQUEST, "type must be 'call' or 'put'".into())),
    };

    let contract = pricer::OptionContract {
        s: params.s,
        k: params.k,
        t: params.t,
        r: params.r,
        sigma: params.sigma,
        option_type,
        exercise_style: pricer::ExerciseStyle::European,
        q: None,
    };

    pricer::black_scholes::black_scholes(&contract)
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

// --- /api/benchmark?n=1000000 ---

#[derive(Deserialize)]
pub struct BenchmarkQuery {
    n: Option<usize>,
}

#[derive(Serialize)]
pub struct BenchmarkResult {
    iterations: usize,
    total_ns: u64,
    per_call_ns: f64,
    calls_per_second: f64,
    python_estimate_ms: f64,
}

pub async fn benchmark(
    Query(params): Query<BenchmarkQuery>,
) -> Json<BenchmarkResult> {
    let n = params.n.unwrap_or(1_000_000).min(10_000_000);

    let contract = pricer::OptionContract {
        s: 100.0,
        k: 100.0,
        t: 1.0,
        r: 0.05,
        sigma: 0.2,
        option_type: pricer::OptionType::Call,
        exercise_style: pricer::ExerciseStyle::European,
        q: None,
    };

    let start = std::time::Instant::now();
    for _ in 0..n {
        let _ = std::hint::black_box(
            pricer::black_scholes::black_scholes(std::hint::black_box(&contract))
        );
    }
    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos() as u64;
    let per_call_ns = total_ns as f64 / n as f64;
    let calls_per_second = 1_000_000_000.0 / per_call_ns;
    // Python scipy.stats.norm BS: ~20μs per call conservatively
    let python_estimate_ms = n as f64 * 20.0 / 1_000.0;

    Json(BenchmarkResult {
        iterations: n,
        total_ns,
        per_call_ns,
        calls_per_second,
        python_estimate_ms,
    })
}

// --- /api/vol-surface?symbol=AAPL ---

#[derive(Serialize)]
pub struct VolSurfacePoint {
    strike: f64,
    expiry_days: f64,
    iv: f64,
}

#[derive(Serialize)]
pub struct VolSurfaceResponse {
    symbol: String,
    spot_price: f64,
    points: Vec<VolSurfacePoint>,
}

pub async fn vol_surface(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SymbolQuery>,
) -> Result<Json<VolSurfaceResponse>, (StatusCode, String)> {
    let client = get_client(&state).await?;
    let (spot, expirations) = client
        .get_expirations(&params.symbol)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Fetch up to 8 near-term expirations in parallel
    let future_exps: Vec<i64> = expirations.into_iter()
        .filter(|&e| e > now + 86400)
        .take(8)
        .collect();

    let r = 0.0425;
    let futures: Vec<_> = future_exps.iter()
        .map(|&exp| {
            let client = client.clone();
            let symbol = params.symbol.clone();
            tokio::spawn(async move {
                client.get_chain_for_expiry(&symbol, exp, spot, r).await
            })
        })
        .collect();

    let mut points = Vec::new();
    for future in futures {
        if let Ok(Ok(entries)) = future.await {
            for entry in entries {
                if let Some(iv) = entry.implied_volatility {
                    // Filter to reasonable IV and near-the-money
                    let pct = (entry.strike - spot).abs() / spot;
                    if iv > 0.01 && iv < 3.0 && pct < 0.30 {
                        points.push(VolSurfacePoint {
                            strike: entry.strike,
                            expiry_days: entry.time_to_expiry * 365.0,
                            iv: iv * 100.0, // as percentage
                        });
                    }
                }
            }
        }
    }

    Ok(Json(VolSurfaceResponse {
        symbol: params.symbol,
        spot_price: spot,
        points,
    }))
}

// --- /api/quotes?symbols=AAPL,MSFT,GOOGL ---

#[derive(Deserialize)]
pub struct QuotesQuery {
    symbols: String,
}

pub async fn quotes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QuotesQuery>,
) -> Result<Json<Vec<market_data::StockQuote>>, (StatusCode, String)> {
    let client = get_client(&state).await?;
    let symbols: Vec<&str> = params.symbols.split(',').map(|s| s.trim()).collect();

    let futures: Vec<_> = symbols.iter()
        .map(|s| {
            let client = client.clone();
            let symbol = s.to_string();
            tokio::spawn(async move { client.get_quote(&symbol).await })
        })
        .collect();

    let mut results = Vec::new();
    for future in futures {
        if let Ok(Ok(quote)) = future.await {
            results.push(quote);
        }
    }

    Ok(Json(results))
}

// --- /api/sparklines?symbols=SPY,QQQ ---

pub async fn sparklines(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QuotesQuery>,
) -> Result<Json<Vec<market_data::SparklineData>>, (StatusCode, String)> {
    let client = get_client(&state).await?;
    let symbols: Vec<&str> = params.symbols.split(',').map(|s| s.trim()).collect();

    let futures: Vec<_> = symbols.iter()
        .map(|s| {
            let client = client.clone();
            let symbol = s.to_string();
            tokio::spawn(async move { client.get_sparkline(&symbol).await })
        })
        .collect();

    let mut results = Vec::new();
    for future in futures {
        if let Ok(Ok(data)) = future.await {
            results.push(data);
        }
    }

    Ok(Json(results))
}

// --- Helper: get or create Yahoo client (created once, reused) ---

async fn get_client(state: &AppState) -> Result<YahooClient, (StatusCode, String)> {
    // Fast path: client already exists
    {
        let guard = state.yahoo.lock().await;
        if let Some(client) = guard.as_ref() {
            return Ok(client.clone());
        }
    }

    // Slow path: create client (only happens once)
    let client = YahooClient::new()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Yahoo connection failed: {}", e)))?;

    let mut guard = state.yahoo.lock().await;
    *guard = Some(client.clone());
    Ok(client)
}
