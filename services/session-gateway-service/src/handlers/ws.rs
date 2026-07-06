use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

#[derive(Serialize)]
struct BroadcastEnvelope {
    from: Uuid,
    from_email: String,
    data: serde_json::Value,
}

#[derive(Serialize)]
struct CrdtApplyRequest {
    operation: serde_json::Value,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(project_id): Path<Uuid>,
    Query(params): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let claims = shared::auth::verify_jwt(&params.token, &state.jwt_secret)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string()))?;

    let role = check_project_access(&state, &params.token, project_id).await?;

    let user_id = claims.sub;
    let email   = claims.email.clone();
    let token   = params.token.clone();

    Ok(ws.on_upgrade(move |socket| {
        handle_socket(socket, project_id, user_id, email, role, token, state)
    }))
}

async fn check_project_access(
    state: &AppState,
    token: &str,
    project_id: Uuid,
) -> Result<String, (StatusCode, String)> {
    let url = format!("{}/projects/{}", state.auth_service_url, project_id);

    let response = state
        .http_client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Auth service unreachable".to_string()))?;

    match response.status() {
        s if s.is_success() => {
            let body: serde_json::Value = response
                .json()
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not parse auth response".to_string()))?;
            let role = body["role"].as_str().unwrap_or("read").to_string();
            Ok(role)
        }
        reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::NOT_FOUND => {
            Err((StatusCode::FORBIDDEN, "No access to this project".to_string()))
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            Err((StatusCode::UNAUTHORIZED, "Invalid token".to_string()))
        }
        _ => Err((StatusCode::SERVICE_UNAVAILABLE, "Auth service error".to_string())),
    }
}

async fn forward_to_crdt_sync(
    state: &AppState,
    token: &str,
    project_id: Uuid,
    operation: serde_json::Value,
) -> Option<serde_json::Value> {
    let url = format!("{}/documents/{}/apply", state.crdt_sync_url, project_id);

    let response = state
        .http_client
        .post(&url)
        .bearer_auth(token)
        .json(&CrdtApplyRequest { operation })
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    response.json::<serde_json::Value>().await.ok()
}

async fn handle_socket(
    mut socket: WebSocket,
    project_id: Uuid,
    user_id: Uuid,
    email: String,
    role: String,
    token: String,
    state: Arc<AppState>,
) {
    let is_writable = role == "owner" || role == "write";
    let tx = state.get_or_create_room(project_id).await;
    let mut rx = tx.subscribe();

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(msg) = serde_json::from_str::<serde_json::Value>(&text) else {
                            continue;
                        };

                        let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");

                        if msg_type == "cursor" {
                            let envelope = BroadcastEnvelope {
                                from: user_id,
                                from_email: email.clone(),
                                data: serde_json::json!({
                                    "cursor_position": msg.get("position")
                                }),
                            };
                            if let Ok(s) = serde_json::to_string(&envelope) {
                                let _ = tx.send(s);
                            }
                        } else if msg_type == "language" {
                            let envelope = BroadcastEnvelope {
                                from: user_id,
                                from_email: email.clone(),
                                data: msg,
                            };
                            if let Ok(s) = serde_json::to_string(&envelope) {
                                let _ = tx.send(s);
                            }

                        } else if msg_type == "disconnect" {
                            let envelope = BroadcastEnvelope {
                                from: user_id,
                                from_email: email.clone(),
                                data: serde_json::json!({ "disconnect": true }),
                            };
                            if let Ok(s) = serde_json::to_string(&envelope) {
                                let _ = tx.send(s);
                            }
                            break;
                        } else if is_writable {
                            if let Some(resolved) =
                                forward_to_crdt_sync(&state, &token, project_id, msg).await
                            {
                                let envelope = BroadcastEnvelope {
                                    from: user_id,
                                    from_email: email.clone(),
                                    data: resolved,
                                };
                                if let Ok(s) = serde_json::to_string(&envelope) {
                                    let _ = tx.send(s);
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => {
                        let envelope = BroadcastEnvelope {
                            from: user_id,
                            from_email: email.clone(),
                            data: serde_json::json!({ "disconnect": true }),
                        };
                        if let Ok(s) = serde_json::to_string(&envelope) {
                            let _ = tx.send(s);
                        }
                        break;
                    }
                    _ => {}
                }
            }

            outgoing = rx.recv() => {
                match outgoing {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        }
    }

    drop(rx);
    state.cleanup_room_if_empty(project_id).await;
}
