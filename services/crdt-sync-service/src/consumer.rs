use futures_util::StreamExt;
use lapin::{options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions}, BasicProperties, Channel};
use redis::AsyncCommands;
use uuid::Uuid;

use shared::rabbitmq::{
    CrdtOperationMessage, CrdtResultMessage, CRDT_OPERATIONS_QUEUE, CRDT_RESULTS_QUEUE,
};

use crate::rga::{ClientOperation, RgaDocument};

pub async fn run_operations_consumer(
    consume_channel: Channel,
    publish_channel: Channel,
    mut redis: redis::aio::ConnectionManager,
) {
    let mut consumer = consume_channel
        .basic_consume(
            CRDT_OPERATIONS_QUEUE,
            "crdt-sync-consumer",
            BasicConsumeOptions::default(),
            Default::default(),
        )
        .await
        .expect("Failed to start crdt.operations consumer");

    println!("CRDT consumer listening on queue '{CRDT_OPERATIONS_QUEUE}'");

    while let Some(delivery) = consumer.next().await {
        let delivery = match delivery {
            Ok(d) => d,
            Err(e) => {
                eprintln!("RabbitMQ delivery error: {e}");
                continue;
            }
        };

        let msg: CrdtOperationMessage = match serde_json::from_slice(&delivery.data) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to deserialize CrdtOperationMessage: {e}");
                let _ = delivery.ack(BasicAckOptions::default()).await;
                continue;
            }
        };

        match apply_and_persist(&mut redis, &msg).await {
            Ok((resolved_op, document_text)) => {
                let result_msg = CrdtResultMessage {
                    project_id: msg.project_id,
                    from_user_id: msg.user_id,
                    from_email: msg.email,
                    resolved_operation: serde_json::to_value(&resolved_op).unwrap_or_default(),
                    document_text,
                };

                if let Ok(payload) = serde_json::to_vec(&result_msg) {
                    let publish_result = publish_channel
                        .basic_publish(
                            "",
                            CRDT_RESULTS_QUEUE,
                            BasicPublishOptions::default(),
                            &payload,
                            BasicProperties::default(),
                        )
                        .await;

                    if let Err(e) = publish_result {
                        eprintln!("Failed to publish to crdt.results: {e}");
                    }
                }
            }
            Err(e) => eprintln!("Failed to apply CRDT operation for project {}: {e}", msg.project_id),
        }

        let _ = delivery.ack(BasicAckOptions::default()).await;
    }
}

async fn apply_and_persist(
    redis: &mut redis::aio::ConnectionManager,
    msg: &CrdtOperationMessage,
) -> Result<(crate::rga::ResolvedOperation, String), String> {
    let key = doc_key(msg.project_id);

    let raw: Option<String> = redis
        .get(&key)
        .await
        .map_err(|e| format!("Redis GET failed: {e}"))?;

    let mut doc: RgaDocument = match raw {
        Some(json) => serde_json::from_str(&json).map_err(|e| format!("Deserialize failed: {e}"))?,
        None => RgaDocument::new(),
    };

    let client_op: ClientOperation = serde_json::from_value(msg.operation.clone())
        .map_err(|e| format!("Invalid ClientOperation: {e}"))?;

    let resolved = doc
        .apply(client_op, msg.user_id)
        .map_err(|e| format!("RGA apply error: {e}"))?;

    let text = doc.text();

    let serialized = serde_json::to_string(&doc).map_err(|e| format!("Serialize failed: {e}"))?;
    redis
        .set::<_, _, ()>(&key, serialized)
        .await
        .map_err(|e| format!("Redis SET failed: {e}"))?;

    Ok((resolved, text))
}

fn doc_key(doc_id: Uuid) -> String {
    format!("doc:{doc_id}")
}
