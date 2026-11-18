// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anki_proto::collection::OpChangesWithId;
use anki_proto::decks::Deck;
use anki_proto::decks::DeckNames;
use anki_proto::decks::GetDeckNamesRequest;
use anki_proto::notes::AddNoteRequest;
use anki_proto::notes::Note;
use anki_proto::notetypes::NotetypeNames;
use anki_proto::search::SearchRequest;
use anki_proto::search::SearchResponse;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use prost::Message;
use serde::Serialize;
use tracing::info;
use tracing::warn;

use crate::rpc::call_rslib_method;
use crate::user_runtime::UserRuntime;
use crate::AppState;

pub const DEFAULT_DEMO_USER: &str = "demo";
pub const DEFAULT_DEMO_PASS: &str = "demo123";
pub const DEFAULT_RESET_INTERVAL_SECS: u64 = 3600; // 60 minutes
pub const DEMO_MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024; // 10 MiB limit for demo accounts

static LAST_RESET_UNIX: AtomicU64 = AtomicU64::new(0);

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn update_reset_timestamp() {
    LAST_RESET_UNIX.store(now_unix_secs(), Ordering::SeqCst);
}

pub fn last_reset_timestamp() -> u64 {
    LAST_RESET_UNIX.load(Ordering::SeqCst)
}

// ============================================================================
// Rate Limiter & Abuse Protection
// ============================================================================

struct TokenBucket {
    tokens: f64,
    last_update: Instant,
}

pub struct DemoRateLimiter {
    buckets: Mutex<HashMap<String, TokenBucket>>,
    capacity: f64,
    refill_rate_per_sec: f64,
}

impl DemoRateLimiter {
    pub fn new(capacity: f64, refill_rate_per_sec: f64) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            capacity,
            refill_rate_per_sec,
        }
    }

    /// Checks if a client (identified by IP or session key) is allowed to proceed.
    pub fn check_rate_limit(&self, client_key: &str) -> bool {
        let mut guard = self.buckets.lock().unwrap();
        let now = Instant::now();

        // Evict stale entries if bucket map gets large
        if guard.len() > 2000 {
            guard.retain(|_, b| now.duration_since(b.last_update).as_secs() < 300);
        }

        let bucket = guard
            .entry(client_key.to_string())
            .or_insert_with(|| TokenBucket {
                tokens: self.capacity,
                last_update: now,
            });

        let elapsed = now.duration_since(bucket.last_update).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_rate_per_sec).min(self.capacity);
        bucket.last_update = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

use std::sync::OnceLock;

static GLOBAL_DEMO_LIMITER: OnceLock<DemoRateLimiter> = OnceLock::new();

pub fn demo_limiter() -> &'static DemoRateLimiter {
    GLOBAL_DEMO_LIMITER.get_or_init(|| DemoRateLimiter::new(30.0, 2.0))
}

/// Checks whether an action performed by the demo user violates abuse rules.
pub fn validate_demo_action(
    username: &str,
    is_demo: bool,
    path: &str,
    client_ip: Option<IpAddr>,
) -> Result<(), (StatusCode, String)> {
    if !is_demo {
        return Ok(());
    }

    // Rate limiting check
    let rate_key = format!(
        "{}:{}",
        username,
        client_ip.map(|ip| ip.to_string()).unwrap_or_else(|| "unknown".into())
    );
    if !demo_limiter().check_rate_limit(&rate_key) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded for demo session. Please wait a few seconds before trying again.".to_string(),
        ));
    }

    // Abuse protection: block destructive actions that affect collection integrity or server storage
    if path.contains("/api/backups/") && path.ends_with("/restore") {
        return Err((
            StatusCode::FORBIDDEN,
            "Backup restoration is disabled in Demo Mode to preserve sample decks.".to_string(),
        ));
    }

    if path == "/api/import/colpkg" {
        return Err((
            StatusCode::FORBIDDEN,
            "Full collection import (.colpkg) is disabled in Demo Mode. Try importing standard .apkg or .csv files instead.".to_string(),
        ));
    }

    Ok(())
}

