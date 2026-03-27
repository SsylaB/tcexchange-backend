use axum::Router;
use axum::routing::get;
use tower_http::cors::{Any as CorsAny, CorsLayer};
mod routes;
mod models;
mod db;
mod chat;

#[tokio::main]
async fn main() {
    // Load environment variables
    if std::env::var("GROQ_API_KEY").is_err() {
        eprintln!("Warning: GROQ_API_KEY not set");
    }

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(CorsAny)
        .allow_methods(CorsAny)
        .allow_headers(CorsAny);

    let app = Router::new()
        .route("/", get(|| async { "✅ TC Exchange Backend is running! Use POST /api/chat" }))
        .nest("/api", routes::api_router().await)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("🚀 Server running on http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .unwrap();
}