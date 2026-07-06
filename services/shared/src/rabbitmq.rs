use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CRDT_OPERATIONS_QUEUE: &str = "crdt.operations";
pub const CRDT_RESULTS_QUEUE: &str = "crdt.results";

#[derive(Serialize, Deserialize)]
pub struct CrdtOperationMessage {
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub operation: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct CrdtResultMessage {
    pub project_id: Uuid,
    pub from_user_id: Uuid,
    pub from_email: String,
    pub resolved_operation: serde_json::Value,
    pub document_text: String,
}