// ============================================================================
// Sample Deck Seeding
// ============================================================================

async fn get_or_create_deck(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    deck_name: &str,
) -> Result<i64> {
    let req = GetDeckNamesRequest {
        skip_empty_default: false,
        include_filtered: true,
    };
    if let Ok(bytes) = call_rslib_method(state, runtime, "getDeckNames", req.encode_to_vec()).await {
        if let Ok(deck_names) = DeckNames::decode(bytes.as_slice()) {
            if let Some(entry) = deck_names.entries.into_iter().find(|d| d.name == deck_name) {
                return Ok(entry.id);
            }
        }
    }

    let deck = Deck {
        id: 0,
        name: deck_name.to_string(),
        mtime_secs: 0,
        usn: 0,
        common: None,
        kind: Some(anki_proto::decks::deck::Kind::Normal(
            anki_proto::decks::deck::Normal::default(),
        )),
    };
    let out = call_rslib_method(state, runtime, "addDeck", deck.encode_to_vec())
        .await
        .map_err(|(s, m)| anyhow!("addDeck failed with status {s}: {m}"))?;
    let changes = OpChangesWithId::decode(out.as_slice())
        .context("decode OpChangesWithId")?;
    Ok(changes.id)
}

async fn add_sample_note(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    deck_id: i64,
    notetype_id: i64,
    tags: Vec<&str>,
    fields: Vec<&str>,
) -> Result<()> {
    let add_req = AddNoteRequest {
        note: Some(Note {
            id: 0,
            guid: String::new(),
            notetype_id,
            mtime_secs: 0,
            usn: 0,
            tags: tags.into_iter().map(String::from).collect(),
            fields: fields.into_iter().map(String::from).collect(),
        }),
        deck_id,
    };
    call_rslib_method(state, runtime, "addNote", add_req.encode_to_vec())
        .await
        .map_err(|(s, m)| anyhow!("addNote failed with status {s}: {m}"))?;
    Ok(())
}

