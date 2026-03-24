use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub username: String,
}

// POST /api/auth/login  { "username": "jdupont" }
pub async fn login(
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    if body.username.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(LoginResponse { username: body.username }))
}
