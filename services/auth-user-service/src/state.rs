use sqlx::PgPool;

pub struct AppState {
    pub db: PgPool,
    pub jwt_secret: String,
}
