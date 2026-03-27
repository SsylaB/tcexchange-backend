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
    let query = "SELECT id, university_name, country, location, url, exchange_type, languages, description, short_name, position FROM destinations";
    let result: Result<Vec<Destination>, sqlx::Error> = sqlx::query_as::<_, Destination>(query).fetch_all(&pool).await;
    Json(result.unwrap_or_default())
}

async fn get_ai_recommendation(
    dotenv::dotenv().ok(); 
    let groq_api_key = std::env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set in .env");
    let url = format!(
        "https://api.groq.com/openai/v1/chat/completions", 
        groq_api_key
    );
    State(pool): State<SqlitePool>,
    Json(payload): Json<QuizAnswers>,
) -> Json<serde_json::Value> {
    
    // On récupère le continent (index 3 selon tes précédents messages)
    let chosen_continent = payload.answers.get(3).cloned().unwrap_or_default();

    println!("🎯 Filtrage SQL pour le continent : {}", chosen_continent);

    // Pré-sélection SQL : On cherche le continent dans le pays ou la description
    // On augmente la limite à 20 pour donner assez de choix à l'IA
    let query = "
        SELECT * FROM destinations 
        WHERE (country LIKE ? OR description LIKE ? OR ? = 'Je suis ouvert à tout !')
        LIMIT 20";

    let destinations = sqlx::query_as::<_, Destination>(query)
        .bind(format!("%{}%", chosen_continent))
        .bind(format!("%{}%", chosen_continent))
        .bind(&chosen_continent)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    // Construction de la liste pour l'IA
    let context_list: Vec<String> = destinations
        .iter()
        .map(|d| format!(
            "ID: {} | Univ: {} | Pays: {} | Type: {} | Description: {}", 
            d.id, d.university_name, d.country, 
            d.exchange_type.as_deref().unwrap_or("N/C"),
            d.description.as_deref().unwrap_or("Pas de description")
        ))
        .collect();
        
    let client = reqwest::Client::new();

    let system_prompt = "Tu es un algorithme de recommandation strict pour l'INSA. 

    ### RÈGLES CRITIQUES :
    1. INTERDICTION d'inventer des universités. Utilise UNIQUEMENT les noms présents dans la 'SOURCE_DB' fournie.
    2. Si la liste est vide, renvoie un JSON avec un tableau vide.
    3. Le champ 'avis' doit contenir un paragraphe de 3 lignes expliquant le choix selon le projet de l'étudiant.

    ### FORMAT DE RÉPONSE ATTENDU (JSON STRICT) :
    {
    \"recommendations\": [
        {
        \"nom\": \"NOM_EXACT_DE_LA_LISTE\",
        \"pays\": \"PAYS_DE_LA_LISTE\",
        \"avis\": \"Texte explicatif détaillé...\",
        \"compatibilite\": 85,
        \"points_forts\": [\"point 1\", \"point 2\"]
        }
    ]
    }";

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", groq_api_key))
        .json(&serde_json::json!({
            "model": "llama-3.1-8b-instant",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": format!("RÉPONSES QUIZ: {:?}\n\nLISTE DES DESTINATIONS PRÉ-SÉLECTIONNÉES:\n{}", payload.answers, context_list.join("\n"))}
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.4
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            let res_body = res.text().await.unwrap_or_default();
            let full_json: serde_json::Value = serde_json::from_str(&res_body).unwrap_or_default();
            let content = full_json["choices"][0]["message"]["content"].as_str().unwrap_or("{}");
            
            println!("--- IA RÉPONSE ---");
            println!("{}", content);

            Json(serde_json::json!({ "raw_text": content }))
        },
        Err(e) => {
            eprintln!("Erreur API Groq: {:?}", e);
            Json(serde_json::json!({ "raw_text": "{\"recommendations\": []}" }))
        }
    }
}
