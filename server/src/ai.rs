// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl.html

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anki_proto::notes::AddNoteRequest;
use anki_proto::notes::AddNoteResponse;
use anki_proto::notes::Note;
use anki_proto::notetypes::Notetype;
use anki_proto::notetypes::NotetypeId;
use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use axum_client_ip::ClientIp;
use prost::Message;
use serde::Deserialize;
use serde::Serialize;
use tracing::warn;

use crate::auth::authenticate_request;
use crate::auth::has_valid_api_token;
use crate::rpc::call_rslib_method;
use crate::AppState;

fn text_err(status: StatusCode, msg: impl Into<String>) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(msg.into()))
        .unwrap()
}

// ============================================================================
// AI Quota & Abuse Management (Per-IP rate limits & global server key limits)
// ============================================================================

#[derive(Debug)]
pub struct AiQuotaManager {
    ip_rate_buckets: Mutex<HashMap<String, (f64, Instant)>>,
    ip_daily_usage: Mutex<HashMap<String, (u64, u32)>>,
    global_daily_usage: Mutex<(u64, u32)>,
    global_daily_limit: u32,
    ip_daily_limit: u32,
    server_api_key: Option<String>,
}

impl AiQuotaManager {
    pub fn new(server_api_key: Option<String>, global_daily_limit: u32, ip_daily_limit: u32) -> Self {
        Self {
            ip_rate_buckets: Mutex::new(HashMap::new()),
            ip_daily_usage: Mutex::new(HashMap::new()),
            global_daily_usage: Mutex::new((0, 0)),
            global_daily_limit,
            ip_daily_limit,
            server_api_key,
        }
    }

    /// Rate limits AI requests per IP: max 10 requests / minute (burst capacity 10, refill 0.166/sec)
    pub fn check_rate_limit(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut guard = self.ip_rate_buckets.lock().unwrap();
        if guard.len() > 2000 {
            guard.retain(|_, (_, last)| now.duration_since(*last).as_secs() < 300);
        }
        let entry = guard.entry(ip.to_string()).or_insert((10.0, now));
        let elapsed = now.duration_since(entry.1).as_secs_f64();
        entry.0 = (entry.0 + elapsed * (10.0 / 60.0)).min(10.0);
        entry.1 = now;
        if entry.0 >= 1.0 {
            entry.0 -= 1.0;
            true
        } else {
            false
        }
    }

    /// Verifies and consumes a server API key quota for the given IP address.
    /// Returns the resolved API key, or an error response.
    pub fn acquire_server_key(&self, ip: &str) -> Result<String, (StatusCode, String)> {
        let Some(ref key) = self.server_api_key else {
            return Err((
                StatusCode::BAD_REQUEST,
                "API key is required. Please provide your own Google Gemini or OpenAI API key in the AI Generator modal, or use local Ollama.".to_string(),
            ));
        };

        let current_day = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 86400;

        // Check global daily usage
        {
            let mut global = self.global_daily_usage.lock().unwrap();
            if global.0 != current_day {
                *global = (current_day, 0);
            }
            if global.1 >= self.global_daily_limit {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    format!(
                        "Cardly server AI daily limit reached (max {} requests/day). Please enter your own Gemini or OpenAI API key to continue.",
                        self.global_daily_limit
                    ),
                ));
            }
            global.1 += 1;
        }

        // Check per-IP daily usage
        {
            let mut ip_map = self.ip_daily_usage.lock().unwrap();
            if ip_map.len() > 5000 {
                ip_map.retain(|_, (day, _)| *day == current_day);
            }
            let entry = ip_map.entry(ip.to_string()).or_insert((current_day, 0));
            if entry.0 != current_day {
                *entry = (current_day, 0);
            }
            if entry.1 >= self.ip_daily_limit {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    format!(
                        "Your IP has reached its daily limit for server-provided AI generation (max {} requests/day). Please enter your own Gemini or OpenAI API key to continue.",
                        self.ip_daily_limit
                    ),
                ));
            }
            entry.1 += 1;
        }

        Ok(key.clone())
    }
}

