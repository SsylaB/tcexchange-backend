use axum::{routing::get, Router, extract::State, Json};
use sqlx::SqlitePool;
use crate::models::Destination;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(get_destinations))  // /destinations/ → /api/destinations
}

async fn get_destinations(
    State(pool): State<SqlitePool>,
) -> Json<Vec<Destination>> {
    let destinations: Vec<Destination> = sqlx::query_as!(
        Destination,
        "SELECT * FROM destinations"
    )
        .fetch_all(&pool)
        .await
        .inspect_err(|e| eprintln!("DB error: {e}"))
        .unwrap_or_default();

    Json(destinations)
}