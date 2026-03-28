use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Deserialize)]
pub struct CompareRequest {
    pub destination_ids: Vec<i32>,
    pub criteria: Vec<String>,
}

pub async fn handle_compare(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CompareRequest>,
) -> impl IntoResponse {
    match crate::compare::generate_comparison(&pool, &payload.destination_ids, &payload.criteria).await {
        Ok(ai_json_text) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            ai_json_text
        ).into_response(),
        Err(e) if e == "QUOTA_EXCEEDED" => (StatusCode::TOO_MANY_REQUESTS, "QUOTA_EXCEEDED").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
    }
}

#[derive(Deserialize)]
pub struct CompareChatRequest {
    pub question: String,
    pub context: crate::compare::CompareChatContext,
    pub history: Vec<crate::compare::CompareChatMessage>,
}

pub async fn handle_compare_chat(
    Json(payload): Json<CompareChatRequest>,
) -> impl IntoResponse {
    match crate::compare::generate_followup(&payload.question, &payload.context, &payload.history).await {
        Ok(answer) => {
            let response_json = serde_json::json!({ "answer": answer });
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                serde_json::to_string(&response_json).unwrap()
            ).into_response()
        },
        Err(e) if e == "QUOTA_EXCEEDED" => (StatusCode::TOO_MANY_REQUESTS, "QUOTA_EXCEEDED").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
    }
}