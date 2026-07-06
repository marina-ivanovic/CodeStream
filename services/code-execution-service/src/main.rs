mod handlers;
mod sandbox;
mod state;

use axum::{routing::{get, post}, Router};
use bollard::Docker;
use std::sync::Arc;
use tokio::net::TcpListener;

use handlers::execute::execute_code;
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let docker = Docker::connect_with_local_defaults()
        .expect("Failed to connect to Docker daemon — is Docker Desktop running?");

    let shared_state = Arc::new(AppState { docker });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/execute", post(execute_code))
        .with_state(shared_state);

    let addr = "0.0.0.0:3003";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Code execution service running on {addr}");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "Code Execution Service is healthy!"
}
