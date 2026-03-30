// Example: fetch an option chain from Yahoo Finance and display IV + Greeks.
// Usage: cargo run -p market-data --example fetch_chain -- AAPL

use market_data::YahooClient;

#[tokio::main]
async fn main() {
    let symbol = std::env::args().nth(1).unwrap_or_else(|| "AAPL".into());
    let r = 0.0425; // ~current risk-free rate (10Y treasury)

    println!("Fetching option chain for {}...\n", symbol);

    let client = match YahooClient::new().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to Yahoo: {}", e);
            return;
        }
    };

    // Get expirations first
    let (spot, expirations) = match client.get_expirations(&symbol).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to fetch expirations: {}", e);
            return;
        }
    };

    println!("Spot price: ${:.2}", spot);
    println!("Available expirations: {}\n", expirations.len());

    // Skip same-day/expired expirations, pick the next valid one
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let one_day = 86400;
    let nearest = *expirations.iter()
        .find(|&&exp| exp > now + one_day)
        .unwrap_or(&expirations[0]);
    let entries = match client.get_chain_for_expiry(&symbol, nearest, spot, r).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to fetch chain: {}", e);
            return;
        }
    };

    println!(
        "{:<8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Type", "Strike", "Bid", "Ask", "IV", "Delta", "Volume"
    );
    println!("{}", "-".repeat(64));

    for entry in &entries {
        let type_str = match entry.option_type {
            pricer::OptionType::Call => "CALL",
            pricer::OptionType::Put => "PUT",
        };
        let iv_str = entry.implied_volatility
            .map(|v| format!("{:.1}%", v * 100.0))
            .unwrap_or_else(|| "N/A".into());
        let delta_str = entry.greeks
            .map(|g| format!("{:.3}", g.delta))
            .unwrap_or_else(|| "N/A".into());

        println!(
            "{:<8} {:>8.2} {:>8.2} {:>8.2} {:>8} {:>8} {:>8}",
            type_str, entry.strike, entry.bid, entry.ask, iv_str, delta_str, entry.volume
        );
    }

    println!("\nTotal contracts: {}", entries.len());
}
