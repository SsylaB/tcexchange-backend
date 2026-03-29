use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use dotenvy::dotenv;

pub async fn create_pool() -> SqlitePool {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "./tcexchange.db".to_string());

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to SQLite")
}
