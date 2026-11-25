// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl.html

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use axum_client_ip::ClientIp;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::auth::authenticate_request;
use crate::auth::has_valid_api_token;
use crate::user_runtime::UserRuntime;
use crate::AppState;

/// Maximum serialized payload/code size for a single external plugin (2 MiB).
pub const MAX_PLUGIN_BYTES: usize = 2 * 1024 * 1024;
/// Maximum external plugins allowed per user.
pub const MAX_EXTERNAL_PLUGINS: usize = 25;

/// Allowed runtime permissions in plugin manifests.
pub const ALLOWED_PERMISSIONS: &[&str] = &[
    "ui:decks",
    "ui:reviewer",
    "events:cards",
    "events:app",
    "stats:read",
    "storage",
    "app:read",
];

pub fn validate_permissions(val: Option<&Value>) -> Result<Vec<String>, String> {
    let Some(raw) = val else {
        return Ok(Vec::new());
    };
    let Some(arr) = raw.as_array() else {
        return Err("manifest.permissions must be an array of strings".to_string());
    };
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(perm) = item.as_str() else {
            return Err("manifest.permissions entries must be strings".to_string());
        };
        if !ALLOWED_PERMISSIONS.contains(&perm) {
            return Err(format!("Unknown plugin permission \"{perm}\""));
        }
        if !out.iter().any(|existing| existing == perm) {
            out.push(perm.to_string());
        }
    }
    Ok(out)
}

fn json_err(status: StatusCode, msg: impl Into<String>) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        Json(serde_json::json!({ "error": msg.into() })),
    )
        .into_response()
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Validates plugin ID strictly against `^[a-z0-9][a-z0-9-]{1,62}$` (total 2..=63 chars).
/// Rejects path traversal (`..`, `/`, `\`), hidden files, uppercase, and special chars.
pub fn is_valid_plugin_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() < 2 || bytes.len() > 63 {
        return false;
    }
    let first = bytes[0];
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return false;
    }
    bytes
        .iter()
        .all(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredExternalPlugin {
    pub manifest: Value,
    pub code: String,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoredPluginState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Serialize)]
pub struct PluginsListResponse {
    pub plugins: Vec<StoredExternalPlugin>,
    pub states: BTreeMap<String, StoredPluginState>,
}

#[derive(Debug, Deserialize)]
pub struct PutPluginRequest {
    pub manifest: Value,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct PutPluginStateRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub config: Option<Value>,
    #[serde(default)]
    pub storage: Option<BTreeMap<String, Value>>,
}

fn external_dir(runtime: &UserRuntime) -> PathBuf {
    runtime.plugins_dir.join("external")
}

fn states_path(runtime: &UserRuntime) -> PathBuf {
    runtime.plugins_dir.join("states.json")
}

