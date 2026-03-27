use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow, Debug)] 
pub struct Destination {
    pub id: i64,
    pub university_name: String,
    pub country: String,
    pub location: Option<String>,
    pub url: Option<String>,
    pub exchange_type: Option<String>,
    pub languages: Option<String>,
    pub description: Option<String>,
    pub short_name: Option<String>,
    pub position: Option<String>,
}