mod consumer;
mod handlers;
mod state;

use axum::{routing::get, Router};
use lapin::{options::QueueDeclareOptions, Connection, ConnectionProperties};
use shared::rabbitmq::{CRDT_OPERATIONS_QUEUE, CRDT_RESULTS_QUEUE};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use handlers::ws::ws_handler;
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let jwt_secret       = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let auth_service_url = std::env::var("AUTH_SERVICE_URL").expect("AUTH_SERVICE_URL must be set");
    let amqp_url         = std::env::var("AMQP_URL").expect("AMQP_URL must be set");

    let rabbit_conn = Connection::connect(&amqp_url, ConnectionProperties::default())
        .await
        .expect("Failed to connect to RabbitMQ");

    let publish_channel = rabbit_conn.create_channel().await.expect("Failed to create publish channel");
    let consume_channel = rabbit_conn.create_channel().await.expect("Failed to create consume channel");

    publish_channel
        .queue_declare(CRDT_OPERATIONS_QUEUE, QueueDeclareOptions::default(), Default::default())
        .await
        .expect("Failed to declare crdt.operations queue");
    consume_channel
        .queue_declare(CRDT_RESULTS_QUEUE, QueueDeclareOptions::default(), Default::default())
        .await
        .expect("Failed to declare crdt.results queue");

    let shared_state = Arc::new(AppState::new(jwt_secret, auth_service_url, publish_channel));

    tokio::spawn(consumer::run_results_consumer(consume_channel, Arc::clone(&shared_state)));

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws/:project_id", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(shared_state);

    let addr = "0.0.0.0:3001";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Session gateway running on {addr}");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "Session Gateway is healthy!"
}
