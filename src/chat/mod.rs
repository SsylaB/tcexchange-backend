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

// ---------- Language helpers ----------

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

fn is_english_message(message: &str) -> bool {
    let msg = message.to_lowercase();
    let english_words = [
        "hello", "hi", "hey", "please", "thank", "thanks", "what", "where",
        "how","who","when","why","can you","could you","tell me","speak","in english",
    ];
    english_words.iter().any(|word| msg.contains(word))
}

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

// ---------- Data helpers using Destination ----------

fn normalize_languages(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

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

fn get_languages(destinations: &[Destination]) -> Vec<String> {
    let mut langs: Vec<String> = destinations
        .iter()
        .flat_map(|d| normalize_languages(d.languages.as_deref()))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    langs.sort();
    langs
}

// Country info DB (same content as your old routes/chat.rs)
fn get_country_info(country: &str, lang: &str) -> String {
    let country_lower = country.to_lowercase();

    let info_fr: HashMap<&str, &str> = [
        ("tunisie", "🇹🇳 **Tunisie**\n\nLa Tunisie est un pays d'Afrique du Nord situé sur la côte méditerranéenne. C'est une destination populaire pour les échanges grâce à :\n\n🎓 **Éducation** : Universités d'ingénieurs reconnues comme ENIS (Sfax), ENIT (Tunis) et Université de Monastir\n🌍 **Langue** : Arabe et Français (pratique pour les étudiants français)\n💰 **Coût de vie** : Abordable comparé à l'Europe\n🌡️ **Climat** : Méditerranéen avec des étés chauds\n🎭 **Culture** : Riche histoire berbère, arabe et méditerranéenne\n\nC'est une excellente destination pour découvrir l'Afrique du Nord tout en continuant vos études d'ingénieur en français."),
        ("allemagne", "🇩🇪 **Allemagne**\n\nL'Allemagne est une destination privilégiée pour les échanges Erasmus, avec de nombreuses universités techniques de renom.\n\n🎓 **Éducation** : Excellente réputation en ingénierie (KIT, TU, RWTH...)\n🌍 **Langue** : Allemand et Anglais (de nombreux cours en anglais)\n💰 **Coût de vie** : Modéré, souvent sans frais d'inscription\n🏭 **Industrie** : Forte présence automobile, mécanique et high-tech\n🎭 **Culture** : Riche histoire, festivals, vie étudiante dynamique\n\nIdéal pour les étudiants TC souhaitant une expérience technique de haut niveau."),
        ("canada", "🇨🇦 **Canada**\n\nLe Canada offre une expérience nord-américaine avec une touche européenne.\n\n🎓 **Éducation** : Universités de recherche de classe mondiale\n🌍 **Langue** : Anglais et Français (Québec)\n💰 **Coût de vie** : Variable selon les villes (Toronto/Vancouver élevées)\n🌲 **Nature** : Paysages exceptionnels, quatre saisons marquées\n🏙️ **Villes** : Multiculturelles et dynamiques\n\nExcellent choix pour une immersion en anglais ou en français hors Europe."),
    ]
        .iter()
        .cloned()
        .collect();

    let info_en: HashMap<&str, &str> = [
        ("tunisie", "🇹🇳 **Tunisia**\n\nTunisia is a North African country on the Mediterranean coast. It's a popular exchange destination thanks to:\n\n🎓 **Education**: Renowned engineering schools like ENIS (Sfax), ENIT (Tunis), and University of Monastir\n🌍 **Language**: Arabic and French (convenient for French-speaking students)\n💰 **Cost of living**: Affordable compared to Europe\n🌡️ **Climate**: Mediterranean with hot summers\n🎭 **Culture**: Rich Berber, Arab, and Mediterranean history\n\nIt's an excellent destination to discover North Africa while continuing engineering studies in French."),
        ("allemagne", "🇩🇪 **Germany**\n\nGermany is a prime destination for Erasmus exchanges, with many renowned technical universities.\n\n🎓 **Education**: Excellent reputation in engineering (KIT, TU, RWTH...)\n🌍 **Language**: German and English (many courses in English)\n💰 **Cost of living**: Moderate, often no tuition fees\n🏭 **Industry**: Strong automotive, mechanical, and high-tech sectors\n🎭 **Culture**: Rich history, festivals, vibrant student life\n\nIdeal for TC students seeking a high-level technical experience."),
        ("canada", "🇨🇦 **Canada**\n\nCanada offers a North American experience with a European touch.\n\n🎓 **Education**: World-class research universities\n🌍 **Language**: English and French (Quebec)\n💰 **Cost of living**: Varies by city (Toronto/Vancouver are expensive)\n🌲 **Nature**: Exceptional landscapes, four distinct seasons\n🏙️ **Cities**: Multicultural and dynamic\n\nExcellent choice for immersion in English or French outside Europe."),
    ]
        .iter()
        .cloned()
        .collect();

    let info_db = if lang == "en" { &info_en } else { &info_fr };

    let default_msg = if lang == "en" {
        format!(
            "I don't have detailed information about {country} yet, but I can tell you about the universities available there!"
        )
    } else {
        format!(
            "Je n'ai pas encore d'informations détaillées sur {country}, mais je peux vous parler des universités disponibles !"
        )
    };

    info_db
        .get(country_lower.as_str())
        .map(|s| s.to_string())
        .unwrap_or(default_msg)
}

// ---------- Matching logic using Destination ----------

fn find_destination<'a>(text: &str, destinations: &'a [Destination]) -> Option<&'a Destination> {
    let msg = text.to_lowercase();
    let msg_words: Vec<&str> = msg.split_whitespace().collect();
    let mut best_match: Option<&Destination> = None;
    let mut best_score: i32 = 0;

    for dest in destinations {
        let short_lower = dest
            .short_name
            .as_deref()
            .unwrap_or("")
            .to_lowercase();
        let uni_lower = dest.university_name.to_lowercase();
        let mut score: i32 = 0;

        if !short_lower.is_empty() && short_lower == msg {
            score = 100;
        } else if !short_lower.is_empty()
            && msg_words.iter().any(|&word| word == short_lower.as_str())
        {
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

fn format_destination(dest: &Destination, lang: &str) -> String {
    let short = dest.short_name.as_deref().unwrap_or("N/A");
    let loc = dest.location.as_deref();
    let langs = normalize_languages(dest.languages.as_deref()).join(", ");
    let ex_type = dest.exchange_type.as_deref().unwrap_or("N/A");
    let desc = dest.description.as_deref().unwrap_or("");

    if lang == "en" {
        let mut parts = vec![
            format!("{short}: {}", dest.university_name),
            if let Some(loc) = loc {
                format!("📍 {loc}, {}", dest.country)
            } else {
                format!("📍 {}", dest.country)
            },
            format!("🌐 Languages: {langs}"),
            format!("📋 Type: {ex_type}"),
        ];
        if let Some(url) = &dest.url {
            parts.push(format!("🔗 {url}"));
        }
        if !desc.is_empty() {
            parts.push(format!("📝 {desc}"));
        }
        parts.join("\n")
    } else {
        let mut parts = vec![
            format!("{short}: {}", dest.university_name),
            if let Some(loc) = loc {
                format!("📍 {loc}, {}", dest.country)
            } else {
                format!("📍 {}", dest.country)
            },
            format!("🌐 Langues: {langs}"),
            format!("📋 Type: {ex_type}"),
        ];
        if let Some(url) = &dest.url {
            parts.push(format!("🔗 {url}"));
        }
        if !desc.is_empty() {
            parts.push(format!("📝 {desc}"));
        }
        parts.join("\n")
    }
}

// ---------- Rule-based response ----------

fn build_response(
    message: &str,
    history: &[ChatMessage],
    destinations: &[Destination],
    context: &mut ChatContext,
) -> Option<String> {
    let msg = message.trim().to_lowercase();
    let countries = get_countries(destinations);
    let languages = get_languages(destinations);

    // Language switch
    if let Some(new_lang) = detect_language_switch(message) {
        context.language = new_lang.clone();
        if new_lang == "en" {
            return Some(
                "Sure! I can now respond in English. How can I help you with your exchange program?"
                    .to_string(),
            );
        } else {
            return Some(
                "Bien sûr ! Je vais maintenant répondre en français. Comment puis-je vous aider pour votre échange ?"
                    .to_string(),
            );
        }
    }

    // Auto-detect language
    if is_english_message(message) {
        context.language = "en".to_string();
    }

    let lang = context.language.clone();

    // Greeting
    if matches!(
        msg.as_str(),
        "bonjour" | "salut" | "coucou" | "hey" | "hello" | "hi"
    ) {
        return Some(get_greeting(&lang, destinations.len(), countries.len()));
    }

    // List countries
    if msg.contains("liste") && msg.contains("pays")
        || msg.contains("list") && msg.contains("countr")
    {
        if lang == "en" {
            return Some(format!(
                "{} countries available:\n\n{}",
                countries.len(),
                countries.join(", ")
            ));
        } else {
            return Some(format!(
                "{} pays disponibles :\n\n{}",
                countries.len(),
                countries.join(", ")
            ));
        }
    }

    // Country mention
    let mut matched_country: Option<String> = None;
    for country in &countries {
        let country_lower = country.to_lowercase();
        if msg.contains(&country_lower) {
            matched_country = Some(country.clone());
            break;
        }
    }

    if let Some(country) = matched_country {
        let dests: Vec<&Destination> = destinations
            .iter()
            .filter(|d| d.country.to_lowercase() == country.to_lowercase())
            .collect();

        let asks_for_universities = msg.contains("universit")
            || msg.contains("école")
            || msg.contains("ecole")
            || msg.contains("school")
            || msg.contains("destinations")
            || msg.contains("options")
            || msg.contains("disponible")
            || msg.contains("available");

        let asks_about_country = msg.contains("parle")
            || msg.contains("tell")
            || msg.contains("about")
            || msg.contains("inform")
            || msg.contains("qu'est-ce")
            || msg.contains("c'est quoi")
            || msg.contains("what is");

        if asks_for_universities || (!asks_about_country && !msg.contains("pays")) {
            let list: Vec<String> = dests
                .iter()
                .map(|d| {
                    let short = d.short_name.as_deref().unwrap_or("N/A");
                    if let Some(loc) = &d.location {
                        format!("- {short}: {} ({loc})", d.university_name)
                    } else {
                        format!("- {short}: {}", d.university_name)
                    }
                })
                .collect();

            if lang == "en" {
                return Some(format!(
                    "{} destinations in {}:\n\n{}",
                    dests.len(),
                    country,
                    list.join("\n")
                ));
            } else {
                return Some(format!(
                    "{} destinations en {} :\n\n{}",
                    dests.len(),
                    country,
                    list.join("\n")
                ));
            }
        } else {
            return Some(get_country_info(&country, &lang));
        }
    }

    // Location follow-up
    if (msg.contains("localisation")
        || msg == "ou"
        || msg.contains("où")
        || msg.contains("ville")
        || msg.contains("where")
        || msg.contains("location"))
        && !history.is_empty()
    {
        if let Some(prev_dest) = find_in_history(history, destinations) {
            let short = prev_dest.short_name.as_deref().unwrap_or("N/A");
            if lang == "en" {
                return Some(format!(
                    "📍 {short} is located in {}, {}",
                    prev_dest.location.as_deref().unwrap_or("city not specified"),
                    prev_dest.country
                ));
            } else {
                return Some(format!(
                    "📍 {short} se trouve à {}, {}",
                    prev_dest.location.as_deref().unwrap_or("ville non spécifiée"),
                    prev_dest.country
                ));
            }
        }
    }

    // Specific destination
    if let Some(dest) = find_destination(message, destinations) {
        return Some(format_destination(dest, &lang));
    }

    // Language queries
    for lang_query in &languages {
        if msg.contains(&lang_query.to_lowercase()) {
            let dests: Vec<&Destination> = destinations
                .iter()
                .filter(|d| {
                    normalize_languages(d.languages.as_deref())
                        .iter()
                        .any(|l| l.to_lowercase().contains(&lang_query.to_lowercase()))
                })
                .collect();

            let list: Vec<String> = dests
                .iter()
                .take(10)
                .map(|d| {
                    let short = d.short_name.as_deref().unwrap_or("N/A");
                    format!("- {short} ({})", d.country)
                })
                .collect();

            let extra = if dests.len() > 10 {
                format!("\n... et {} autres.", dests.len() - 10)
            } else {
                String::new()
            };

            if lang == "en" {
                return Some(format!(
                    "{} destinations where you can study in {}:\n\n{}{}",
                    dests.len(),
                    lang_query,
                    list.join("\n"),
                    extra
                ));
            } else {
                return Some(format!(
                    "{} destinations en {} :\n\n{}{}",
                    dests.len(),
                    lang_query,
                    list.join("\n"),
                    extra
                ));
            }
        }
    }

    None
}

// ---------- AI fallback ----------

fn create_ai_prompt(
    message: &str,
    history: &[ChatMessage],
    destinations: &[Destination],
    context: &ChatContext,
) -> String {
    let countries = get_countries(destinations);
    let languages = get_languages(destinations);

    let recent_history: Vec<String> = history
        .iter()
        .rev()
        .take(4)
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect();

    if context.language == "en" {
        format!(
            r#"You are the TC Exchange Assistant from INSA Lyon (Institut National des Sciences Appliquees de Lyon).

CONTEXT:
- You help engineering students at INSA Lyon find exchange destinations
- Exchange programs are ONLY available for TC (Telecommunications) students in years 3TC, 4TC, 3TCA, or 4TCA
- TC stands for Telecommunications specialty
- Students study engineering only
- Available destinations: {} in {} countries

AVAILABLE COUNTRIES: {}...

LANGUAGES: {}

INSTRUCTIONS:
- Answer general questions about exchanges for engineering students
- Be friendly and concise
- If asked about specific destinations not in the database, say "I couldn't find that destination in my database. Here are the available countries..."
- Keep responses under 3 sentences when possible
- Remember: Only TC students (3TC, 4TC, 3TCA, 4TCA) can apply for exchanges

RECENT CONVERSATION:
{}

User message: {}"#,
            destinations.len(),
            countries.len(),
            countries[..countries.len().min(20)].join(", "),
            languages.join(", "),
            recent_history.join("\n"),
            message
        )
    } else {
        format!(
            r#"Tu es l'assistant TC Exchange d'INSA Lyon (Institut National des Sciences Appliquees de Lyon).

CONTEXTE:
- Tu aides les etudiants en ecole d'ingenieurs de l'INSA Lyon a trouver des destinations d'echange
- Les echanges sont UNIQUEMENT disponibles pour les etudiants TC (Telecommunications) en 3TC, 4TC, 3TCA ou 4TCA
- TC signifie specialite Telecommunications
- Les etudiants etudient uniquement l'ingenierie
- Destinations disponibles : {} dans {} pays

PAYS DISPONIBLES: {}...

LANGUES: {}

INSTRUCTIONS:
- Reponds aux questions generales sur les echanges pour etudiants en ingenierie
- Sois concis et amical
- Pour les destinations specifiques non dans la base, dis "Je n'ai pas trouve cette destination dans ma base. Voici les pays disponibles..."
- Garde les reponses courtes (max 3 phrases)
- Rappelle : Seuls les etudiants TC (3TC, 4TC, 3TCA, 4TCA) peuvent candidater aux echanges

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
}

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

// ---------- Public entry point (DB-backed) ----------

pub async fn process_chat(
    payload: ChatRequest,
    destinations: &[Destination],
) -> ChatResponse {
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
        build_response(&payload.message, &payload.history, destinations, &mut context)
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
        destinations,
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