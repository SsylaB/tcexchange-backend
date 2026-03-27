use axum::{Json, http::StatusCode, routing::post, Router, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;  // For shared state (if needed later)

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub username: String,
}

// POST /auth/  { "username": "jdupont" } → /api/auth/login
pub async fn login(
    // TODO : implemeent further mock data (list of allowed logins?
    State(_pool): State<SqlitePool>,  // Add for future DB auth
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    if body.username.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(LoginResponse { username: body.username }))
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", post(login))  // /auth/ → /api/auth/login after nesting
}