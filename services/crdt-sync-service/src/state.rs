use redis::aio::ConnectionManager;

/// Shared application state injected into every handler by Axum.
/// `ConnectionManager` wraps a single multiplexed async Redis connection
/// that automatically reconnects — safe to clone and share across tasks.
/// JWT validation is handled by the `shared::AuthUser` extractor, which
/// reads `JWT_SECRET` from the environment directly.
#[derive(Clone)]
pub struct AppState {
    pub redis: ConnectionManager,
}
