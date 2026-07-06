use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use redis::AsyncCommands;
use serde::Serialize;
use shared::AuthUser;
use uuid::Uuid;

use crate::rga::{RgaChar, RgaDocument};
use crate::state::AppState;

#[derive(Serialize)]
pub struct DocumentStateResponse {
    pub chars: Vec<RgaChar>,
}

fn doc_key(doc_id: Uuid) -> String {
    format!("doc:{doc_id}")
}

pub async fn get_document_state(
    State(mut state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(doc_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let raw: Option<String> = state
        .redis
        .get(doc_key(doc_id))
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Redis GET failed".to_string()))?;

    let doc: RgaDocument = match raw {
        Some(json) => serde_json::from_str(&json)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to deserialize document".to_string()))?,
        None => RgaDocument::new(),
    };

    Ok((StatusCode::OK, Json(DocumentStateResponse { chars: doc.chars })))
}
