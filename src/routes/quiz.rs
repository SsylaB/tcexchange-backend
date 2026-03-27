use axum::{extract::{State, Json}, response::IntoResponse, routing::post, Router};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use anyhow::Context;

// --------- Public types (API contract) ---------

#[derive(Serialize, Deserialize)]
pub struct QuizRequest {
    pub answers: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Recommendation {
    pub nom: String,
    pub pays: String,
    pub avis: String,
    pub points_forts: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct QuizResponse {
    pub recommendations: Vec<Recommendation>,
}

// --------- Groq types (internal) ---------

#[derive(Serialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct GroqRequestBody {
    model: String,
    messages: Vec<GroqMessage>,
    temperature: f32,
    max_tokens: u32,
    top_p: f32,
    #[serde(rename = "response_format")]
    response_format: GroqResponseFormat,
}

#[derive(Serialize)]
struct GroqResponseFormat {
    #[serde(rename = "type")]
    r#type: String,
}

#[derive(Deserialize)]
struct GroqChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct GroqChoice {
    message: GroqChoiceMessage,
}

#[derive(Deserialize)]
struct GroqCompletionResponse {
    choices: Vec<GroqChoice>,
}

// --------- Handler ---------

pub async fn handle_quiz(
    State(_pool): State<SqlitePool>,  // reserved for future DB use
    Json(payload): Json<QuizRequest>,
) -> impl IntoResponse {
    // Get API key
    let groq_key = match std::env::var("GROQ_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            // Fallback: deterministic mock if no key configured
            let fallback = fallback_recommendations(&payload.answers);
            return Json(QuizResponse {
                recommendations: fallback,
            });
        }
    };

    // Build prompts
    let answers_text = payload
        .answers
        .iter()
        .enumerate()
        .map(|(i, ans)| format!("Q{}: {}", i + 1, ans))
        .collect::<Vec<_>>()
        .join(", ");

    let system_prompt = r#"
Tu es un assistant d'orientation pour les étudiants TC de l'INSA Lyon.
Ta tâche est de proposer un TOP 3 de destinations d'échange adaptées au profil de l'étudiant.

Tu DOIS répondre EXCLUSIVEMENT au format JSON suivant, sans texte autour :
{
  "recommendations": [
    {
      "nom": "Nom court de la destination (ex: KIT)",
      "pays": "Pays (ex: Allemagne)",
      "avis": "Court paragraphe en français expliquant pourquoi cette destination est adaptée",
      "points_forts": [
        "Point fort 1",
        "Point fort 2",
        "Point fort 3"
      ]
    }
  ]
}

- Toujours renvoyer entre 1 et 3 recommandations.
- Le JSON doit être valide.
"#;

    let user_prompt = format!(
        "Voici les réponses de l'étudiant au quiz : [{}]. Propose-lui un TOP 3 de destinations.",
        answers_text
    );

    // Call Groq and try to parse JSON into QuizResponse
    let result = call_groq_for_quiz(&system_prompt, &user_prompt, &groq_key).await;

    match result {
        Ok(resp) => Json(resp),
        Err(err) => {
            eprintln!("Groq quiz error: {err:?}");
            // Fallback: still return something the frontend can display
            let fallback = fallback_recommendations(&payload.answers);
            Json(QuizResponse {
                recommendations: fallback,
            })
        }
    }
}

// Router wiring (unchanged)
pub fn router() -> Router<SqlitePool> {
    Router::new().route("/", post(handle_quiz))
}

// --------- Groq helper ---------

async fn call_groq_for_quiz(
    system_prompt: &str,
    user_prompt: &str,
    api_key: &str,
) -> anyhow::Result<QuizResponse> {
    let client = reqwest::Client::new();

    let body = GroqRequestBody {
        model: "llama-3.1-8b-instant".to_string(),
        messages: vec![
            GroqMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            GroqMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            },
        ],
        temperature: 0.7,
        max_tokens: 512,
        top_p: 0.9,
        response_format: GroqResponseFormat {
            r#type: "json_object".to_string(),
        },
    };

    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to call Groq API")?;

    let status = res.status();
    let text = res.text().await.context("Failed to read Groq response body")?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Groq API error: status={} body={}",
            status,
            text
        ));
    }

    // Parse the outer completion, then parse the JSON content into our struct
    let completion: GroqCompletionResponse = serde_json::from_str(&text)
        .context("Failed to deserialize Groq completion JSON")?;

    let content = completion
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| anyhow::anyhow!("Groq returned no choices"))?;

    let parsed: QuizResponse = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse Groq content as QuizResponse: {}", content))?;

    Ok(parsed)
}

// --------- Fallback if Groq is down / misconfigured ---------

fn fallback_recommendations(answers: &[String]) -> Vec<Recommendation> {
    vec![Recommendation {
        nom: "KIT".to_string(),
        pays: "Allemagne".to_string(),
        avis: format!(
            "Basé sur tes réponses ({:?}), KIT semble un excellent match.",
            answers
        ),
        points_forts: vec![
            "Excellente réputation en ingénierie".to_string(),
            "Nombreux cours en anglais".to_string(),
            "Fort environnement de recherche".to_string(),
        ],
    }]
}