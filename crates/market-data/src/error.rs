use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarketDataError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to parse response: {0}")]
    Parse(String),

    #[error("No data found for symbol: {0}")]
    NotFound(String),

    #[error("Yahoo auth failed: {0}")]
    Auth(String),

    #[error("Pricer error: {0}")]
    Pricer(#[from] pricer::PricerError),
}
