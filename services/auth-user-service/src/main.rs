mod handlers;
mod models;
mod state;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;

use handlers::auth::{login_user, me, register_user};
use handlers::projects::{create_project, delete_project, get_project, grant_access, list_projects};
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET environment variable must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Could not connect to the database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let shared_state = Arc::new(AppState { db: pool, jwt_secret });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/register", post(register_user))
        .route("/login", post(login_user))
        .route("/me", get(me))
        .route("/projects", post(create_project).get(list_projects))
        .route("/projects/{id}", get(get_project).delete(delete_project))
        .route("/projects/{id}/access", post(grant_access))
        .with_state(shared_state);

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Auth servis is running on: {}", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "Auth Service is healthy!"
}
