use sqlx::{SqlitePool, sqlite::{SqlitePoolOptions, SqliteConnectOptions}};
use std::str::FromStr;

pub async fn create_pool() -> SqlitePool {
    // On configure les options de connexion pour autoriser la création du fichier
    let connection_options = SqliteConnectOptions::from_str("sqlite:tcexchange.db")
        .unwrap()
        .create_if_missing(true); // <--- L'option magique est ici

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await
        .expect("Failed to connect to SQLite")
}