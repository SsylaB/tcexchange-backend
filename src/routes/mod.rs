use axum::{Router, routing::post};
use sqlx::SqlitePool;

// Déclaration des modules de routes
pub mod auth;
pub mod chat;
pub mod destinations;
pub mod quiz;
pub mod compare;

pub fn api_router() -> Router<SqlitePool> {
    Router::new()
        .nest("/destinations", destinations::router())
        .nest("/auth", auth::router())
        .nest("/quiz", quiz::router())
        .nest("/chat", chat::router())
        .nest("/compare", compare::router()
        )
}