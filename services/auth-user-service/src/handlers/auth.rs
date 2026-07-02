use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use shared::AuthUser;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{LoginRequest, LoginResponse, RegisterRequest, UserResponse, UserRow};
use crate::state::AppState;

const TOKEN_TTL_SECONDS: i64 = 60 * 60 * 24; // 24h

pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Hashing error: {e}")))?
        .to_string();

    let user_id = Uuid::new_v4();

    let insert_result = sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&payload.email)
        .bind(&password_hash)
        .execute(&state.db)
        .await;

    match insert_result {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(UserResponse {
                id: user_id,
                email: payload.email,
            }),
        )),
        Err(e) => {
            if let Some(db_err) = e.as_database_error() {
                if db_err.code().as_deref() == Some("23505") {
                    return Err((StatusCode::CONFLICT, "User with this email already exists".to_string()));
                }
            }
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Error occurred while accessing the database".to_string()))
        }
    }
}

pub async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user = sqlx::query_as::<_, UserRow>("SELECT id, email, password_hash FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()))?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid password hash".to_string()))?;

    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()))?;

    let token = shared::auth::create_jwt(user.id, &user.email, &state.jwt_secret, TOKEN_TTL_SECONDS)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error occurred while creating token".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(LoginResponse {
            token,
            user: UserResponse {
                id: user.id,
                email: user.email,
            },
        }),
    ))
}

/// Protected route: proves the `AuthUser` extractor works end-to-end.
pub async fn me(AuthUser(claims): AuthUser) -> impl IntoResponse {
    Json(UserResponse {
        id: claims.sub,
        email: claims.email,
    })
}
