use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use shared::AuthUser;
use std::sync::Arc;

use crate::sandbox::{run_in_sandbox, SandboxError};
use crate::state::AppState;

const MAX_CODE_BYTES: usize = 64 * 1024; // 64 KB
const DEFAULT_TIMEOUT_SECS: u64 = 10;
const MAX_TIMEOUT_SECS: u64 = 30;

#[derive(Deserialize)]
pub struct ExecuteRequest {
    pub language: String,
    pub code: String,
    pub timeout_seconds: Option<u64>,
}

#[derive(Serialize)]
pub struct ExecuteResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
    pub execution_time_ms: u128,
}

pub async fn execute_code(
    State(state): State<Arc<AppState>>,
    AuthUser(_claims): AuthUser,
    Json(payload): Json<ExecuteRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if payload.code.len() > MAX_CODE_BYTES {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "Code exceeds 64 KB limit".to_string()));
    }

    let timeout = payload
        .timeout_seconds
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .min(MAX_TIMEOUT_SECS);

    match run_in_sandbox(&state.docker, &payload.language, &payload.code, timeout).await {
        Ok(result) => Ok((
            StatusCode::OK,
            Json(ExecuteResponse {
                stdout: result.stdout,
                stderr: result.stderr,
                exit_code: result.exit_code,
                execution_time_ms: result.execution_time_ms,
            }),
        )),
        Err(SandboxError::UnsupportedLanguage(lang)) => Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported language: {lang}. Supported: python, javascript"),
        )),
        Err(SandboxError::Timeout(secs)) => Err((
            StatusCode::REQUEST_TIMEOUT,
            format!("Execution timed out after {secs}s"),
        )),
        Err(SandboxError::DockerError(msg)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Execution failed: {msg}"),
        )),
    }
}
