use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use shared::AuthUser;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{AccessResponse, CreateProjectRequest, GrantAccessRequest, ProjectResponse, ProjectRow};
use crate::state::AppState;

async fn fetch_project(db: &PgPool, project_id: Uuid) -> Result<ProjectRow, (StatusCode, String)> {
    sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, owner_id, created_at FROM projects WHERE id = $1",
    )
    .bind(project_id)
    .fetch_optional(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Project does not exist".to_string()))
}

async fn fetch_access_role(db: &PgPool, project_id: Uuid, user_id: Uuid) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT role FROM project_access WHERE project_id = $1 AND user_id = $2",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
}

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let project_id = Uuid::new_v4();

    let created_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO projects (id, name, owner_id) VALUES ($1, $2, $3) RETURNING created_at",
    )
    .bind(project_id)
    .bind(&payload.name)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error occurred while creating project".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(ProjectResponse {
            id: project_id,
            name: payload.name,
            owner_id: claims.sub,
            role: "owner".to_string(),
            created_at,
        }),
    ))
}

pub async fn list_projects(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let rows: Vec<(Uuid, String, Uuid, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT p.id, p.name, p.owner_id,
               CASE WHEN p.owner_id = $1 THEN 'owner' ELSE pa.role END AS role,
               p.created_at
        FROM projects p
        LEFT JOIN project_access pa ON pa.project_id = p.id AND pa.user_id = $1
        WHERE p.owner_id = $1 OR pa.user_id = $1
        ORDER BY p.created_at DESC
        "#,
    )
    .bind(claims.sub)
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    let projects: Vec<ProjectResponse> = rows
        .into_iter()
        .map(|(id, name, owner_id, role, created_at)| ProjectResponse {
            id,
            name,
            owner_id,
            role,
            created_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(projects)))
}

pub async fn get_project(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let project = fetch_project(&state.db, project_id).await?;

    let role = if project.owner_id == claims.sub {
        "owner".to_string()
    } else {
        fetch_access_role(&state.db, project_id, claims.sub)
            .await
            .ok_or((StatusCode::FORBIDDEN, "You do not have access to this project".to_string()))?
    };

    Ok((
        StatusCode::OK,
        Json(ProjectResponse {
            id: project.id,
            name: project.name,
            owner_id: project.owner_id,
            role,
            created_at: project.created_at,
        }),
    ))
}

pub async fn grant_access(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<GrantAccessRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if payload.role != "read" && payload.role != "write" {
        return Err((StatusCode::BAD_REQUEST, "role must be 'read' or 'write'".to_string()));
    }

    let project = fetch_project(&state.db, project_id).await?;
    if project.owner_id != claims.sub {
        return Err((StatusCode::FORBIDDEN, "Only the project owner can grant access".to_string()));
    }

    let target_user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User does not exist".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO project_access (project_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id, user_id) DO UPDATE SET role = EXCLUDED.role
        "#,
    )
    .bind(project_id)
    .bind(target_user_id)
    .bind(&payload.role)
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error occurred while granting access".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(AccessResponse {
            project_id,
            user_id: target_user_id,
            role: payload.role,
        }),
    ))
}

pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(project_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let project = fetch_project(&state.db, project_id).await?;
    if project.owner_id != claims.sub {
        return Err((StatusCode::FORBIDDEN, "Only the project owner can delete the project".to_string()));
    }

    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(project_id)
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error occurred while deleting project".to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
