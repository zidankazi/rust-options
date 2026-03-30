use pricer::{OptionType, PricingResult};
use serde::{Deserialize, Serialize};

// One option contract from the market with computed Greeks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionChainEntry {
    pub contract_symbol: String,
    pub option_type: OptionType,
    pub strike: f64,
    pub expiration: i64,
    pub time_to_expiry: f64,
    pub last_price: f64,
    pub bid: f64,
    pub ask: f64,
    pub mid_price: f64,
    pub volume: u64,
    pub open_interest: u64,
    pub in_the_money: bool,
    pub implied_volatility: Option<f64>,
    pub greeks: Option<PricingResult>,
}

// Full option chain for a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionChainData {
    pub symbol: String,
    pub spot_price: f64,
    pub risk_free_rate: f64,
    pub expiration_dates: Vec<i64>,
    pub entries: Vec<OptionChainEntry>,
}

// Intraday sparkline data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparklineData {
    pub symbol: String,
    pub prices: Vec<f64>,
}

// --- Yahoo Finance JSON deserialization structs ---
// These match Yahoo's response format exactly.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct YahooResponse {
    pub option_chain: YahooOptionChain,
}

#[derive(Deserialize)]
pub(crate) struct YahooOptionChain {
    pub result: Vec<YahooResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct YahooResult {
    pub underlying_symbol: String,
    pub expiration_dates: Vec<i64>,
    pub quote: YahooQuote,
    pub options: Vec<YahooOptions>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct YahooQuote {
    pub regular_market_price: Option<f64>,
    pub regular_market_change: Option<f64>,
    pub regular_market_change_percent: Option<f64>,
    pub short_name: Option<String>,
    pub symbol: Option<String>,
}

// Cleaned-up quote for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockQuote {
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct YahooOptions {
    pub calls: Vec<YahooContract>,
    pub puts: Vec<YahooContract>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct YahooContract {
    pub contract_symbol: Option<String>,
    pub strike: Option<f64>,
    pub last_price: Option<f64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub volume: Option<u64>,
    pub open_interest: Option<u64>,
    pub implied_volatility: Option<f64>,
    pub in_the_money: Option<bool>,
    pub expiration: Option<i64>,
}
