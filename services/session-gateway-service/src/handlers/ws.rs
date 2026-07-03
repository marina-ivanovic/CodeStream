use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::state::AppState;

/// Query parameters required to initiate a WebSocket connection.
/// The JWT is passed here because the browser WebSocket API does not
/// support custom HTTP headers during the upgrade handshake.
#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

/// Thin wrapper placed around every outgoing broadcast message so the
/// frontend can display which collaborator produced each operation.
#[derive(Serialize)]
struct BroadcastEnvelope<'a> {
    from: Uuid,
    data: &'a serde_json::Value,
}

/// HTTP upgrade handler: authenticates the caller, checks project access,
/// then hands the socket off to `handle_socket`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(project_id): Path<Uuid>,
    Query(params): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate JWT locally using the shared secret — no DB call needed.
    let claims = shared::auth::verify_jwt(&params.token, &state.jwt_secret)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string()))?;

    // Verify that the authenticated user actually has access to this project
    // by calling the auth-user-service REST API.
    check_project_access(&state, &params.token, project_id).await?;

    let user_id = claims.sub;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, project_id, user_id, state)))
}

/// Verifies project access by calling the auth service.
/// Returns Ok if the user has at least read access, Err otherwise.
async fn check_project_access(
    state: &AppState,
    token: &str,
    project_id: Uuid,
) -> Result<(), (StatusCode, String)> {
    let url = format!("{}/projects/{}", state.auth_service_url, project_id);

    let response = state
        .http_client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Auth service unreachable".to_string()))?;

    match response.status() {
        s if s.is_success() => Ok(()),
        reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::NOT_FOUND => {
            Err((StatusCode::FORBIDDEN, "No access to this project".to_string()))
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            Err((StatusCode::UNAUTHORIZED, "Invalid token".to_string()))
        }
        _ => Err((StatusCode::SERVICE_UNAVAILABLE, "Auth service error".to_string())),
    }
}

/// Runs for the lifetime of a single WebSocket connection.
/// Uses tokio::select! to multiplex:
///   - messages arriving FROM this client → published to the room broadcast channel
///   - messages arriving FROM the broadcast channel → forwarded to this client
///
/// When crdt-sync-service is integrated (task #3 / #5), the "forward to broadcast"
/// step will be replaced by "publish to RabbitMQ → receive resolved op → broadcast".
async fn handle_socket(
    mut socket: WebSocket,
    project_id: Uuid,
    user_id: Uuid,
    state: Arc<AppState>,
) {
    let tx = state.get_or_create_room(project_id).await;
    let mut rx = tx.subscribe();

    loop {
        tokio::select! {
            // Inbound: message from this client.
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        // Parse the raw text as JSON so we can re-wrap it with
                        // the sender's identity before broadcasting.
                        match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(payload) => {
                                let envelope = BroadcastEnvelope { from: user_id, data: &payload };
                                if let Ok(serialized) = serde_json::to_string(&envelope) {
                                    // Ignore send errors — they only mean no receivers yet.
                                    let _ = tx.send(serialized);
                                }
                            }
                            Err(_) => {
                                // Malformed JSON from client — silently drop.
                            }
                        }
                    }
                    // Client closed the connection or sent an unrecoverable error.
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    // Ping/Pong/Binary frames — not used in this protocol.
                    _ => {}
                }
            }

            // Outbound: message from another client via broadcast channel.
            outgoing = rx.recv() => {
                match outgoing {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    // Channel closed (all senders dropped) — should not happen normally.
                    Err(broadcast::error::RecvError::Closed) => break,
                    // This receiver fell too far behind; skip missed messages and continue.
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        }
    }

    // Drop the receiver before checking count so the count is accurate.
    drop(rx);
    state.cleanup_room_if_empty(project_id).await;
}
