use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use shared::AuthUser;
use uuid::Uuid;

use crate::rga::{ClientOperation, ResolvedOperation, RgaDocument};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ApplyRequest {
    pub operation: ClientOperation,
}

#[derive(Serialize)]
pub struct ApplyResponse {
    pub resolved_operation: ResolvedOperation,
    pub document_text: String,
}

#[derive(Serialize)]
pub struct DocumentResponse {
    pub document_text: String,
    pub char_count: usize,
}

fn doc_key(doc_id: Uuid) -> String {
    format!("doc:{doc_id}")
}

async fn load_document(
    redis: &mut redis::aio::ConnectionManager,
    doc_id: Uuid,
) -> Result<RgaDocument, (StatusCode, String)> {
    let data: Option<String> = redis
        .get(doc_key(doc_id))
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Redis GET failed".to_string()))?;

    match data {
        Some(json) => serde_json::from_str(&json)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to deserialize document".to_string())),
        None => Ok(RgaDocument::new()),
    }
}

async fn save_document(
    redis: &mut redis::aio::ConnectionManager,
    doc_id: Uuid,
    doc: &RgaDocument,
) -> Result<(), (StatusCode, String)> {
    let json = serde_json::to_string(doc)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to serialize document".to_string()))?;

    redis
        .set::<_, _, ()>(doc_key(doc_id), json)
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Redis SET failed".to_string()))
}

pub async fn apply_operation(
    State(mut state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(doc_id): Path<Uuid>,
    Json(payload): Json<ApplyRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut doc = load_document(&mut state.redis, doc_id).await?;

    let resolved = doc
        .apply(payload.operation, claims.sub)
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e))?;

    save_document(&mut state.redis, doc_id, &doc).await?;

    Ok((
        StatusCode::OK,
        Json(ApplyResponse {
            resolved_operation: resolved,
            document_text: doc.text(),
        }),
    ))
}

pub async fn get_document(
    State(mut state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(doc_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let doc = load_document(&mut state.redis, doc_id).await?;
    let text = doc.text();
    let char_count = text.chars().count();

    Ok((
        StatusCode::OK,
        Json(DocumentResponse {
            document_text: text,
            char_count,
        }),
    ))
}
