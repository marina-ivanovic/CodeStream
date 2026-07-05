mod handlers;
mod rga;
mod state;

use axum::{routing::{get, post}, Router};
use redis::aio::ConnectionManager;
use tokio::net::TcpListener;

use handlers::documents::{apply_operation, get_document};
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

    let redis_client = redis::Client::open(redis_url).expect("Invalid Redis URL");
    let redis_manager = ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");

    let shared_state = AppState { redis: redis_manager };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/documents/:doc_id", get(get_document))
        .route("/documents/:doc_id/apply", post(apply_operation))
        .with_state(shared_state);

    let addr = "0.0.0.0:3002";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("CRDT sync service running on {addr}");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "CRDT Sync Service is healthy!"
}
