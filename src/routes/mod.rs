pub mod auth;
pub mod quiz;
pub mod chat;
pub mod destinations;

use axum::Router;
use crate::db::create_pool;

pub async fn api_router() -> Router {
    let pool = create_pool().await;
    Router::new()
        .nest("/destinations", destinations::router())
        .nest("/auth", auth::router())
        .nest("/quiz", quiz::router())
        .nest("/chat", chat::router())
        .with_state(pool)
}
