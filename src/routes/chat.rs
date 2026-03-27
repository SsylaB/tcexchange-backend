use axum::{extract::{Json, State}, response::IntoResponse, routing::post, Router};
use sqlx::SqlitePool;
use crate::chat::{ChatRequest, ChatResponse, process_chat};
use crate::models::Destination;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", post(chat_handler))  // /chat/ → /api/chat
}

async fn chat_handler(
    State(pool): State<SqlitePool>,  // kept for future DB-backed chat if you want it
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    let destinations: Vec<Destination> = sqlx::query_as!(
        Destination,
        "SELECT * FROM destinations"
    )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    // Call process_chat with BOTH arguments
    let resp: ChatResponse = process_chat(payload, &destinations).await;

    Json(resp)
}