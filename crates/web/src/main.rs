use axum::{Router, routing::get, response::Html};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

mod api;

struct AppState {
    yahoo: Mutex<Option<market_data::YahooClient>>,
}

// Serve index.html for any route that isn't /api/* or a static asset.
// This lets the frontend handle routing via the History API.
async fn spa_fallback() -> Html<String> {
    let html = tokio::fs::read_to_string("crates/web/static/index.html")
        .await
        .unwrap_or_else(|_| "<!DOCTYPE html><html><body>Not found</body></html>".into());
    Html(html)
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
        .route("/api/benchmark", get(api::benchmark))
        .route("/api/vol-surface", get(api::vol_surface))
        .route("/api/quotes", get(api::quotes))
        .route("/api/sparklines", get(api::sparklines))
        .nest_service("/assets", ServeDir::new("crates/web/static/assets"))
        .fallback(spa_fallback)
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{}", port);
    println!("Server running at http://localhost:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
