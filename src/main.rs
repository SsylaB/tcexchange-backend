use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tower_http::cors::{Any, CorsLayer};

// Types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeItem {
    id: u32,
    country: String,
    universityName: String,
    location: Option<String>,
    url: Option<String>,
    languages: Vec<String>,
    description: String,
    shortName: String,
    exchangeType: String,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
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

// Load knowledge base
fn load_knowledge_base() -> Vec<KnowledgeItem> {
    let data = include_str!("../data/destinations.json");
    serde_json::from_str(data).unwrap_or_default()
}

// Extract unique values
fn get_countries(destinations: &[KnowledgeItem]) -> Vec<String> {
    let mut countries: Vec<String> = destinations
        .iter()
        .map(|d| d.country.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    countries.sort();
    countries
}

fn get_languages(destinations: &[KnowledgeItem]) -> Vec<String> {
    let mut languages: Vec<String> = destinations
        .iter()
        .flat_map(|d| d.languages.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    languages.sort();
    languages
}

// Find destination by various criteria
fn find_destination<'a>(text: &str, destinations: &'a [KnowledgeItem]) -> Option<&'a KnowledgeItem> {
    let msg = text.to_lowercase();
    let mut best_match: Option<&KnowledgeItem> = None;
    let mut best_score: i32 = 0;

    for dest in destinations {
        let short_lower = dest.shortName.to_lowercase();
        let uni_lower = dest.universityName.to_lowercase();
        let mut score: i32 = 0;

        // Exact short name match
        if short_lower == msg {
            score = 100;
        }
        // Word boundary match for short name
        else if msg.split_whitespace().any(|word| word == short_lower.as_str()) {
            score = 50 + short_lower.len() as i32;
        }

        // University name keywords match
        let uni_words: Vec<&str> = uni_lower.split_whitespace().collect();
        for word in uni_words {
            if word.len() > 3 {
                let word_regex = regex::Regex::new(&format!(r"\b{}\b", regex::escape(word))).ok()?;
                if word_regex.is_match(&msg) {
                    score += 20 + word.len() as i32;
                }
            }
        }

        if score > best_score {
            best_score = score;
            best_match = Some(dest);
        }
    }

    if best_score >= 10 {
        best_match
    } else {
        None
    }
}

// Find destination in history
fn find_in_history<'a>(
    history: &'a [ChatMessage],
    destinations: &'a [KnowledgeItem],
) -> Option<&'a KnowledgeItem> {
    for msg in history.iter().rev().take(4) {
        if let Some(dest) = find_destination(&msg.content, destinations) {
            return Some(dest);
        }
    }
    None
}

// Format destination for response
fn format_destination(dest: &KnowledgeItem) -> String {
    let mut parts = vec![
        format!("{}: {}", dest.shortName, dest.universityName),
        if let Some(loc) = &dest.location {
            format!("📍 {}, {}", loc, dest.country)
        } else {
            format!("📍 {}", dest.country)
        },
        format!("🌐 Langues: {}", dest.languages.join(", ")),
        format!("📋 Type: {}", dest.exchangeType),
    ];

    if let Some(url) = &dest.url {
        parts.push(format!("🔗 {}", url));
    }

    parts.join("\n")
}

// Build response using exact matches
fn build_response(
    message: &str,
    history: &[ChatMessage],
    destinations: &[KnowledgeItem],
) -> Option<String> {
    let msg = message.trim().to_lowercase();
    let countries = get_countries(destinations);
    let languages = get_languages(destinations);

    // Greeting
    if matches!(msg.as_str(), "bonjour" | "salut" | "coucou" | "hey" | "hello") {
        return Some(format!(
            "Bonjour ! Je suis l'assistant TC Exchange d'INSA Lyon. Je peux t'aider à trouver des destinations d'échange parmi {} destinations dans {} pays.\n\nTu peux me demander :\n- Une destination spécifique (ex: \"c'est quoi KIT ?\")\n- Un pays (ex: \"destinations au Canada\")\n- Une langue (ex: \"où partir en anglais ?\")",
            destinations.len(),
            countries.len()
        ));
    }

    // List countries
    if msg.contains("liste") && msg.contains("pays") {
        return Some(format!(
            "{} pays disponibles :\n\n{}",
            countries.len(),
            countries.join(", ")
        ));
    }

    // Country queries (check first)
    for country in &countries {
        if msg.contains(&country.to_lowercase()) {
            let dests: Vec<&KnowledgeItem> = destinations
                .iter()
                .filter(|d| d.country.to_lowercase() == country.to_lowercase())
                .collect();

            let list: Vec<String> = dests
                .iter()
                .map(|d| {
                    if let Some(loc) = &d.location {
                        format!("- {}: {} ({})", d.shortName, d.universityName, loc)
                    } else {
                        format!("- {}: {}", d.shortName, d.universityName)
                    }
                })
                .collect();

            return Some(format!(
                "{} destinations en {} :\n\n{}",
                dests.len(),
                country,
                list.join("\n")
            ));
        }
    }

    // Location follow-up
    if (msg.contains("localisation") || msg == "ou" || msg.contains("où") || msg.contains("ville"))
        && !history.is_empty()
    {
        if let Some(prev_dest) = find_in_history(history, destinations) {
            return Some(format!(
                "📍 {} se trouve à {}, {}",
                prev_dest.shortName,
                prev_dest.location.as_deref().unwrap_or("ville non spécifiée"),
                prev_dest.country
            ));
        }
    }

    // Specific destination
    if let Some(dest) = find_destination(message, destinations) {
        return Some(format_destination(dest));
    }

    // Language queries
    for lang in &languages {
        if msg.contains(&lang.to_lowercase()) {
            let dests: Vec<&KnowledgeItem> = destinations
                .iter()
                .filter(|d| {
                    d.languages.iter().any(|l| {
                        l.to_lowercase().contains(&lang.to_lowercase())
                    })
                })
                .collect();

            let list: Vec<String> = dests
                .iter()
                .take(10)
                .map(|d| format!("- {} ({})", d.shortName, d.country))
                .collect();

            let extra = if dests.len() > 10 {
                format!("\n... et {} autres.", dests.len() - 10)
            } else {
                String::new()
            };

            return Some(format!(
                "{} destinations en {} :\n\n{}{}",
                dests.len(),
                lang,
                list.join("\n"),
                extra
            ));
        }
    }

    None
}

