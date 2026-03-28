use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::env;

// Structure adaptée à ta table SQLite
#[derive(sqlx::FromRow)]
pub struct DestRow {
    pub university_name: Option<String>,
    pub location: Option<String>,
    pub country: Option<String>,
    pub description: Option<String>,
    pub languages: Option<String>,
    pub exchange_type: Option<String>,
}

// ==========================================
// 1. MOTEUR DE COMPARAISON INITIALE
// ==========================================

pub async fn generate_comparison(
    pool: &SqlitePool,
    destination_ids: &[i32],
    criteria: &[String],
) -> Result<String, String> {
    let api_key = env::var("GEMINI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("Clé API Gemini manquante dans le .env".to_string());
    }

    if destination_ids.is_empty() {
        return Err("Aucune destination sélectionnée".to_string());
    }

    // Adapté pour SQLite : on transforme le tableau [1, 2] en texte "1,2"
    let ids_str = destination_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let query_str = format!(
        "SELECT university_name, location, country, description, languages, exchange_type \
         FROM destinations WHERE id IN ({})",
        ids_str
    );

    // query_as contourne l'erreur SQLX_OFFLINE de la macro
    let destinations = match sqlx::query_as::<_, DestRow>(&query_str)
        .fetch_all(pool)
        .await
    {
        Ok(d) => d,
        Err(e) => return Err(format!("Erreur DB: {}", e)),
    };

    let mut destination_index_block = String::new();
    let mut destinations_block = String::new();

    for (i, d) in destinations.iter().enumerate() {
        let name = d.university_name.as_deref().unwrap_or("Non précisé");
        destination_index_block.push_str(&format!("{}. {}\n", i + 1, name));
        destinations_block.push_str(&format!(
            "DESTINATION_{}\nofficial_name: {}\ncountry: {}\nlocation: {}\ntype: {}\nlanguages: {}\ndescription: {}\n\n",
            i + 1,
            name,
            d.country.as_deref().unwrap_or("Non précisé"),
            d.location.as_deref().unwrap_or("Non précisé"),
            d.exchange_type.as_deref().unwrap_or("Non précisé"),
            d.languages.as_deref().unwrap_or("Non précisé"),
            d.description.as_deref().unwrap_or("Non précisé")
        ));
    }

    let criteria_str = if criteria.is_empty() {
        "Aucun critère explicite fourni".to_string()
    } else {
        criteria.join(", ")
    };

    // TON PROMPT ET TON SCHEMA JSON EXACTS (100% reproduits)
    // Les accolades { et } du JSON sont doublées ({{ et }}) car c'est la syntaxe Rust pour la macro format!
    let prompt = format!(
        r#"Tu es un conseiller Erasmus/échange universitaire premium.
Tu parles directement à un étudiant INSA Lyon en Télécommunications.
Tu réponds en français, tu t'adresses toujours à lui avec "tu".

Destinations à comparer :
{}

Données disponibles :
{}

Critères prioritaires de l'étudiant :
{}

Mission :
Fais une comparaison courte, nette, concrète, utile.

Règles obligatoires :
- Tu DOIS utiliser EXACTEMENT les noms officiels fournis ci-dessus dans le champ "name".
- Tu DOIS inclure TOUTES les destinations dans "destinationSummaries".
- Tu DOIS produire un classement complet dans "ranking" avec uniquement les noms exacts fournis.
- Tu ne dois inventer ni université, ni coût, ni météo, ni information absente.
- Si une information manque, dis-le de façon prudente et brève.
- Pas de longs paragraphes.
- Compare d'abord selon les critères donnés par l'étudiant.
- Fais ressortir ce qui distingue vraiment chaque destination des autres.
- Le classement doit dépendre uniquement des critères fournis.
- Si deux destinations sont proches, conserve l'ordre le plus logique selon les critères explicitement mentionnés.
- Base-toi d'abord sur les critères choisis, puis seulement ensuite sur les autres éléments.

Format attendu pour chaque destination :
- shortIntro : 1 phrase courte d'impression générale
- criteriaBreakdown : 2 à 5 entrées maximum
- pour chaque entrée de criteriaBreakdown :
  - criterion : nom du critère
  - level : utilise UNIQUEMENT yes, medium ou no
  - assessment : 1 phrase courte et concrète
- bestFor : 1 phrase courte sur le profil d'étudiant adapté

Le verdict :
- 3 à 5 phrases maximum
- il doit expliquer pourquoi le top 1 passe devant les autres
- il doit mentionner explicitement les critères les plus décisifs
- il doit être direct, utile et personnalisé.

JSON Schema:
{{
  "type": "object",
  "properties": {{
    "destinationSummaries": {{
      "type": "array",
      "items": {{
        "type": "object",
        "properties": {{
          "name": {{ "type": "string" }},
          "shortIntro": {{ "type": "string" }},
          "criteriaBreakdown": {{
            "type": "array",
            "items": {{
              "type": "object",
              "properties": {{
                "criterion": {{ "type": "string" }},
                "level": {{ "type": "string" }},
                "assessment": {{ "type": "string" }}
              }},
              "required": ["criterion", "level", "assessment"]
            }}
          }},
          "bestFor": {{ "type": "string" }}
        }},
        "required": ["name", "shortIntro", "criteriaBreakdown", "bestFor"]
      }}
    }},
    "ranking": {{ "type": "array", "items": {{ "type": "string" }} }},
    "verdict": {{ "type": "string" }}
  }},
  "required": ["destinationSummaries", "ranking", "verdict"]
}}"#,
        destination_index_block.trim(),
        destinations_block.trim(),
        criteria_str
    );

    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-09-2025:generateContent?key={}",
        api_key
    );

    let gemini_payload = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": {
            "temperature": 0.2,
            "responseMimeType": "application/json"
        }
    });

    match client.post(&url).json(&gemini_payload).send().await {
        Ok(res) => {
            if res.status().as_u16() == 429 {
                return Err("QUOTA_EXCEEDED".to_string());
            }
            let body: serde_json::Value = res.json().await.unwrap_or_default();
            
            // On vérifie si on n'a pas tapé de limite de quota invisible dans la réponse brute
            let raw_text = body["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("");
            if raw_text.contains("RESOURCE_EXHAUSTED") || raw_text.contains("429") {
                return Err("QUOTA_EXCEEDED".to_string());
            }

            if !raw_text.is_empty() {
                Ok(raw_text.to_string())
            } else {
                Err("Erreur de formatage IA".to_string())
            }
        }
        Err(_) => Err("Erreur réseau vers Gemini".to_string()),
    }
}

