pub mod auth;
use axum::{routing::{get, post}, Router, Json, extract::State};
use sqlx::SqlitePool;
use crate::models::Destination;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct QuizAnswers {
    pub answers: Vec<String>,
}

pub fn create_router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/api/destinations", get(get_destinations))
        .route("/api/auth/login", post(auth::login))
        .route("/api/quiz", post(get_ai_recommendation))
        .with_state(pool)
}

async fn get_destinations(
    State(pool): State<SqlitePool>,
) -> Json<Vec<Destination>> {
    // Utilisation de query_as (sans !) pour éviter les erreurs de compilation offline
    let destinations = sqlx::query_as::<_, Destination>(
        "SELECT id, name, country, continent, description, image_url, budget FROM destinations"
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    Json(destinations)
}

async fn get_ai_recommendation(
    State(pool): State<SqlitePool>,
    Json(payload): Json<QuizAnswers>,
) -> Json<serde_json::Value> {
    
    let chosen_continent = payload.answers.get(3).cloned().unwrap_or_default();

    // On utilise query_as (sans !) pour plus de souplesse
    let destinations = sqlx::query_as::<_, Destination>(
        "SELECT * FROM destinations WHERE continent = ? OR ? = 'Je suis ouvert à tout !'"
    )
    .bind(&chosen_continent)
    .bind(&chosen_continent)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

// --- CORRECTION DES TYPES (String vs Option<String>) ---
    let context_list: Vec<String> = destinations
        .iter()
        .take(12) 
        .map(|d| {
            // university_name et country sont des String (d'après l'erreur E0599)
            // location est une Option<String> (d'après l'erreur E0277 précédente)
            let univ = &d.university_name;
            let pays = &d.country;
            let ville = d.location.as_deref().unwrap_or("Ville inconnue");

            format!(
                "[ID: {}] {} ({}) - Ville: {}", 
                d.id, 
                univ, 
                pays, 
                ville
            )
        })
        .collect();
        
    let groq_api_key = "METTRE TOKEN GROQ ICI"; 
    let client = reqwest::Client::new();

    let system_prompt = "Tu es un expert en mobilité internationale. Analyse le profil de l'étudiant et propose un TOP 3 parmi la liste fournie. \
        Réponds UNIQUEMENT en JSON avec cette structure : \
        { \"recommendations\": [ { \"nom\": \"...\", \"pays\": \"...\", \"avis\": \"...\", \"points_forts\": [\"...\", \"...\", \"...\"] } ] }";

    let user_content = format!(
        "DESTINATIONS POSSIBLES : {}. RÉPONSES ÉTUDIANT : {:?}", 
        context_list.join(" | "), 
        payload.answers
    );

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", groq_api_key))
        .json(&serde_json::json!({
            "model": "llama-3.1-8b-instant",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_content}
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.7
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            let res_body = res.text().await.unwrap_or_default();
            let full_json: serde_json::Value = serde_json::from_str(&res_body).unwrap_or_default();
            let content = full_json["choices"][0]["message"]["content"].as_str().unwrap_or(&res_body);

            println!("--- IA RÉPONSE ---");
            println!("{}", content);

            Json(serde_json::json!({ "raw_text": content }))
        },
        Err(e) => {
            eprintln!("Erreur: {:?}", e);
            Json(serde_json::json!({ "raw_text": "Erreur service IA." }))
        }
    }
}