pub async fn seed_demo_collection_if_empty(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
) -> Result<()> {
    let _guard = runtime.acquire_web_backend().await?;

    let search_req = SearchRequest {
        search: "".to_string(),
        order: None,
    };
    let out = call_rslib_method(state, runtime, "searchNotes", search_req.encode_to_vec())
        .await
        .map_err(|(s, m)| anyhow!("searchNotes failed ({s}): {m}"))?;
    let notes = SearchResponse::decode(out.as_slice())
        .context("decode SearchResponse")?;

    if !notes.ids.is_empty() {
        return Ok(());
    }

    info!(user = %runtime.username, "Demo collection is empty. Seeding rich sample decks...");

    // Retrieve default notetypes
    let nt_out = call_rslib_method(state, runtime, "getNotetypeNames", vec![])
        .await
        .map_err(|(s, m)| anyhow!("getNotetypeNames failed ({s}): {m}"))?;
    let nt_names = NotetypeNames::decode(nt_out.as_slice())
        .context("decode NotetypeNames")?;

    let basic_id = nt_names
        .entries
        .iter()
        .find(|e| e.name == "Basic")
        .map(|e| e.id)
        .ok_or_else(|| anyhow!("Basic notetype missing"))?;

    let cloze_id = nt_names
        .entries
        .iter()
        .find(|e| e.name == "Cloze")
        .map(|e| e.id);

    // 1. Deck: General Knowledge
    let gk_deck_id = get_or_create_deck(state, runtime, "General Knowledge").await?;
    add_sample_note(
        state,
        runtime,
        gk_deck_id,
        basic_id,
        vec!["demo", "geography"],
        vec!["What is the capital city of Japan?", "Tokyo (東京)"],
    ).await?;
    add_sample_note(
        state,
        runtime,
        gk_deck_id,
        basic_id,
        vec!["demo", "astronomy"],
        vec![
            "Which planet is known as the Red Planet?",
            "Mars<br><small>Named after the Roman god of war due to its reddish appearance from iron oxide.</small>",
        ],
    ).await?;
    add_sample_note(
        state,
        runtime,
        gk_deck_id,
        basic_id,
        vec!["demo", "history"],
        vec!["In which year did the Apollo 11 mission land humans on the Moon?", "1969 (July 20)"],
    ).await?;
    add_sample_note(
        state,
        runtime,
        gk_deck_id,
        basic_id,
        vec!["demo", "literature"],
        vec!["Who wrote the tragedy <i>Hamlet</i>?", "William Shakespeare (c. 1599–1601)"],
    ).await?;

    // 2. Deck: Languages (Spanish & Kazakh)
    let lang_deck_id = get_or_create_deck(state, runtime, "Languages::Spanish & Kazakh").await?;
    add_sample_note(
        state,
        runtime,
        lang_deck_id,
        basic_id,
        vec!["demo", "spanish"],
        vec!["¿Cómo te llamas? (Spanish)", "What is your name?"],
    ).await?;
    add_sample_note(
        state,
        runtime,
        lang_deck_id,
        basic_id,
        vec!["demo", "spanish"],
        vec!["Muchas gracias por su ayuda. (Spanish)", "Thank you very much for your help."],
    ).await?;
    add_sample_note(
        state,
        runtime,
        lang_deck_id,
        basic_id,
        vec!["demo", "kazakh"],
        vec!["Сәлеметсіз бе! (Kazakh)", "Hello / Formal Greeting<br><i>Sälemetsiz be!</i>"],
    ).await?;
    add_sample_note(
        state,
        runtime,
        lang_deck_id,
        basic_id,
        vec!["demo", "kazakh"],
        vec!["Қош келдіңіз! (Kazakh)", "Welcome!<br><i>Qosh keldiñiz!</i>"],
    ).await?;

    // 3. Deck: Science & Mathematics (Cloze + MathJax)
    let sci_deck_id = get_or_create_deck(state, runtime, "Science & Mathematics").await?;

    if let Some(cid) = cloze_id {
        add_sample_note(
            state,
            runtime,
            sci_deck_id,
            cid,
            vec!["demo", "biology", "cloze"],
            vec![
                "The biochemical process by which plants convert light energy into chemical energy is {{c1::photosynthesis}}.",
                "Takes place in chloroplasts using chlorophyll pigments.",
            ],
        ).await?;

        add_sample_note(
            state,
            runtime,
            sci_deck_id,
            cid,
            vec!["demo", "physics", "cloze", "mathjax"],
            vec![
                "In special relativity, the mass-energy equivalence is formulated as {{c1::\\( E = mc^2 \\)}}.",
                "Published by Albert Einstein in his 1905 Annus Mirabilis paper.",
            ],
        ).await?;

        add_sample_note(
            state,
            runtime,
            sci_deck_id,
            cid,
            vec!["demo", "chemistry", "cloze"],
            vec![
                "Water consists of {{c1::two}} hydrogen atoms bonded to {{c2::one}} oxygen atom, with molecular formula {{c3::\\( \\text{H}_2\\text{O} \\)}}.",
                "Forms strong hydrogen bonds responsible for its high boiling point.",
            ],
        ).await?;
    }

    add_sample_note(
        state,
        runtime,
        sci_deck_id,
        basic_id,
        vec!["demo", "math", "mathjax"],
        vec![
            "State Euler's identity connecting five fundamental constants:",
            "\\[ e^{i\\pi} + 1 = 0 \\]<br><small>Connects <i>e</i>, <i>i</i>, <i>π</i>, 1, and 0.</small>",
        ],
    ).await?;

    add_sample_note(
        state,
        runtime,
        sci_deck_id,
        basic_id,
        vec!["demo", "math", "mathjax"],
        vec![
            "Write the quadratic formula for solving \\( ax^2 + bx + c = 0 \\):",
            "\\[ x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a} \\]",
        ],
    ).await?;

    add_sample_note(
        state,
        runtime,
        sci_deck_id,
        basic_id,
        vec!["demo", "physics", "mathjax"],
        vec![
            "State Newton's Second Law of Motion:",
            "\\[ \\vec{F} = \\frac{d\\vec{p}}{dt} = m\\vec{a} \\]<br><small>Net force equals mass times acceleration.</small>",
        ],
    ).await?;

    info!(user = %runtime.username, "Demo sample decks seeded successfully!");
    Ok(())
}

