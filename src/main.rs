mod models;
mod routes;
mod db;

use tower_http::cors::CorsLayer;
// Utilise crate::routes si ax_auth ne fonctionne pas, selon le nom de ton projet
use crate::routes::create_router; 

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = db::create_pool().await;

    // Migrations automatiques
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // On définit le CORS une seule fois de manière simple
    let cors = CorsLayer::permissive();

    // On applique le router et la couche CORS
    let app = create_router(pool)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    
    axum::serve(listener, app).await.unwrap();
}
