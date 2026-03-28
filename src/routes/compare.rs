use axum::{extract::Json, http::StatusCode, response::IntoResponse, routing::post, Router};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DestinationInput {
    pub id: i64,
    pub name: String,
    pub country: String,
    pub location: String,
    #[serde(rename = "type")]
    pub exchange_type: String,
    pub languages: Vec<String>,
    pub description: String,
}

#[derive(Deserialize)]
pub struct CompareRequest {
    pub destinations: Vec<DestinationInput>,
    pub criteria: Vec<String>,
}

#[derive(Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct FollowupRequest {
    pub destinations: Vec<DestinationInput>,
    pub criteria: Vec<String>,
    pub ranking: Vec<String>,
    pub verdict: String,
    pub summaries: Vec<DestinationSummary>,
    pub messages: Vec<ChatMessage>,
    pub question: String,
}

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct CriterionEntry {
    pub criterion: String,
    pub level: String,
    pub assessment: String,
}

#[derive(Serialize, Deserialize)]
pub struct DestinationSummary {
    pub name: String,
    #[serde(default)]
    pub short_intro: String,
    #[serde(default)]
    pub criteria_breakdown: Vec<CriterionEntry>,
    #[serde(default)]
    pub best_for: String,
    #[serde(default)]
    pub analysis: String,
}

#[derive(Serialize, Deserialize)]
pub struct CompareResponse {
    pub destination_summaries: Vec<DestinationSummary>,
    pub ranking: Vec<String>,
    pub verdict: String,
}

#[derive(Serialize)]
pub struct FollowupResponse {
    pub answer: String,
}

// ─── Gemini types ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiConfig,
}

#[derive(Serialize)]
struct GeminiConfig {
    temperature: f32,
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiCandidatePart>,
}

#[derive(Deserialize)]
struct GeminiCandidatePart {
    text: String,
}

// ─── Gemini helper ────────────────────────────────────────────────────────────

async fn call_gemini(prompt: &str, json_mode: bool) -> anyhow::Result<String> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY manquante"))?;

    let mime = if json_mode {
        "application/json"
    } else {
        "text/plain"
    };

    let body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart {
                text: prompt.to_string(),
            }],
        }],
        generation_config: GeminiConfig {
            temperature: 0.2,
            response_mime_type: mime.to_string(),
        },
    };

    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let res = client.post(&url).json(&body).send().await?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await?;
        return Err(anyhow::anyhow!("Gemini error {}: {}", status, text));
    }

    let gemini: GeminiResponse = res.json().await?;
    let text = gemini
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_default();

    Ok(text.trim().to_string())
}

// ─── Handlers ────────────────────────────────────────────────────────────────

pub async fn handle_compare(
    Json(payload): Json<CompareRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let dest_index_block = payload
        .destinations
        .iter()
        .enumerate()
        .map(|(i, d)| format!("{}. {}", i + 1, d.name))
        .collect::<Vec<_>>()
        .join("\n");

    let dest_data_block = payload
        .destinations
        .iter()
        .enumerate()
        .map(|(i, d)| {
            format!(
                "DESTINATION_{}\nofficial_name: {}\ncountry: {}\nlocation: {}\ntype: {}\nlanguages: {}\ndescription: {}",
                i + 1,
                d.name,
                d.country,
                d.location,
                d.exchange_type,
                d.languages.join(", "),
                d.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let criteria_block = if payload.criteria.is_empty() {
        "Aucun critère explicite fourni".to_string()
    } else {
        payload.criteria.join(", ")
    };

    let prompt = format!(
        r#"Tu es un conseiller Erasmus/échange universitaire premium.
Tu parles directement à un étudiant INSA Lyon en Télécommunications.
Tu réponds en français, tu t'adresses toujours à lui avec "tu".

Destinations à comparer :
{dest_index_block}

Données disponibles :
{dest_data_block}

Critères prioritaires de l'étudiant :
{criteria_block}

Mission :
Fais une comparaison courte, nette, concrète, utile.

Règles obligatoires :
- Tu DOIS utiliser EXACTEMENT les noms officiels fournis ci-dessus dans le champ "name".
- Tu DOIS inclure TOUTES les destinations dans "destinationSummaries".
- Tu DOIS produire un classement complet dans "ranking" avec uniquement les noms exacts fournis.
- Tu ne dois inventer ni université, ni coût, ni météo, ni information absente.
- Si une information manque, dis-le de façon prudente et brève.
- Compare d'abord selon les critères donnés par l'étudiant.
- Fais ressortir ce qui distingue vraiment chaque destination des autres.

Format JSON attendu :
{{
  "destinationSummaries": [
    {{
      "name": "nom exact",
      "shortIntro": "1 phrase d'impression générale",
      "criteriaBreakdown": [
        {{"criterion": "nom critère", "level": "yes|medium|no", "assessment": "1 phrase courte"}}
      ],
      "bestFor": "1 phrase sur le profil adapté"
    }}
  ],
  "ranking": ["nom exact 1", "nom exact 2"],
  "verdict": "3 à 5 phrases expliquant le classement"
}}"#
    );

    let raw = call_gemini(&prompt, true)
        .await
        .map_err(|e| {
            eprintln!("Gemini compare error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        eprintln!("JSON parse error: {e}\nRaw: {raw}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(parsed))
}

pub async fn handle_followup(
    Json(payload): Json<FollowupRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let dest_block = payload
        .destinations
        .iter()
        .map(|d| {
            format!(
                "- {} | pays: {} | lieu: {} | type: {} | langues: {} | description: {}",
                d.name,
                d.country,
                d.location,
                d.exchange_type,
                d.languages.join(", "),
                d.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let summaries_block = payload
        .summaries
        .iter()
        .map(|s| format!("{}:\n{}", s.name, s.analysis))
        .collect::<Vec<_>>()
        .join("\n\n");

    let history_block = payload
        .messages
        .iter()
        .map(|m| {
            let role = if m.role == "user" { "Étudiant" } else { "Assistant" };
            format!("{}: {}", role, m.content)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let criteria_block = if payload.criteria.is_empty() {
        "Aucun critère explicite".to_string()
    } else {
        payload.criteria.join(", ")
    };

    let prompt = format!(
        r#"Tu es un conseiller Erasmus/échange très clair, très utile.
Tu parles directement à un étudiant INSA TC.
Tu réponds uniquement sur la comparaison en cours.
Tu tutoies toujours l'étudiant. Tu écris sans markdown.
Tu n'inventes pas d'informations absentes. Tu restes précis, comparatif et bref.

Destinations comparées :
{dest_block}

Critères choisis :
{criteria_block}

Classement actuel :
{ranking}

Synthèse actuelle :
{summaries_block}

Verdict actuel :
{verdict}

Historique :
{history}

Nouvelle question de l'étudiant :
{question}

Consignes : réponds en français, tutoie, 3 à 6 phrases, direct, comparatif, pas de markdown."#,
        ranking = payload.ranking.join(" > "),
        verdict = payload.verdict,
        history = if history_block.is_empty() { "Aucun".to_string() } else { history_block },
        question = payload.question,
    );

    let answer = call_gemini(&prompt, false)
        .await
        .map_err(|e| {
            eprintln!("Gemini followup error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(FollowupResponse { answer }))
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", post(handle_compare))
        .route("/followup", post(handle_followup))
}