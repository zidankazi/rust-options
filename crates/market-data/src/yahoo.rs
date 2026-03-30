// Yahoo Finance client for fetching option chains.
// Handles the cookie/crumb authentication dance automatically.

use crate::error::MarketDataError;
use crate::types::*;
use pricer::{ExerciseStyle, OptionContract, OptionType};
use std::time::{SystemTime, UNIX_EPOCH};

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Clone)]
pub struct YahooClient {
    client: reqwest::Client,
    crumb: String,
}

impl YahooClient {
    // Create a new client, authenticating with Yahoo automatically.
    pub async fn new() -> Result<Self, MarketDataError> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .user_agent(USER_AGENT)
            .build()?;

        // Step 1: hit fc.yahoo.com to get a session cookie (the 404 is expected)
        let _ = client
            .get("https://fc.yahoo.com")
            .send()
            .await;

        // Step 2: use that cookie to get a crumb token
        let crumb = client
            .get("https://query1.finance.yahoo.com/v1/test/getcrumb")
            .send()
            .await?
            .text()
            .await?;

        if crumb.is_empty() || crumb.contains("<!DOCTYPE") {
            return Err(MarketDataError::Auth("Failed to get crumb from Yahoo".into()));
        }

        Ok(Self { client, crumb })
    }

    // Get the list of available expiration dates and spot price for a symbol.
    pub async fn get_expirations(&self, symbol: &str) -> Result<(f64, Vec<i64>), MarketDataError> {
        let url = format!(
            "https://query1.finance.yahoo.com/v7/finance/options/{}?crumb={}",
            symbol, self.crumb
        );

        let resp: YahooResponse = self.client.get(&url).send().await?.json().await?;
        let result = resp.option_chain.result.into_iter().next()
            .ok_or_else(|| MarketDataError::NotFound(symbol.into()))?;

        let spot = result.quote.regular_market_price
            .ok_or_else(|| MarketDataError::Parse("No spot price in response".into()))?;

        Ok((spot, result.expiration_dates))
    }

    // Fetch the option chain for a specific expiration date.
    pub async fn get_chain_for_expiry(
        &self,
        symbol: &str,
        expiry: i64,
        spot: f64,
        risk_free_rate: f64,
    ) -> Result<Vec<OptionChainEntry>, MarketDataError> {
        let url = format!(
            "https://query1.finance.yahoo.com/v7/finance/options/{}?date={}&crumb={}",
            symbol, expiry, self.crumb
        );

        let resp: YahooResponse = self.client.get(&url).send().await?.json().await?;
        let result = resp.option_chain.result.into_iter().next()
            .ok_or_else(|| MarketDataError::NotFound(symbol.into()))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut entries = Vec::new();

        for options in &result.options {
            for contract in &options.calls {
                if let Some(entry) = parse_contract(contract, OptionType::Call, spot, risk_free_rate, now) {
                    entries.push(entry);
                }
            }
            for contract in &options.puts {
                if let Some(entry) = parse_contract(contract, OptionType::Put, spot, risk_free_rate, now) {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    // Fetch a basic quote (price, change, name) for a single symbol.
    pub async fn get_quote(&self, symbol: &str) -> Result<StockQuote, MarketDataError> {
        let url = format!(
            "https://query1.finance.yahoo.com/v7/finance/options/{}?crumb={}",
            symbol, self.crumb
        );

        let resp: YahooResponse = self.client.get(&url).send().await?.json().await?;
        let result = resp.option_chain.result.into_iter().next()
            .ok_or_else(|| MarketDataError::NotFound(symbol.into()))?;

        Ok(StockQuote {
            symbol: result.quote.symbol.unwrap_or_else(|| symbol.to_string()),
            name: result.quote.short_name.unwrap_or_default(),
            price: result.quote.regular_market_price.unwrap_or(0.0),
            change: result.quote.regular_market_change.unwrap_or(0.0),
            change_percent: result.quote.regular_market_change_percent.unwrap_or(0.0),
        })
    }

    // Fetch intraday sparkline data (closing prices at 15-min intervals).
    pub async fn get_sparkline(&self, symbol: &str) -> Result<SparklineData, MarketDataError> {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=1d&interval=15m&crumb={}",
            symbol, self.crumb
        );

        let val: serde_json::Value = self.client.get(&url).send().await?.json().await?;
        let closes = val["chart"]["result"][0]["indicators"]["quote"][0]["close"]
            .as_array()
            .ok_or_else(|| MarketDataError::Parse("No chart data".into()))?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect::<Vec<f64>>();

        Ok(SparklineData {
            symbol: symbol.to_string(),
            prices: closes,
        })
    }

    // Fetch the full option chain across all expirations.
    pub async fn get_full_chain(
        &self,
        symbol: &str,
        risk_free_rate: f64,
    ) -> Result<OptionChainData, MarketDataError> {
        let (spot, expirations) = self.get_expirations(symbol).await?;

        let mut all_entries = Vec::new();

        for &exp in &expirations {
            let entries = self.get_chain_for_expiry(symbol, exp, spot, risk_free_rate).await?;
            all_entries.extend(entries);
        }

        Ok(OptionChainData {
            symbol: symbol.to_string(),
            spot_price: spot,
            risk_free_rate,
            expiration_dates: expirations,
            entries: all_entries,
        })
    }
}

// Parse a Yahoo contract into our OptionChainEntry, computing IV and Greeks.
// Returns None if the contract has missing/invalid data.
fn parse_contract(
    contract: &YahooContract,
    option_type: OptionType,
    spot: f64,
    r: f64,
    now: i64,
) -> Option<OptionChainEntry> {
    let strike = contract.strike?;
    let expiration = contract.expiration?;
    let bid = contract.bid.unwrap_or(0.0);
    let ask = contract.ask.unwrap_or(0.0);
    let last_price = contract.last_price.unwrap_or(0.0);
    let mid_price = if bid > 0.0 && ask > 0.0 { (bid + ask) / 2.0 } else { last_price };

    // Skip contracts with no meaningful price
    if mid_price <= 0.0 {
        return None;
    }

    let t = (expiration - now) as f64 / (365.25 * 24.0 * 3600.0);

    // Skip expired or nearly expired contracts
    if t < 0.001 {
        return None;
    }

    // Build a pricer contract and solve for IV + Greeks
    let pricer_contract = OptionContract {
        s: spot,
        k: strike,
        t,
        r,
        sigma: 0.2, // placeholder, IV solver will find the real value
        option_type,
        exercise_style: ExerciseStyle::European, // BS assumption
        q: None,
    };

    let iv = pricer::implied_volatility::implied_volatility(mid_price, &pricer_contract).ok();

    // If we found IV, compute Greeks at that IV
    let greeks = iv.and_then(|sigma| {
        let mut c = pricer_contract;
        c.sigma = sigma;
        pricer::black_scholes::black_scholes(&c).ok()
    });

    Some(OptionChainEntry {
        contract_symbol: contract.contract_symbol.clone().unwrap_or_default(),
        option_type,
        strike,
        expiration,
        time_to_expiry: t,
        last_price,
        bid,
        ask,
        mid_price,
        volume: contract.volume.unwrap_or(0),
        open_interest: contract.open_interest.unwrap_or(0),
        in_the_money: contract.in_the_money.unwrap_or(false),
        implied_volatility: iv,
        greeks,
    })
}
