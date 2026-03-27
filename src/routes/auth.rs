use axum::{Json, http::StatusCode, routing::post, Router, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;  // For shared state (future DB auth)

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub username: String,
}

// POST /api/auth/  { "username": "jdupont" }
pub async fn login(
    State(_pool): State<SqlitePool>,  // Reserved for future DB checks
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    if body.username.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(LoginResponse { username: body.username }))
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", post(login))  // /auth/ → /api/auth/ after nesting
}