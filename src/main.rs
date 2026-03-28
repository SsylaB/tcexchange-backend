use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;

mod routes;
mod models;
mod db;
mod chat;
mod compare; // ✅ C'est cette ligne qui manquait pour lier le Moteur IA !

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    if std::env::var("GROQ_API_KEY").is_err() {
        eprintln!("Warning: GROQ_API_KEY not set");
    }

    let pool = db::create_pool().await;

    // Run migrations automatically on startup
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Simple permissive CORS
    let cors = CorsLayer::permissive();

    // Build API router (defined in routes/mod.rs)
    let api = routes::api_router();

    // Provide state ONCE at the outer router level
    let app = Router::new()
        .route(
            "/",
            get(|| async { "✅ TC Exchange Backend is running! Use POST /api/chat" }),
        )
        .nest("/api", api)
        .with_state(pool)  // after this, app: Router<()>
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("🚀 Server running on http://0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .unwrap();
}