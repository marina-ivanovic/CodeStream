use futures_util::StreamExt;
use lapin::{options::{BasicAckOptions, BasicConsumeOptions}, Channel};
use std::sync::Arc;

use shared::rabbitmq::{CrdtResultMessage, CRDT_RESULTS_QUEUE};

use crate::state::AppState;

pub async fn run_results_consumer(channel: Channel, state: Arc<AppState>) {
    let mut consumer = channel
        .basic_consume(
            CRDT_RESULTS_QUEUE,
            "gateway-results-consumer",
            BasicConsumeOptions::default(),
            Default::default(),
        )
        .await
        .expect("Failed to start crdt.results consumer");

    println!("Gateway consumer listening on queue '{CRDT_RESULTS_QUEUE}'");

    while let Some(delivery) = consumer.next().await {
        let delivery = match delivery {
            Ok(d) => d,
            Err(e) => {
                eprintln!("RabbitMQ delivery error: {e}");
                continue;
            }
        };

        if let Ok(msg) = serde_json::from_slice::<CrdtResultMessage>(&delivery.data) {
            let json = serde_json::json!({
                "from": msg.from_user_id,
                "from_email": msg.from_email,
                "data": {
                    "resolved_operation": msg.resolved_operation,
                    "document_text": msg.document_text
                }
            })
            .to_string();

            let rooms = state.rooms.read().await;
            if let Some(tx) = rooms.get(&msg.project_id) {
                let _ = tx.send(json);
            }
        }

        let _ = delivery.ack(BasicAckOptions::default()).await;
    }
}