// Create AI prompt for unknown queries
fn create_ai_prompt(
    message: &str,
    history: &[ChatMessage],
    destinations: &[KnowledgeItem],
) -> String {
    let countries = get_countries(destinations);
    let languages = get_languages(destinations);

    let recent_history: Vec<String> = history
        .iter()
        .rev()
        .take(4)
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect();

    format!(
        r#"Tu es l'assistant TC Exchange d'INSA Lyon.

BASE DE DONNEES: {} destinations dans {} pays.

PAYS DISPONIBLES: {}...

LANGUES: {}

INSTRUCTIONS:
- Tu peux repondre aux questions generales sur les echanges
- Pour les destinations specifiques, dis "Je n'ai pas trouve cette destination dans ma base. Voici les pays disponibles..."
- Sois concis et amical
- Reponds en francais

CONVERSATION RECENTE:
{}

Message utilisateur: {}"#,
        destinations.len(),
        countries.len(),
        countries[..countries.len().min(20)].join(", "),
        languages.join(", "),
        recent_history.join("\n"),
        message
    )
}

// Call Groq API
async fn call_groq(message: &str, groq_key: &str) -> anyhow::Result<String> {
    let client = Client::new();

    let request_body = GroqRequest {
        messages: vec![
            GroqMessage {
                role: "user".to_string(),
                content: message.to_string(),
            },
        ],
        model: "llama-3.1-8b-instant".to_string(),
        temperature: 0.3,
        max_tokens: 512,
        top_p: 0.9,
    };

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", groq_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let groq_response: GroqResponse = response.json().await?;
        Ok(groq_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "Désolé, je n'ai pas compris.".to_string()))
    } else {
        let status = response.status();
        let text = response.text().await?;
        Err(anyhow::anyhow!("Groq API error: {} - {}", status, text))
    }
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    username: String,
}

// Handler for login endpoint
async fn login_handler(
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    println!("Login attempt: {}", payload.username);
    // Mock login - accept any username for now
    Ok((StatusCode::OK, Json(LoginResponse {
        username: payload.username,
    })))
}

// Handler for destinations endpoint
async fn destinations_handler() -> Result<impl IntoResponse, StatusCode> {
    let destinations = load_knowledge_base();
    Ok((StatusCode::OK, Json(destinations)))
}

// Handler for chat endpoint
async fn chat_handler(
    Json(payload): Json<ChatRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let destinations = load_knowledge_base();

    // Try exact match first
    if let Some(response) = build_response(&payload.message, &payload.history, &destinations) {
        return Ok((StatusCode::OK, Json(ChatResponse { response })));
    }

    // Fall back to Groq AI
    let groq_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let prompt = create_ai_prompt(&payload.message, &payload.history, &destinations
    );

    match call_groq(&prompt, &groq_key).await {
        Ok(response) => Ok((StatusCode::OK, Json(ChatResponse { response }))),
        Err(_) => Ok((
            StatusCode::OK,
            Json(ChatResponse {
                response: "Désolé, je rencontre un problème technique. Réessaie dans un instant !".to_string(),
            }),
        )),
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables
    if std::env::var("GROQ_API_KEY").is_err() {
        eprintln!("Warning: GROQ_API_KEY not set");
    }

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Router
    let app = Router::new()
        .route("/", get(|| async { "✅ TC Exchange Backend is running! Use POST /api/chat" }))
        .route("/api/auth/login", post(login_handler))
        .route("/api/destinations", get(destinations_handler))
        .route("/api/chat", post(chat_handler))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("🚀 Server running on http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .unwrap();
}