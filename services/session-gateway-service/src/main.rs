mod handlers;
mod state;

use axum::{routing::get, Router};
use std::sync::Arc;
use tokio::net::TcpListener;

use handlers::ws::ws_handler;
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let auth_service_url = std::env::var("AUTH_SERVICE_URL").expect("AUTH_SERVICE_URL must be set");
    let crdt_sync_url = std::env::var("CRDT_SYNC_URL").expect("CRDT_SYNC_URL must be set");

    let shared_state = Arc::new(AppState::new(jwt_secret, auth_service_url, crdt_sync_url));

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws/:project_id", get(ws_handler))
        .with_state(shared_state);

    let addr = "0.0.0.0:3001";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Session gateway running on {addr}");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "Session Gateway is healthy!"
}
