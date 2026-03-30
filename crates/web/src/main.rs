use axum::{Router, routing::get};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

mod api;

struct AppState {
    yahoo: Mutex<Option<market_data::YahooClient>>,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        yahoo: Mutex::new(None),
    });

    let app = Router::new()
        .route("/api/expirations", get(api::expirations))
        .route("/api/chain", get(api::chain))
        .route("/api/price", get(api::price))
        .route("/api/quotes", get(api::quotes))
        .route("/api/sparklines", get(api::sparklines))
        .fallback_service(ServeDir::new("crates/web/static"))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{}", port);
    println!("Server running at http://localhost:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
