use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

pub const ROOM_CHANNEL_CAPACITY: usize = 256;

/// Shared state across all WebSocket connections.
/// `rooms` maps project_id → broadcast sender for that room.
/// When a client sends a message it is published to the sender;
/// every subscriber (other connected clients) receives it automatically.
#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<RwLock<HashMap<Uuid, broadcast::Sender<String>>>>,
    pub jwt_secret: String,
    pub auth_service_url: String,
    pub crdt_sync_url: String,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new(jwt_secret: String, auth_service_url: String, crdt_sync_url: String) -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            jwt_secret,
            auth_service_url,
            crdt_sync_url,
            http_client: reqwest::Client::new(),
        }
    }

    /// Returns the broadcast sender for the given room, creating it if needed.
    pub async fn get_or_create_room(&self, project_id: Uuid) -> broadcast::Sender<String> {
        let mut rooms = self.rooms.write().await;
        rooms
            .entry(project_id)
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(ROOM_CHANNEL_CAPACITY);
                tx
            })
            .clone()
    }

    /// Removes a room once no receivers remain (last client disconnected).
    pub async fn cleanup_room_if_empty(&self, project_id: Uuid) {
        let mut rooms = self.rooms.write().await;
        if let Some(tx) = rooms.get(&project_id) {
            if tx.receiver_count() == 0 {
                rooms.remove(&project_id);
            }
        }
    }
}
