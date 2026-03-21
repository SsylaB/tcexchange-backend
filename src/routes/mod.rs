use axum::{routing::get, Router, Json, extract::State};
use sqlx::SqlitePool;
use crate::models::Destination;


pub fn create_router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/api/destinations", get(get_destinations))
        .with_state(pool)
}

async fn get_destinations(
    State(pool): State<SqlitePool>,
) -> Json<Vec<Destination>> {
    let destinations : Vec<Destination> = sqlx::query_as!(
        Destination,
        "SELECT * FROM destinations"
    )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    Json(destinations)
}
