mod models;
mod routes;
mod db;

use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

// --- TYPES POUR LE CHATBOT ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeItem {
    id: u32,
    country: String,
    #[serde(rename = "universityName")]
    university_name: String,
    location: Option<String>,
    url: Option<String>,
    languages: Vec<String>,
    description: String,
    #[serde(rename = "shortName")]
    short_name: String,
    #[serde(rename = "exchangeType")]
    exchange_type: String,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    history: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    response: String,
}

#[derive(Debug, Serialize)]
struct GroqRequest {
    messages: Vec<GroqMessage>,
    model: String,
    temperature: f32,
    max_tokens: u32,
    top_p: f32,
}

#[derive(Debug, Serialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct GroqChoiceMessage {
    content: String,
}

// --- LOGIQUE DU CHATBOT ---

fn load_knowledge_base() -> Vec<KnowledgeItem> {
    // Note: Assure-toi que le dossier 'data' est à la racine de ton projet
    let data = include_str!("../data/destinations.json");
    serde_json::from_str(data).unwrap_or_else(|e| {
        eprintln!("Erreur lecture destinations.json: {}", e);
        vec![]
    })
}

fn find_destination<'a>(text: &str, destinations: &'a [KnowledgeItem]) -> Option<&'a KnowledgeItem> {
    let msg = text.to_lowercase();
    destinations.iter().find(|d| {
        msg.contains(&d.short_name.to_lowercase()) || msg.contains(&d.university_name.to_lowercase())
    })
}

async fn call_groq(user_msg: &str, key: &str) -> anyhow::Result<String> {
    let client = Client::new();
    let body = GroqRequest {
        messages: vec![GroqMessage { role: "user".to_string(), content: user_msg.to_string() }],
        model: "llama-3.1-8b-instant".to_string(),
        temperature: 0.5,
        max_tokens: 512,
        top_p: 1.0,
    };

    let response = client.post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", key))
        .json(&body)
        .send().await?;

    let res_json: GroqResponse = response.json().await?;
    Ok(res_json.choices[0].message.content.clone())
}

// --- HANDLER DU CHAT ---

async fn chat_handler(Json(payload): Json<ChatRequest>) -> impl IntoResponse {
    let destinations = load_knowledge_base();
    let msg = payload.message.trim().to_lowercase();

    // 1. Recherche locale (Exact Match dans le JSON)
    if let Some(dest) = find_destination(&msg, &destinations) {
        let resp = format!(
            "🏛️ **{}** ({})\n📍 {}, {}\n🌐 Langues: {}\n📋 Type: {}\n🔗 {}",
            dest.university_name, dest.short_name, 
            dest.location.as_deref().unwrap_or("N/A"), 
            dest.country, dest.languages.join(", "), 
            dest.exchange_type, dest.url.as_deref().unwrap_or("N/A")
        );
        return (StatusCode::OK, Json(ChatResponse { response: resp }));
    }

    // 2. Fallback Groq AI
    let groq_key = std::env::var("GROQ_API_KEY").unwrap_or_default();
    if groq_key.is_empty() {
        return (StatusCode::OK, Json(ChatResponse { 
            response: "Destination non trouvée et clé API IA non configurée.".to_string() 
        }));
    }

    match call_groq(&payload.message, &groq_key).await {
        Ok(res) => (StatusCode::OK, Json(ChatResponse { response: res })),
        Err(e) => {
            eprintln!("Erreur Groq: {:?}", e);
            (StatusCode::OK, Json(ChatResponse { response: "Erreur technique avec l'IA.".to_string() }))
        }
    }
}

// --- MAIN ---

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = db::create_pool().await;

    // Migrations SQLx
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Fusion des routes : create_router(pool) + route de chat
    let app = routes::create_router(pool)
        .route("/api/chat", post(chat_handler))
        .layer(cors);

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    
    axum::serve(listener, app).await.unwrap();
}