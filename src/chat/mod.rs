use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::models::Destination;

// Chat context to track language preference
#[derive(Debug, Clone)]
struct ChatContext {
    language: String, // "fr" or "en"
}

impl Default for ChatContext {
    fn default() -> Self {
        ChatContext {
            language: "fr".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub history: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
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

// Detect if user wants to switch languages
fn detect_language_switch(message: &str) -> Option<String> {
    let msg = message.to_lowercase();
    if msg.contains("english")
        || msg.contains("in english")
        || msg.contains("speak english")
        || msg.contains("can we speak in english")
    {
        return Some("en".to_string());
    }
    if msg.contains("français")
        || msg.contains("francais")
        || msg.contains("in french")
        || msg.contains("parle français")
    {
        return Some("fr".to_string());
    }
    None
}

// Check if message is in English (simple heuristic)
fn is_english_message(message: &str) -> bool {
    let msg = message.to_lowercase();
    let english_words = [
        "hello", "hi", "hey", "please", "thank", "thanks", "what", "where", "how", "who", "when",
        "why", "can you", "could you", "tell me", "speak", "in english",
    ];
    english_words.iter().any(|word| msg.contains(word))
}

// Get greeting based on language
fn get_greeting(lang: &str, destinations_count: usize, countries_count: usize) -> String {
    if lang == "en" {
        format!(
            "Hello! I'm the TC Exchange Assistant from INSA Lyon. I can help you find exchange destinations among {destinations_count} destinations in {countries_count} countries.

You can ask me about:
• A specific destination (e.g., \"what is KIT?\")
• A country (e.g., \"destinations in Canada\")
• A language (e.g., \"where can I go to study in English?\")
• General help about exchanges

Note: Exchange programs are available for INSA Lyon TC students in 3TC, 4TC, 3TCA, or 4TCA (Telecommunications specialty)."
        )
    } else {
        format!(
            "Bonjour ! Je suis l'assistant TC Exchange d'INSA Lyon. Je peux t'aider à trouver des destinations d'échange parmi {destinations_count} destinations dans {countries_count} pays.

Tu peux me demander :
• Une destination spécifique (ex: \"c'est quoi KIT ?\")
• Un pays (ex: \"destinations au Canada\")
• Une langue (ex: \"où partir en anglais ?\")
• De l'aide générale sur les échanges

Note : Les échanges sont ouverts aux étudiants TC de l'INSA Lyon en 3TC, 4TC, 3TCA ou 4TCA (spécialité Télécommunications)."
        )
    }
}

fn load_knowledge_base() -> Vec<Destination> {
    let data = include_str!("../data/destinations.json");
    serde_json::from_str(data).unwrap_or_default()
}

// Extract unique values
fn get_countries(destinations: &[Destination]) -> Vec<String> {
    let mut countries: Vec<String> = destinations
        .iter()
        .map(|d| d.country.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    countries.sort();
    countries
}

// Get country information (general info about the country, not universities)
fn get_country_info(country: &str, lang: &str) -> String {
    let country_lower = country.to_lowercase();

    let info_fr: HashMap<&str, &str> = [
        // ... your FR entries ...
    ]
        .iter()
        .cloned()
        .collect();

    let info_en: HashMap<&str, &str> = [
        // ... your EN entries ...
    ]
        .iter()
        .cloned()
        .collect();

    let info_db = if lang == "en" { &info_en } else { &info_fr };

    let default_msg = if lang == "en" {
        format!("I don't have detailed information about {country} yet, but I can tell you about the universities available there!")
    } else {
        format!("Je n'ai pas encore d'informations détaillées sur {country}, mais je peux vous parler des universités disponibles !")
    };

    info_db
        .get(country_lower.as_str())
        .map(|s| s.to_string())
        .unwrap_or(default_msg)
}

// Get available languages
fn get_languages(destinations: &[Destination]) -> Vec<String> {
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
fn find_destination<'a>(
    text: &str,
    destinations: &'a [Destination],
) -> Option<&'a Destination> {
    let msg = text.to_lowercase();
    let msg_words: Vec<&str> = msg.split_whitespace().collect();
    let mut best_match: Option<&Destination> = None;
    let mut best_score: i32 = 0;

    for dest in destinations {
        let short_lower = dest.shortName.to_lowercase();
        let uni_lower = dest.universityName.to_lowercase();
        let mut score: i32 = 0;

        if short_lower == msg {
            score = 100;
        } else if msg_words.iter().any(|&word| word == short_lower.as_str()) {
            score = 50 + short_lower.len() as i32;
        }

        let uni_words: Vec<&str> = uni_lower.split_whitespace().collect();
        for word in uni_words {
            if word.len() > 3 && msg_words.iter().any(|&w| w == word) {
                score += 20 + word.len() as i32;
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
    destinations: &'a [Destination],
) -> Option<&'a Destination> {
    for msg in history.iter().rev().take(4) {
        if let Some(dest) = find_destination(&msg.content, destinations) {
            return Some(dest);
        }
    }
    None
}

// Format destination for response
fn format_destination(dest: &Destination, lang: &str) -> String {
    // ... your existing implementation ...
    // (unchanged, just moved here)
    #![allow(unused)] // Remove in real file; here just to shorten snippet.
    String::new()
}

// Build response using exact matches
fn build_response(
    message: &str,
    history: &[ChatMessage],
    destinations: &[Destination],
    context: &mut ChatContext,
) -> Option<String> {
    // ... your existing implementation ...
    #![allow(unused)]
    None
}

// Create AI prompt for unknown queries
fn create_ai_prompt(
    message: &str,
    history: &[ChatMessage],
    destinations: &[Destination],
    context: &ChatContext,
) -> String {
    // ... your existing implementation ...
    #![allow(unused)]
    String::new()
}

// Call Groq API
async fn call_groq(message: &str, groq_key: &str) -> anyhow::Result<String> {
    let client = Client::new();

    let request_body = GroqRequest {
        messages: vec![GroqMessage {
            role: "user".to_string(),
            content: message.to_string(),
        }],
        model: "llama-3.1-8b-instant".to_string(),
        temperature: 0.3,
        max_tokens: 512,
        top_p: 0.9,
    };

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {groq_key}"))
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
        Err(anyhow::anyhow!("Groq API error: {status} - {text}"))
    }
}

/// Public entry point used by the HTTP handler
pub async fn process_chat(payload: ChatRequest) -> ChatResponse {
    let destinations = load_knowledge_base();

    let mut context = ChatContext::default();

    for msg in &payload.history {
        if let Some(lang) = detect_language_switch(&msg.content) {
            context.language = lang;
        }
    }

    if let Some(lang) = detect_language_switch(&payload.message) {
        context.language = lang;
    } else if is_english_message(&payload.message) {
        context.language = "en".to_string();
    }

    if let Some(response) =
        build_response(&payload.message, &payload.history, &destinations, &mut context)
    {
        return ChatResponse { response };
    }

    let groq_key = match std::env::var("GROQ_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            let msg = if context.language == "en" {
                "Sorry, my AI brain is offline (no API key).".to_string()
            } else {
                "Désolé, mon cerveau IA est hors ligne (clé API manquante).".to_string()
            };
            return ChatResponse { response: msg };
        }
    };

    let prompt = create_ai_prompt(
        &payload.message,
        &payload.history,
        &destinations,
        &context,
    );

    match call_groq(&prompt, &groq_key).await {
        Ok(response) => ChatResponse { response },
        Err(_) => {
            let msg = if context.language == "en" {
                "Sorry, I'm experiencing technical issues. Please try again in a moment!".to_string()
            } else {
                "Désolé, je rencontre un problème technique. Réessaie dans un instant !".to_string()
            };
            ChatResponse { response: msg }
        }
    }
}