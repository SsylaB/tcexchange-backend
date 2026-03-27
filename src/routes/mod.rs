pub mod auth;
pub mod quiz;
pub mod chat;
pub mod destinations;

use axum::Router;
use sqlx::SqlitePool;

pub fn api_router() -> Router<SqlitePool> {
    Router::new()
        .nest("/destinations", destinations::router())
        .nest("/auth", auth::router())
        .nest("/quiz", quiz::router())
        .nest("/chat", chat::router())
}