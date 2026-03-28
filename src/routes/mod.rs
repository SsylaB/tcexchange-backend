use axum::{Router, routing::post};
use sqlx::SqlitePool;

// Déclaration des modules de routes
pub mod auth;
pub mod chat;
pub mod destinations;
pub mod quiz;
pub mod compare;

/// Point d'entrée principal pour toutes les routes de l'API.
/// Utilise le pattern .nest() pour regrouper les routes par thématique.
pub fn api_router() -> Router<SqlitePool> {
    Router::new()
        // Routes gérées par des sous-routeurs dédiés
        .nest("/destinations", destinations::router())
        .nest("/auth", auth::router())
        .nest("/quiz", quiz::router())
        .nest("/chat", chat::router())
        
        // Routes du comparateur branchées en mode "nested"
        .nest("/compare", Router::new()
            .route("/", post(compare::handle_compare))
            .route("/chat", post(compare::handle_compare_chat))
        )
}