// ============================================================================
// Automated Reset Routine
// ============================================================================

pub async fn reset_demo_collection(state: &AppState) -> Result<()> {
    let demo_username = state.config.demo_user.as_deref().unwrap_or(DEFAULT_DEMO_USER);
    let runtime = state.users.get_or_init(demo_username)?;

    info!(user = %demo_username, "Executing automated demo collection reset...");

    runtime
        .with_closed_collection(|_backend| {
            // Delete collection database and WAL/SHM files
            for file_name in [
                "collection.anki2",
                "collection.anki2-wal",
                "collection.anki2-shm",
                "collection.media.db2",
                "media.db",
            ] {
                let p = runtime.folder.join(file_name);
                let _ = fs::remove_file(p);
            }

            // Wipe media directory contents
            if let Ok(entries) = fs::read_dir(&runtime.media_dir) {
                for entry in entries.flatten() {
                    let _ = fs::remove_file(entry.path());
                }
            }

            // Wipe temporary directory contents
            if let Ok(entries) = fs::read_dir(&runtime.tmp_dir) {
                for entry in entries.flatten() {
                    let _ = fs::remove_file(entry.path());
                }
            }

            Ok(())
        })
        .await?;

    // Re-seed sample decks
    seed_demo_collection_if_empty(state, &runtime).await?;
    update_reset_timestamp();
    info!(user = %demo_username, "Demo collection reset and re-seeded successfully");
    Ok(())
}

pub fn start_demo_reset_task(state: Arc<AppState>, interval_secs: u64) {
    update_reset_timestamp();
    tokio::spawn(async move {
        info!(
            interval_minutes = interval_secs / 60,
            "Started background demo reset scheduler"
        );
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        // The first tick completes immediately; skip it so we don't reset right at startup.
        interval.tick().await;

        loop {
            interval.tick().await;
            if let Err(err) = reset_demo_collection(&state).await {
                warn!(error = ?err, "Failed to execute periodic demo collection reset");
            }
        }
    });
}

// ============================================================================
// Handlers & Status API
// ============================================================================

#[derive(Debug, Serialize)]
pub struct DemoStatusResponse {
    pub demo_mode: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_interval_minutes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_in_seconds: Option<u64>,
}

pub async fn demo_status_handler(State(state): State<Arc<AppState>>) -> Response {
    if !state.config.demo_mode {
        return Json(DemoStatusResponse {
            demo_mode: false,
            demo_username: None,
            reset_interval_minutes: None,
            resets_in_seconds: None,
        })
        .into_response();
    }

    let interval = state
        .config
        .demo_reset_interval_secs
        .unwrap_or(DEFAULT_RESET_INTERVAL_SECS);
    let last_reset = last_reset_timestamp();
    let now = now_unix_secs();
    let elapsed = if last_reset == 0 {
        0
    } else {
        now.saturating_sub(last_reset)
    };
    let resets_in = interval.saturating_sub(elapsed);

    Json(DemoStatusResponse {
        demo_mode: true,
        demo_username: state.config.demo_user.clone(),
        reset_interval_minutes: Some(interval / 60),
        resets_in_seconds: Some(resets_in),
    })
    .into_response()
}
