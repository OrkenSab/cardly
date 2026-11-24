// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

use crate::config::UserCredentials;
use crate::AppState;

pub const SESSION_COOKIE_NAME: &str = "cardly_session";
pub const API_TOKEN_HEADER: &str = "x-cardly-api-token";
const SESSION_TTL_SECS: u64 = 60 * 60 * 24 * 30; // 30 days

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    pub u: String,
    pub exp: u64,
    pub tok: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthMeResponse {
    pub authenticated: bool,
    pub username: String,
    pub api_token: String,
    pub demo_mode: bool,
    pub is_demo_user: bool,
}

/// RFC 2104 HMAC-SHA256 using sha2::Sha256 (avoids crate version mismatch with digest 0.11).
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut key_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let hashed = Sha256::digest(key);
        key_block[..32].copy_from_slice(&hashed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    outer.finalize().into()
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn sign_session(secret: &[u8], username: &str) -> (String, SessionClaims) {
    let mut token_bytes = [0u8; 32];
    rand::fill(&mut token_bytes);
    let claims = SessionClaims {
        u: username.to_string(),
        exp: now_unix_secs() + SESSION_TTL_SECS,
        tok: hex::encode(token_bytes),
    };
    let payload_json = serde_json::to_vec(&claims).expect("serialize session claims");
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json);
    let sig = hex::encode(hmac_sha256(secret, payload_b64.as_bytes()));
    (format!("{payload_b64}.{sig}"), claims)
}

pub fn verify_session(secret: &[u8], cookie_val: &str) -> Option<SessionClaims> {
    let (payload_b64, sig_hex) = cookie_val.split_once('.')?;
    let provided_sig = hex::decode(sig_hex).ok()?;
    let expected_sig = hmac_sha256(secret, payload_b64.as_bytes());
    if !constant_time_eq(&provided_sig, &expected_sig) {
        return None;
    }
    let payload_bytes = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let claims: SessionClaims = serde_json::from_slice(&payload_bytes).ok()?;
    if claims.exp < now_unix_secs() {
        return None;
    }
    Some(claims)
}

pub fn extract_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    for value in headers.get_all(header::COOKIE) {
        let Ok(cookie_str) = value.to_str() else {
            continue;
        };
        for pair in cookie_str.split(';') {
            let pair = pair.trim();
            if let Some((k, v)) = pair.split_once('=') {
                if k.trim() == name {
                    return Some(v.trim());
                }
            }
        }
    }
    None
}

pub fn authenticate_request(state: &AppState, headers: &HeaderMap) -> Option<SessionClaims> {
    let cookie = extract_cookie(headers, SESSION_COOKIE_NAME)?;
    let claims = verify_session(&state.config.session_secret, cookie)?;
    // Ensure user exists in dynamic user registry or config
    if !state.user_registry.has_user(&claims.u) && !state.config.users_by_name.contains_key(&claims.u) {
        return None;
    }
    Some(claims)
}

/// Checks per-session API token header (either `X-Cardly-Api-Token` or `Authorization: Bearer ...`)
/// to prevent untrusted card JS or cross-site forms from invoking collection mutations.
pub fn has_valid_api_token(headers: &HeaderMap, claims: &SessionClaims) -> bool {
    if let Some(val) = headers.get(API_TOKEN_HEADER).and_then(|v| v.to_str().ok()) {
        if constant_time_eq(val.trim().as_bytes(), claims.tok.as_bytes()) {
            return true;
        }
    }
    if let Some(auth) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            if constant_time_eq(token.trim().as_bytes(), claims.tok.as_bytes()) {
                return true;
            }
        }
    }
    false
}

/// Constant-time credential verification via user registry.
pub fn verify_credentials(
    state: &AppState,
    username: &str,
    password: &str,
) -> Option<UserCredentials> {
    state.user_registry.verify_credentials(username, password)
}

fn build_session_cookie(value: &str, max_age: i64, secure: bool) -> String {
    let mut cookie = format!(
        "{SESSION_COOKIE_NAME}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}"
    );
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Response {
    let Some(user) = verify_credentials(&state, req.username.trim(), &req.password) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid username or password" })),
        )
            .into_response();
    };

    let (signed, claims) = sign_session(&state.config.session_secret, &user.username);
    let cookie = build_session_cookie(&signed, SESSION_TTL_SECS as i64, state.config.cookie_secure);

    let is_demo = state.config.demo_mode
        && (state
            .config
            .demo_user
            .as_deref()
            .map(|u| u == user.username)
            .unwrap_or(false)
            || state.user_registry.is_guest(&user.username));

    let mut resp = Json(AuthMeResponse {
        authenticated: true,
        username: user.username.clone(),
        api_token: claims.tok,
        demo_mode: state.config.demo_mode,
        is_demo_user: is_demo,
    })
    .into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    resp
}

pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    let clean_user = req.username.trim();
    let creds = match state.user_registry.register(clean_user, &req.password) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Initialize user runtime and verify collection directory
    let runtime = match state.users.get_or_init(&creds.username) {
        Ok(rt) => rt,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to initialize collection: {e}") })),
            )
                .into_response();
        }
    };

    // Ensure collection.anki2 is cleanly created
    let _ = runtime.acquire_web_backend().await;

    let (signed, claims) = sign_session(&state.config.session_secret, &creds.username);
    let cookie = build_session_cookie(&signed, SESSION_TTL_SECS as i64, state.config.cookie_secure);

    let mut resp = Json(AuthMeResponse {
        authenticated: true,
        username: creds.username,
        api_token: claims.tok,
        demo_mode: state.config.demo_mode,
        is_demo_user: false,
    })
    .into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    resp
}

pub async fn guest_handler(State(state): State<Arc<AppState>>) -> Response {
    if !state.config.demo_mode {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Guest demo access is disabled" })),
        )
            .into_response();
    }

    let guest_creds = match state.user_registry.register_ephemeral_guest() {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to create guest session: {e}") })),
            )
                .into_response();
        }
    };

    let runtime = match state.users.get_or_init(&guest_creds.username) {
        Ok(rt) => rt,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Seed sample decks for this guest so they have full demo experience
    let _ = crate::demo::seed_demo_collection_if_empty(&state, &runtime).await;

    let (signed, claims) = sign_session(&state.config.session_secret, &guest_creds.username);
    let cookie = build_session_cookie(&signed, SESSION_TTL_SECS as i64, state.config.cookie_secure);

    let mut resp = Json(AuthMeResponse {
        authenticated: true,
        username: guest_creds.username,
        api_token: claims.tok,
        demo_mode: true,
        is_demo_user: true,
    })
    .into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    resp
}

pub async fn logout_handler(State(state): State<Arc<AppState>>) -> Response {
    let cookie = build_session_cookie("", 0, state.config.cookie_secure);
    let mut resp = Json(serde_json::json!({ "ok": true })).into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    resp
}

pub async fn me_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let maybe_claims = authenticate_request(&state, &headers);
    let claims = match maybe_claims {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "authenticated": false,
                    "demo_mode": state.config.demo_mode
                })),
            )
                .into_response();
        }
    };

    let is_demo = state.config.demo_mode
        && (state
            .config
            .demo_user
            .as_deref()
            .map(|u| u == claims.u)
            .unwrap_or(false)
            || state.user_registry.is_guest(&claims.u));

    Json(AuthMeResponse {
        authenticated: true,
        username: claims.u,
        api_token: claims.tok,
        demo_mode: state.config.demo_mode,
        is_demo_user: is_demo,
    })
    .into_response()
}