// ============================================================================
// Data Transfer Objects
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AiGenerateRequest {
    pub text: Option<String>,
    pub image_base64: Option<String>,
    pub image_mime: Option<String>,
    #[serde(default = "default_mode")]
    pub mode: String, // "basic" | "cloze"
    pub target_count: Option<usize>,
    pub language: Option<String>,
    #[serde(default = "default_provider")]
    pub provider: String, // "gemini" | "openai"
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

fn default_mode() -> String {
    "basic".to_string()
}

fn default_provider() -> String {
    "gemini".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCard {
    #[serde(default)]
    pub front: String,
    #[serde(default)]
    pub back: String,
    #[serde(default)]
    pub cloze: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiGenerateResponse {
    pub cards: Vec<GeneratedCard>,
}

#[derive(Debug, Deserialize)]
pub struct BatchAddCardInput {
    pub front: Option<String>,
    pub back: Option<String>,
    pub text: Option<String>,
    pub cloze: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AiBatchAddRequest {
    pub deck_id: i64,
    pub notetype_id: i64,
    pub cards: Vec<BatchAddCardInput>,
}

#[derive(Debug, Serialize)]
pub struct AiBatchAddResponse {
    pub added_count: usize,
    pub note_ids: Vec<i64>,
}

// ============================================================================
// Mock Generation for Offline & Integration Testing
// ============================================================================

fn generate_mock_cards(mode: &str, target_count: usize, language: Option<&str>) -> Vec<GeneratedCard> {
    let lang = language.unwrap_or("en").to_lowercase();
    let is_cloze = mode.eq_ignore_ascii_case("cloze");

    let mut cards = if is_cloze {
        if lang.starts_with("ru") {
            vec![
                GeneratedCard {
                    front: String::new(),
                    back: String::new(),
                    cloze: "Интервальное повторение использует {{c1::увеличивающиеся интервалы}} для надежного запоминания.".to_string(),
                    tags: vec!["ai-generated".to_string(), "обучение".to_string()],
                },
                GeneratedCard {
                    front: String::new(),
                    back: String::new(),
                    cloze: "Алгоритм {{c1::FSRS}} раздельно оценивает стабильность и {{c2::извлекаемость}} памяти.".to_string(),
                    tags: vec!["ai-generated".to_string(), "fsrs".to_string()],
                },
                GeneratedCard {
                    front: String::new(),
                    back: String::new(),
                    cloze: "Медиафайлы Anki хранятся в каталоге {{c1::collection.media}}.".to_string(),
                    tags: vec!["ai-generated".to_string(), "anki".to_string()],
                },
            ]
        } else if lang.starts_with("kk") {
            vec![
                GeneratedCard {
                    front: String::new(),
                    back: String::new(),
                    cloze: "Интервалды қайталау жүйесі есте сақтау үшін {{c1::ұзарып отыратын уақыт аралығын}} қолданады.".to_string(),
                    tags: vec!["ai-generated".to_string(), "жады".to_string()],
                },
                GeneratedCard {
                    front: String::new(),
                    back: String::new(),
                    cloze: "{{c1::FSRS}} алгоритмі есте сақтаудың тұрақтылығы мен {{c2::қайтарымдылығын}} есептейді.".to_string(),
                    tags: vec!["ai-generated".to_string(), "fsrs".to_string()],
                },
            ]
        } else {
            vec![
                GeneratedCard {
                    front: String::new(),
                    back: String::new(),
                    cloze: "Spaced repetition uses {{c1::increasing intervals}} between reviews to optimize memory retention.".to_string(),
                    tags: vec!["ai-generated".to_string(), "memory".to_string()],
                },
                GeneratedCard {
                    front: String::new(),
                    back: String::new(),
                    cloze: "The {{c1::FSRS}} algorithm models memory {{c2::stability}} and {{c3::retrievability}} separately.".to_string(),
                    tags: vec!["ai-generated".to_string(), "fsrs".to_string()],
                },
                GeneratedCard {
                    front: String::new(),
                    back: String::new(),
                    cloze: "Anki stores media files inside the {{c1::collection.media}} directory.".to_string(),
                    tags: vec!["ai-generated".to_string(), "storage".to_string()],
                },
            ]
        }
    } else {
        if lang.starts_with("ru") {
            vec![
                GeneratedCard {
                    front: "Что такое интервальное повторение?".to_string(),
                    back: "Метод обучения, при котором материал повторяется через возрастающие промежутки времени.".to_string(),
                    cloze: String::new(),
                    tags: vec!["ai-generated".to_string(), "обучение".to_string()],
                },
                GeneratedCard {
                    front: "Каково ключевое преимущество FSRS перед SM-2?".to_string(),
                    back: "FSRS точнее прогнозирует вероятность вспоминания и сокращает число повторений на 20-30%.".to_string(),
                    cloze: String::new(),
                    tags: vec!["ai-generated".to_string(), "fsrs".to_string()],
                },
                GeneratedCard {
                    front: "В каком каталоге Anki хранятся медиафайлы?".to_string(),
                    back: "collection.media".to_string(),
                    cloze: String::new(),
                    tags: vec!["ai-generated".to_string(), "anki".to_string()],
                },
            ]
        } else if lang.starts_with("kk") {
            vec![
                GeneratedCard {
                    front: "Интервалды қайталау деген не?".to_string(),
                    back: "Ақпаратты ұмытып қалмау үшін қайталау аралығын біртіндеп ұзартатын әдіс.".to_string(),
                    cloze: String::new(),
                    tags: vec!["ai-generated".to_string(), "жады".to_string()],
                },
                GeneratedCard {
                    front: "FSRS алгоритмінің артықшылығы неде?".to_string(),
                    back: "Ол жадының тұрақтылығын жоғары дәлдікпен бағалап, артық қайталауларды азайтады.".to_string(),
                    cloze: String::new(),
                    tags: vec!["ai-generated".to_string(), "fsrs".to_string()],
                },
            ]
        } else {
            vec![
                GeneratedCard {
                    front: "What is spaced repetition?".to_string(),
                    back: "A learning technique that incorporates increasing intervals of time between subsequent review of previously learned material.".to_string(),
                    cloze: String::new(),
                    tags: vec!["ai-generated".to_string(), "memory".to_string()],
                },
                GeneratedCard {
                    front: "What is the primary advantage of the FSRS algorithm over SM-2?".to_string(),
                    back: "FSRS models memory stability and retrievability separately, achieving higher retention with fewer reviews.".to_string(),
                    cloze: String::new(),
                    tags: vec!["ai-generated".to_string(), "fsrs".to_string()],
                },
                GeneratedCard {
                    front: "Where does Anki store images and audio for notes?".to_string(),
                    back: "Inside the collection.media directory within the user collection folder.".to_string(),
                    cloze: String::new(),
                    tags: vec!["ai-generated".to_string(), "storage".to_string()],
                },
            ]
        }
    };

    if target_count > 0 && target_count < cards.len() {
        cards.truncate(target_count);
    }
    cards
}

fn parse_llm_cards_json(raw: &str) -> Result<Vec<GeneratedCard>, String> {
    let trimmed = raw.trim();
    let unquoted = if trimmed.starts_with("```") {
        let lines: Vec<&str> = trimmed.lines().collect();
        if lines.len() >= 2 {
            let inner = &lines[1..lines.len() - 1];
            inner.join("\n")
        } else {
            trimmed.to_string()
        }
    } else {
        trimmed.to_string()
    };

    // 1. Check { "cards": [...] } or { "flashcards": [...] }
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&unquoted) {
        if let Some(arr) = val
            .get("cards")
            .or_else(|| val.get("flashcards"))
            .and_then(|v| v.as_array())
        {
            if let Ok(cards) =
                serde_json::from_value::<Vec<GeneratedCard>>(serde_json::Value::Array(arr.clone()))
            {
                return Ok(cards);
            }
        }
        // 2. Check direct array [...]
        if let Ok(cards) = serde_json::from_value::<Vec<GeneratedCard>>(val) {
            return Ok(cards);
        }
    }

    Err(format!(
        "Could not parse valid card array from LLM response: {raw}"
    ))
}

fn build_prompts(req: &AiGenerateRequest, target_count: usize) -> (String, String) {
    let lang = req
        .language
        .as_deref()
        .unwrap_or("the same language as input text");
    let is_cloze = req.mode.eq_ignore_ascii_case("cloze");

    let system_instruction = if is_cloze {
        format!(
            "You are an expert spaced repetition flashcard author for Cardly (Anki-compatible). \
Generate exactly {target_count} high-retention atomic cloze deletion flashcards. \
Guidelines: \
1. Use standard Anki cloze syntax: {{{{c1::hidden text}}}}, {{{{c2::hidden text}}}}. \
2. Cloze deletions MUST hide key terms, definitions, formulas, or numbers, leaving sufficient context to answer. \
3. Keep each card focused on ONE fact (Minimum Information Principle). \
4. Output in {lang}. \
5. Provide 1 to 3 relevant lowercase tags per card. \
Return ONLY a valid JSON object matching this schema: \
{{\"cards\": [{{\"cloze\": \"Sentence with {{{{c1::cloze deletion}}}}\", \"tags\": [\"tag1\", \"tag2\"]}}]}}"
        )
    } else {
        format!(
            "You are an expert spaced repetition flashcard author for Cardly (Anki-compatible). \
Generate exactly {target_count} high-retention atomic question-and-answer flashcards. \
Guidelines: \
1. 'front': A clear, unambiguous question or prompt. \
2. 'back': A concise, accurate, direct answer. \
3. Keep each card focused on ONE concept (Minimum Information Principle). Avoid long paragraphs. \
4. Output in {lang}. \
5. Provide 1 to 3 relevant lowercase tags per card. \
Return ONLY a valid JSON object matching this schema: \
{{\"cards\": [{{\"front\": \"Question\", \"back\": \"Answer\", \"tags\": [\"tag1\", \"tag2\"]}}]}}"
        )
    };

    let user_prompt = match &req.text {
        Some(t) if !t.trim().is_empty() => format!("Source material:\n```\n{}\n```", t.trim()),
        _ => "Generate cards from the attached image.".to_string(),
    };

    (system_instruction, user_prompt)
}

// ============================================================================
// External Provider Integrations (Gemini & OpenAI)
// ============================================================================

async fn call_gemini(
    client: &reqwest::Client,
    req: &AiGenerateRequest,
    api_key: &str,
    system_instruction: &str,
    user_prompt: &str,
) -> Result<Vec<GeneratedCard>, Response> {
    let model = req.model.as_deref().unwrap_or("gemini-1.5-flash");
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let mut parts = Vec::new();
    if let (Some(b64), Some(mime)) = (&req.image_base64, &req.image_mime) {
        if !b64.is_empty() {
            parts.push(serde_json::json!({
                "inline_data": {
                    "mime_type": mime,
                    "data": b64
                }
            }));
        }
    }
    let combined_prompt = format!("{}\n\n{}", system_instruction, user_prompt);
    parts.push(serde_json::json!({
        "text": combined_prompt
    }));

    let request_body = serde_json::json!({
        "contents": [
            {
                "parts": parts
            }
        ],
        "generationConfig": {
            "response_mime_type": "application/json",
            "temperature": 0.2
        }
    });

    let resp = match client
        .post(&url)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return Err(text_err(
                StatusCode::BAD_GATEWAY,
                format!("Failed to reach Gemini API: {e}"),
            ))
        }
    };

    let status = resp.status();
    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            return Err(text_err(
                StatusCode::BAD_GATEWAY,
                format!("Failed reading Gemini response: {e}"),
            ))
        }
    };

    if !status.is_success() {
        return Err(text_err(
            StatusCode::BAD_GATEWAY,
            format!("Gemini API error ({status}): {text}"),
        ));
    }

    let val: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return Err(text_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed parsing Gemini JSON envelope: {e}"),
            ))
        }
    };

    let card_text = val["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("");

    match parse_llm_cards_json(card_text) {
        Ok(cards) => Ok(cards),
        Err(e) => Err(text_err(StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

async fn call_openai(
    client: &reqwest::Client,
    req: &AiGenerateRequest,
    api_key: &str,
    system_instruction: &str,
    user_prompt: &str,
) -> Result<Vec<GeneratedCard>, Response> {
    let raw_base = req
        .base_url
        .as_deref()
        .unwrap_or("https://api.openai.com/v1");
    let trimmed = raw_base.trim_end_matches('/');
    let url = if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    };

    let model = req.model.as_deref().unwrap_or("gpt-4o-mini");

    let user_content = if let (Some(b64), Some(mime)) = (&req.image_base64, &req.image_mime) {
        if !b64.is_empty() {
            serde_json::json!([
                { "type": "text", "text": user_prompt },
                { "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{b64}") } }
            ])
        } else {
            serde_json::json!(user_prompt)
        }
    } else {
        serde_json::json!(user_prompt)
    };

    let request_body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_instruction },
            { "role": "user", "content": user_content }
        ],
        "response_format": { "type": "json_object" },
        "temperature": 0.2
    });

    let resp = match client
        .post(&url)
        .header(header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return Err(text_err(
                StatusCode::BAD_GATEWAY,
                format!("Failed to reach OpenAI API: {e}"),
            ))
        }
    };

    let status = resp.status();
    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            return Err(text_err(
                StatusCode::BAD_GATEWAY,
                format!("Failed reading OpenAI response: {e}"),
            ))
        }
    };

    if !status.is_success() {
        return Err(text_err(
            StatusCode::BAD_GATEWAY,
            format!("OpenAI API error ({status}): {text}"),
        ));
    }

    let val: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return Err(text_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed parsing OpenAI JSON envelope: {e}"),
            ))
        }
    };

    let card_text = val["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    match parse_llm_cards_json(card_text) {
        Ok(cards) => Ok(cards),
        Err(e) => Err(text_err(StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/ai/generate
/// Generates atomic, high-retention flashcards from text or multimodal image input.
pub async fn ai_generate_handler(
    State(state): State<Arc<AppState>>,
    ClientIp(client_ip): ClientIp,
    headers: HeaderMap,
    Json(req): Json<AiGenerateRequest>,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return text_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    if !has_valid_api_token(&headers, &claims) {
        return text_err(StatusCode::FORBIDDEN, "Missing or invalid Cardly API token");
    }

    let is_demo = state.config.demo_mode
        && state
            .config
            .demo_user
            .as_deref()
            .map(|u| u == claims.u)
            .unwrap_or(false);

    if let Err((status, msg)) =
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/ai/generate", Some(client_ip))
    {
        return text_err(status, msg);
    }

    // Per-IP rate limiting (10 requests/minute to protect server resources)
    let ip_str = client_ip.to_string();
    if !state.ai_quota.check_rate_limit(&ip_str) {
        return text_err(
            StatusCode::TOO_MANY_REQUESTS,
            "AI generation rate limit exceeded (maximum 10 requests per minute). Please wait a moment before trying again.",
        );
    }

    // Input payload size constraints
    if let Some(ref text) = req.text {
        if text.len() > 100_000 {
            return text_err(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Input text exceeds maximum allowed size (100 KB)",
            );
        }
    }
    if let Some(ref b64) = req.image_base64 {
        if b64.len() > 15 * 1024 * 1024 {
            return text_err(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Input image exceeds maximum allowed size (10 MB)",
            );
        }
    }

    if req.text.as_ref().map(|t| t.trim().is_empty()).unwrap_or(true)
        && req
            .image_base64
            .as_ref()
            .map(|b| b.trim().is_empty())
            .unwrap_or(true)
    {
        return text_err(
            StatusCode::BAD_REQUEST,
            "Either text or image_base64 must be provided",
        );
    }

    let target_count = req.target_count.unwrap_or(3).clamp(1, 20);

    let is_mock = req
        .api_key
        .as_deref()
        .map(|k| k == "test-mock-key" || k == "mock")
        .unwrap_or(false)
        || std::env::var("CARDLY_TEST_MOCK_AI").is_ok();

    if is_mock {
        let cards = generate_mock_cards(&req.mode, target_count, req.language.as_deref());
        return (StatusCode::OK, Json(AiGenerateResponse { cards })).into_response();
    }

    // Resolve API key: either client-provided key or server-configured fallback with strict quotas
    let api_key = match req
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty() && *k != "server")
    {
        Some(user_key) => user_key.to_string(),
        None => match state.ai_quota.acquire_server_key(&ip_str) {
            Ok(server_key) => server_key,
            Err((status, msg)) => return text_err(status, msg),
        },
    };

    let (system_instruction, user_prompt) = build_prompts(&req, target_count);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap_or_default();

    let provider = req.provider.to_lowercase();
    let result = if provider == "openai" {
        call_openai(&client, &req, &api_key, &system_instruction, &user_prompt).await
    } else {
        call_gemini(&client, &req, &api_key, &system_instruction, &user_prompt).await
    };

    match result {
        Ok(cards) => (StatusCode::OK, Json(AiGenerateResponse { cards })).into_response(),
        Err(err_resp) => err_resp,
    }
}

/// POST /api/ai/batch-add
/// Adds generated cards to the user's collection in a single batch with full undo grouping.
pub async fn ai_batch_add_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AiBatchAddRequest>,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return text_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    if !has_valid_api_token(&headers, &claims) {
        return text_err(StatusCode::FORBIDDEN, "Missing or invalid Cardly API token");
    }

    if req.cards.is_empty() {
        return (
            StatusCode::OK,
            Json(AiBatchAddResponse {
                added_count: 0,
                note_ids: vec![],
            }),
        )
            .into_response();
    }

    let is_demo = state.config.demo_mode
        && state
            .config
            .demo_user
            .as_deref()
            .map(|u| u == claims.u)
            .unwrap_or(false);

    if let Err((status, msg)) =
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/ai/batch-add", None)
    {
        return text_err(status, msg);
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(r) => r,
        Err(e) => {
            return text_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to access user collection: {e}"),
            )
        }
    };

    // Query notetype to inspect field count
    let nt_id_req = NotetypeId {
        ntid: req.notetype_id,
    };
    let field_count = match call_rslib_method(
        &state,
        &runtime,
        "getNotetype",
        nt_id_req.encode_to_vec(),
    )
    .await
    {
        Ok(bytes) => Notetype::decode(bytes.as_slice())
            .map(|nt| nt.fields.len())
            .unwrap_or(2),
        Err(_) => 2,
    };

    // Group the batch in Anki undo log
    let undo_entry = anki_proto::generic::String {
        val: "Add AI Cards".to_string(),
    };
    let undo_id = match call_rslib_method(
        &state,
        &runtime,
        "addCustomUndoEntry",
        undo_entry.encode_to_vec(),
    )
    .await
    {
        Ok(bytes) => anki_proto::generic::UInt32::decode(bytes.as_slice())
            .map(|u| u.val)
            .unwrap_or(0),
        Err(_) => 0,
    };

    let mut note_ids = Vec::with_capacity(req.cards.len());
    for card in req.cards {
        let mut fields = vec![String::new(); field_count.max(2)];
        if let Some(cloze) = card.cloze.or(card.text) {
            if !cloze.trim().is_empty() {
                fields[0] = cloze;
                if let Some(back) = card.back {
                    if fields.len() > 1 {
                        fields[1] = back;
                    }
                }
            } else {
                if let Some(front) = card.front {
                    fields[0] = front;
                }
                if let Some(back) = card.back {
                    if fields.len() > 1 {
                        fields[1] = back;
                    }
                }
            }
        } else {
            if let Some(front) = card.front {
                fields[0] = front;
            }
            if let Some(back) = card.back {
                if fields.len() > 1 {
                    fields[1] = back;
                }
            }
        }

        let add_req = AddNoteRequest {
            note: Some(Note {
                id: 0,
                guid: String::new(),
                notetype_id: req.notetype_id,
                mtime_secs: 0,
                usn: 0,
                tags: card.tags,
                fields,
            }),
            deck_id: req.deck_id,
        };

        let res = call_rslib_method(&state, &runtime, "addNote", add_req.encode_to_vec()).await;
        match res {
            Ok(bytes) => match AddNoteResponse::decode(bytes.as_slice()) {
                Ok(resp) => {
                    note_ids.push(resp.note_id);
                }
                Err(e) => {
                    warn!("AddNoteResponse::decode error: {e:?}");
                }
            },
            Err((s, m)) => {
                warn!("addNote failed: status {s:?}, message {m:?}");
            }
        }
    }

    // Merge undo entries so the entire batch can be undone with a single Ctrl+Z
    if undo_id > 0 && !note_ids.is_empty() {
        let merge_req = anki_proto::generic::UInt32 { val: undo_id };
        let _ =
            call_rslib_method(&state, &runtime, "mergeUndoEntries", merge_req.encode_to_vec()).await;
    }

    (
        StatusCode::OK,
        Json(AiBatchAddResponse {
            added_count: note_ids.len(),
            note_ids,
        }),
    )
        .into_response()
}