fn resolve_plugin_file(runtime: &UserRuntime, id: &str) -> Result<PathBuf, Response> {
    if !is_valid_plugin_id(id) {
        return Err(json_err(
            StatusCode::BAD_REQUEST,
            "Invalid plugin id (must match ^[a-z0-9][a-z0-9-]{1,62}$)",
        ));
    }
    let dir = external_dir(runtime);
    fs::create_dir_all(&dir).map_err(|e| json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let canon_dir = dir
        .canonicalize()
        .map_err(|e| json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let candidate = canon_dir.join(format!("{id}.json"));
    if candidate.parent() != Some(canon_dir.as_path()) {
        return Err(json_err(StatusCode::BAD_REQUEST, "Invalid plugin path"));
    }
    Ok(candidate)
}

fn read_states_map(runtime: &UserRuntime) -> BTreeMap<String, StoredPluginState> {
    let path = states_path(runtime);
    let Ok(bytes) = fs::read(path) else {
        return BTreeMap::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn write_states_map(
    runtime: &UserRuntime,
    map: &BTreeMap<String, StoredPluginState>,
) -> Result<(), Response> {
    fs::create_dir_all(&runtime.plugins_dir)
        .map_err(|e| json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let serialized = serde_json::to_vec_pretty(map)
        .map_err(|e| json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    fs::write(states_path(runtime), serialized)
        .map_err(|e| json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(())
}

fn read_external_plugins(runtime: &UserRuntime) -> Vec<StoredExternalPlugin> {
    let dir = external_dir(runtime);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        if let Ok(plugin) = serde_json::from_slice::<StoredExternalPlugin>(&bytes) {
            out.push(plugin);
        }
    }
    out.sort_by(|a, b| {
        let a_id = a.manifest.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let b_id = b.manifest.get("id").and_then(|v| v.as_str()).unwrap_or("");
        a_id.cmp(b_id)
    });
    out
}

pub async fn list_plugins_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let _guard = runtime.plugins_lock.lock().unwrap();
    let plugins = read_external_plugins(&runtime);
    let states = read_states_map(&runtime);
    Json(PluginsListResponse { plugins, states }).into_response()
}

pub async fn put_plugin_handler(
    State(state): State<Arc<AppState>>,
    ClientIp(client_ip): ClientIp,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    if !has_valid_api_token(&headers, &claims) {
        return json_err(StatusCode::FORBIDDEN, "Missing or invalid Cardly API token");
    }

    let is_demo = state.is_demo_user(&claims.u);
    if let Err((status, msg)) =
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/plugins", Some(client_ip))
    {
        return json_err(status, msg);
    }

    if body.len() > MAX_PLUGIN_BYTES {
        return json_err(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("Plugin exceeds maximum allowed size of {} bytes", MAX_PLUGIN_BYTES),
        );
    }

    let req: PutPluginRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, format!("Invalid JSON payload: {e}")),
    };

    let Some(manifest_obj) = req.manifest.as_object() else {
        return json_err(StatusCode::BAD_REQUEST, "manifest must be a JSON object");
    };
    let manifest_id = manifest_obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if manifest_id != id {
        return json_err(
            StatusCode::BAD_REQUEST,
            "manifest.id must match URL path parameter",
        );
    }
    let has_name = manifest_obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let has_version = manifest_obj
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !has_name || !has_version {
        return json_err(
            StatusCode::BAD_REQUEST,
            "manifest.name and manifest.version are required",
        );
    }
    if req.code.trim().is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "Plugin code must not be empty");
    }
    if req.code.len() > MAX_PLUGIN_BYTES {
        return json_err(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Plugin code exceeds maximum size",
        );
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    if is_demo {
        if let Err((status, msg)) = crate::demo::check_demo_storage_cap(&runtime.folder) {
            return json_err(status, msg);
        }
    }

    let _guard = runtime.plugins_lock.lock().unwrap();
    let target = match resolve_plugin_file(&runtime, &id) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    let is_new = !target.exists();
    if is_new {
        let existing_count = read_external_plugins(&runtime).len();
        if existing_count >= MAX_EXTERNAL_PLUGINS {
            return json_err(
                StatusCode::BAD_REQUEST,
                format!("Maximum of {MAX_EXTERNAL_PLUGINS} external plugins reached"),
            );
        }
    }

    let validated_permissions = match validate_permissions(manifest_obj.get("permissions")) {
        Ok(perms) => perms,
        Err(msg) => return json_err(StatusCode::BAD_REQUEST, msg),
    };

    let mut clean_manifest = manifest_obj.clone();
    clean_manifest.insert("builtIn".to_string(), Value::Bool(false));
    clean_manifest.insert(
        "permissions".to_string(),
        Value::Array(validated_permissions.into_iter().map(Value::String).collect()),
    );

    let stored = StoredExternalPlugin {
        manifest: Value::Object(clean_manifest),
        code: req.code,
        updated_at: now_unix_secs(),
    };

    let serialized = match serde_json::to_vec(&stored) {
        Ok(b) => b,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    if serialized.len() > MAX_PLUGIN_BYTES {
        return json_err(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Serialized plugin exceeds 2 MiB limit",
        );
    }

    if let Err(e) = fs::write(&target, serialized) {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    (StatusCode::OK, Json(stored)).into_response()
}

pub async fn delete_plugin_handler(
    State(state): State<Arc<AppState>>,
    ClientIp(client_ip): ClientIp,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    if !has_valid_api_token(&headers, &claims) {
        return json_err(StatusCode::FORBIDDEN, "Missing or invalid Cardly API token");
    }

    let is_demo = state.is_demo_user(&claims.u);
    if let Err((status, msg)) =
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/plugins", Some(client_ip))
    {
        return json_err(status, msg);
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let _guard = runtime.plugins_lock.lock().unwrap();
    let target = match resolve_plugin_file(&runtime, &id) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    if target.exists() {
        if let Err(e) = fs::remove_file(&target) {
            return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    }

    let mut states = read_states_map(&runtime);
    if states.remove(&id).is_some() {
        if let Err(resp) = write_states_map(&runtime, &states) {
            return resp;
        }
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"deleted":true}"#))
        .unwrap()
}

pub async fn put_plugin_state_handler(
    State(state): State<Arc<AppState>>,
    ClientIp(client_ip): ClientIp,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    if !has_valid_api_token(&headers, &claims) {
        return json_err(StatusCode::FORBIDDEN, "Missing or invalid Cardly API token");
    }
    if !is_valid_plugin_id(&id) {
        return json_err(
            StatusCode::BAD_REQUEST,
            "Invalid plugin id (must match ^[a-z0-9][a-z0-9-]{1,62}$)",
        );
    }

    let is_demo = state.is_demo_user(&claims.u);
    if let Err((status, msg)) =
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/plugins/state", Some(client_ip))
    {
        return json_err(status, msg);
    }

    if body.len() > MAX_PLUGIN_BYTES {
        return json_err(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Plugin state payload exceeds maximum allowed size",
        );
    }

    let req: PutPluginStateRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, format!("Invalid JSON payload: {e}")),
    };

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let _guard = runtime.plugins_lock.lock().unwrap();
    let mut states = read_states_map(&runtime);
    let entry = states.entry(id).or_default();

    if let Some(enabled) = req.enabled {
        entry.enabled = Some(enabled);
    }
    if let Some(config) = req.config {
        if config.is_null() {
            entry.config = None;
        } else if config.is_object() {
            entry.config = Some(config);
        } else {
            return json_err(StatusCode::BAD_REQUEST, "config must be a JSON object or null");
        }
    }
    if let Some(storage) = req.storage {
        entry.storage = Some(storage);
    }

    let saved = entry.clone();
    if let Err(resp) = write_states_map(&runtime, &states) {
        return resp;
    }

    (StatusCode::OK, Json(saved)).into_response()
}
