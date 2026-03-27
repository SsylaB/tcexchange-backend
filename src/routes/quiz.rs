use axum::{extract::{State, Json}, response::IntoResponse, routing::post, Router};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Deserialize)]
pub struct QuizRequest { answers: Vec<String> }

#[derive(Serialize)]
pub struct QuizResponse { raw_text: String }  // Your Groq response

pub async fn handle_quiz(
    State(pool): State<SqlitePool>,  // DB for destinations/preferences
    Json(payload): Json<QuizRequest>,
) -> impl IntoResponse {
    // TODO: Move your real quiz logic + Groq call from main.rs
    // Example: fetch destinations, match quiz answers, ask Groq

    // let destinations = get_relevant_destinations(&pool, &payload.answers).await?;

    Json(QuizResponse {
        raw_text: format!("Quiz processed: {:?}", payload.answers)  // Placeholder
    })
}

pub fn router() -> Router<SqlitePool> {  // Expects pool state
    Router::new()
        .route("/", post(handle_quiz))  // /quiz/ → /api/quiz after nesting
}