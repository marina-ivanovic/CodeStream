mod consumer;
mod handlers;
mod rga;
mod state;

use axum::{routing::get, Router};
use lapin::{options::QueueDeclareOptions, Connection, ConnectionProperties};
use redis::aio::ConnectionManager;
use shared::rabbitmq::{CRDT_OPERATIONS_QUEUE, CRDT_RESULTS_QUEUE};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use handlers::documents::get_document_state;
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let amqp_url  = std::env::var("AMQP_URL").expect("AMQP_URL must be set");

    let redis_client  = redis::Client::open(redis_url).expect("Invalid Redis URL");
    let redis_manager = ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");

    let rabbit_conn = Connection::connect(&amqp_url, ConnectionProperties::default())
        .await
        .expect("Failed to connect to RabbitMQ");

    let consume_channel = rabbit_conn.create_channel().await.expect("Failed to create consume channel");
    let publish_channel = rabbit_conn.create_channel().await.expect("Failed to create publish channel");

    consume_channel
        .queue_declare(CRDT_OPERATIONS_QUEUE, QueueDeclareOptions::default(), Default::default())
        .await
        .expect("Failed to declare crdt.operations queue");
    publish_channel
        .queue_declare(CRDT_RESULTS_QUEUE, QueueDeclareOptions::default(), Default::default())
        .await
        .expect("Failed to declare crdt.results queue");

    tokio::spawn(consumer::run_operations_consumer(
        consume_channel,
        publish_channel,
        redis_manager.clone(),
    ));

    let shared_state = AppState { redis: redis_manager };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/documents/:doc_id/state", get(get_document_state))
        .layer(CorsLayer::permissive())
        .with_state(shared_state);

    let addr = "0.0.0.0:3002";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("CRDT sync service running on {addr}");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "CRDT Sync Service is healthy!"
}