// ==========================================
// 2. MOTEUR DU CHAT FOLLOW-UP
// ==========================================

#[derive(Deserialize)]
pub struct CompareChatContext {
    pub destinations: Vec<String>,
    pub criteria: Vec<String>,
    pub previous_verdict: String,
}

#[derive(Deserialize)]
pub struct CompareChatMessage {
    pub role: String,
    pub content: String,
}

pub async fn generate_followup(
    question: &str,
    context: &CompareChatContext,
    history: &[CompareChatMessage],
) -> Result<String, String> {
    let api_key = env::var("GEMINI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("Clé API manquante".to_string());
    }

    let history_block = history
        .iter()
        .map(|m| format!("{}: {}", if m.role == "user" { "Étudiant" } else { "Assistant" }, m.content))
        .collect::<Vec<String>>()
        .join("\n");

    // Ton prompt exact adapté pour garder tout le contexte (Destinations, Critères, Verdict et Historique)
    let prompt = format!(
        "Tu es un conseiller Erasmus INSA TC Lyon.\n\n\
        Contexte de la comparaison en cours :\n\
        - Destinations : {:?}\n\
        - Critères : {:?}\n\
        - Ton verdict précédent : {}\n\n\
        Question : {}\n\n\
        Historique de conversation : {}",
        context.destinations,
        context.criteria,
        context.previous_verdict,
        question,
        if history_block.is_empty() { "Aucun" } else { &history_block }
    );

    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-09-2025:generateContent?key={}",
        api_key
    );

    let payload = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }]
    });

    match client.post(&url).json(&payload).send().await {
        Ok(res) => {
            if res.status().as_u16() == 429 {
                return Err("QUOTA_EXCEEDED".to_string());
            }
            let body: serde_json::Value = res.json().await.unwrap_or_default();
            if let Some(ai_text) = body["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                Ok(ai_text.to_string())
            } else {
                Err("Erreur formatage IA".to_string())
            }
        }
        Err(_) => Err("Erreur réseau vers Gemini".to_string()),
    }
}