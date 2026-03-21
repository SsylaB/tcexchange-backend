use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub async fn create_pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:./tcexchange.db")
        .await
        .expect("Failed to connect to SQLite")
}
