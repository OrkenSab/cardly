// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use anki_proto::backend::BackendError;
use anki_proto::collection::CreateBackupRequest;
use anki_proto::generic;
use anki_proto::import_export::CsvMetadataRequest;
use anki_proto::import_export::ImportAnkiPackageRequest;
use anki_proto::import_export::ImportCsvRequest;
use anki_proto::media::AddMediaFromPathRequest;
use anki_proto::media::AddMediaFromUrlResponse;
use anki_proto::media::TrashMediaFilesRequest;
use anki_proto::search::BrowserRow;
use anki_proto_gen::get_services;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use axum_client_ip::ClientIp;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use inflections::Inflect;
use prost::Message;
use prost_reflect::DescriptorPool;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::warn;

use crate::auth::authenticate_request;
use crate::auth::has_valid_api_token;
use crate::user_runtime::UserRuntime;
use crate::AppState;

/// Embedded protobuf descriptors generated from upstream `proto/anki/*.proto`.
const DESCRIPTORS_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/anki_descriptors.bin"));

/// Protobuf messages from `proto/anki/frontend.proto` (not re-exported by `anki_proto`).
#[derive(Clone, PartialEq, prost::Message)]
pub struct ConvertPastedImageRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub data: Vec<u8>,
    #[prost(string, tag = "2")]
    pub ext: String,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ConvertPastedImageResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub data: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct SetSettingJsonRequest {
    #[prost(string, tag = "1")]
    pub key: String,
    #[prost(bytes = "vec", tag = "2")]
    pub value_json: Vec<u8>,
}

/// Endpoints that untrusted reviewer card frames may call without the privileged API token
/// (matching `qt/aqt/mediasrv.py`'s `_check_dynamic_request_permissions`).
const REVIEWER_WHITELIST: &[&str] = &[
    "i18nResources",
    "congratsInfo",
    "getSchedulingStatesWithContext",
    "setSchedulingStates",
];

/// Methods blocked from direct client invocation because they accept raw server filesystem paths
/// or control collection lifecycle.
const BLOCKED_DIRECT_METHODS: &[&str] = &[
    "openCollection",
    "closeCollection",
    "importCollectionPackage",
    "exportCollectionPackage",
    "exportAnkiPackage",
    "exportNoteCsv",
    "exportCardCsv",
    "exportDataset",
    "getAbsoluteMediaPath",
];

#[derive(Debug, Clone, Copy)]
pub struct RpcTarget {
    pub service: u32,
    pub method: u32,
}

pub struct RpcRegistry {
    by_camel_name: HashMap<String, RpcTarget>,
}

impl RpcRegistry {
    pub fn build() -> Self {
        let pool = DescriptorPool::decode(DESCRIPTORS_BYTES)
            .expect("failed to decode embedded anki_descriptors.bin");
        let (_col_services, backend_services) = get_services(&pool);

        let mut by_camel_name = HashMap::new();
        for service in &backend_services {
            if service.name == "BackendAnkidroidService" {
                continue;
            }
            for method in service.all_methods() {
                let camel = method.name.to_camel_case();
                by_camel_name.insert(
                    camel,
                    RpcTarget {
                        service: service.index as u32,
                        method: method.index as u32,
                    },
                );
            }
        }
        Self { by_camel_name }
    }

    pub fn lookup(&self, method_name: &str) -> Option<RpcTarget> {
        self.by_camel_name.get(method_name).copied()
    }

    pub fn len(&self) -> usize {
        self.by_camel_name.len()
    }
}

fn binary_response(bytes: Vec<u8>) -> Response {
    if bytes.is_empty() {
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(axum::body::Body::empty())
            .unwrap();
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/binary")
        .body(axum::body::Body::from(bytes))
        .unwrap()
}

fn text_error(status: StatusCode, msg: impl Into<String>) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(axum::body::Body::from(msg.into()))
        .unwrap()
}

fn decode_backend_error(err_bytes: &[u8]) -> String {
    match BackendError::decode(err_bytes) {
        Ok(err) if !err.message.is_empty() => err.message,
        Ok(err) => format!("Backend error (kind {})", err.kind),
        Err(_) => "Unknown backend error".to_string(),
    }
}

/// Executes a single `rslib` service method on a Tokio blocking thread.
pub async fn call_rslib_method(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    method_name: &str,
    input: Vec<u8>,
) -> Result<Vec<u8>, (StatusCode, String)> {
    let target = state
        .rpc
        .lookup(method_name)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Unknown RPC: {method_name}")))?;

    let _mode_guard = runtime
        .acquire_web_backend()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let backend = runtime.backend.clone();
    let result = tokio::task::spawn_blocking(move || {
        backend.run_service_method(target.service, target.method, &input)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("task join error: {e}")))?;

    match result {
        Ok(out) => Ok(out),
        Err(err_bytes) => Err((StatusCode::INTERNAL_SERVER_ERROR, decode_backend_error(&err_bytes))),
    }
}

/// Main `POST /_anki/{method}` handler matching `qt/aqt/mediasrv.py`'s protobuf-over-HTTP protocol.
pub async fn handle_anki_rpc(
    State(state): State<Arc<AppState>>,
    ClientIp(client_ip): ClientIp,
    Path(method): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Require authenticated session cookie.
    let Some(claims) = authenticate_request(&state, &headers) else {
        return text_error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    // Match mediasrv.py: require non-simple Content-Type (`application/binary`) to block CSRF forms.
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !content_type.starts_with("application/binary")
        && !content_type.starts_with("application/octet-stream")
    {
        return text_error(
            StatusCode::FORBIDDEN,
            "Invalid Content-Type; expected application/binary",
        );
    }

    // Require per-session API token for all non-whitelisted endpoints (blocks untrusted card JS).
    if !REVIEWER_WHITELIST.contains(&method.as_str()) && !has_valid_api_token(&headers, &claims) {
        return text_error(StatusCode::FORBIDDEN, "Missing or invalid Cardly API token");
    }

    if BLOCKED_DIRECT_METHODS.contains(&method.as_str()) {
        return text_error(
            StatusCode::FORBIDDEN,
            format!("RPC '{method}' is not directly callable over HTTP"),
        );
    }

    let is_demo = state.config.demo_mode
        && state
            .config
            .demo_user
            .as_deref()
            .map(|u| u == claims.u)
            .unwrap_or(false);

    if let Err((status, msg)) =
        crate::demo::validate_demo_action(&claims.u, is_demo, &format!("/_anki/{method}"), Some(client_ip))
    {
        return text_error(status, msg);
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    debug!(user = %claims.u, method = %method, bytes = body.len(), "POST /_anki");

    // 1. Custom FrontendService / mediasrv.py handlers implemented server-side.
    match method.as_str() {
        "getProfileConfigJson" => return handle_get_json_setting(&runtime, false, &body),
        "setProfileConfigJson" => return handle_set_json_setting(&runtime, false, &body),
        "getMetaJson" => return handle_get_json_setting(&runtime, true, &body),
        "setMetaJson" => return handle_set_json_setting(&runtime, true, &body),
        "convertPastedImage" => return handle_convert_pasted_image(&body),
        "saveCustomColours" => {
            // Browser color picker palette is persisted via setConfigJson when customized.
            return binary_response(Vec::new());
        }
        // UI lifecycle notifications (if not intercepted in browser bridge, acknowledge cleanly).
        "deckOptionsReady"
        | "deckOptionsRequireClose"
        | "importDone"
        | "importDialogRequireClose"
        | "closeAddCards"
        | "closeEditCurrent"
        | "openFieldsDialog"
        | "openCardsDialog"
        | "openMedia"
        | "showInMediaFolder"
        | "openLink"
        | "playFile"
        | "searchInBrowser" => {
            return binary_response(Vec::new());
        }
        // Path-rewriting wrappers for safe uploads & backups.
        "getCsvMetadata" => return handle_get_csv_metadata(&state, &runtime, &body).await,
        "importCsv" => return handle_import_csv(&state, &runtime, &body).await,
        "importAnkiPackage" => return handle_import_apkg(&state, &runtime, &body).await,
        "importJsonFile" => return handle_import_json_file(&state, &runtime, &body).await,
        "addMediaFromPath" => return handle_add_media_from_path(&state, &runtime, &body).await,
        "createBackup" => return handle_create_backup_rpc(&state, &runtime, &body).await,
        _ => {}
    }

    // 2. Direct `rslib` dispatch for all 250+ backend RPC methods.
    let input_vec = body.to_vec();
    match call_rslib_method(&state, &runtime, &method, input_vec.clone()).await {
        Ok(out) => {
            // Mirror media mutations into ServerMediaDatabase so AnkiDroid/Desktop sync stays in lockstep.
            mirror_media_mutation(&runtime, &method, &input_vec, &out);
            binary_response(out)
        }
        Err((_, msg)) if method == "getConfigJson" && msg.contains("No such value") => {
            // Match Qt/tslib `getColConfig(key) ?? default` behavior when an optional config key is unset.
            let mut out = Vec::new();
            generic::Json {
                json: b"null".to_vec(),
            }
            .encode(&mut out)
            .unwrap();
            binary_response(out)
        }
        Err((status, msg)) => text_error(status, msg),
    }
}

fn mirror_media_mutation(runtime: &UserRuntime, method: &str, input: &[u8], output: &[u8]) {
    match method {
        "addMediaFile" => {
            if let Ok(fname) = generic::String::decode(output) {
                if !fname.val.is_empty() {
                    runtime.media_sync.lock().unwrap().record_web_file_added(&fname.val);
                }
            }
        }
        "addMediaFromUrl" => {
            if let Ok(resp) = AddMediaFromUrlResponse::decode(output) {
                if let Some(fname) = resp.filename {
                    if !fname.is_empty() {
                        runtime.media_sync.lock().unwrap().record_web_file_added(&fname);
                    }
                }
            }
        }
        "trashMediaFiles" => {
            if let Ok(req) = TrashMediaFilesRequest::decode(input) {
                let mut store = runtime.media_sync.lock().unwrap();
                for fname in &req.fnames {
                    store.record_web_file_removed(fname);
                }
            }
        }
        "restoreTrash" => {
            runtime.media_sync.lock().unwrap().reconcile_directory();
        }
        _ => {}
    }
}

fn handle_get_json_setting(runtime: &UserRuntime, is_meta: bool, body: &[u8]) -> Response {
    let Ok(req) = generic::String::decode(body) else {
        return text_error(StatusCode::BAD_REQUEST, "Invalid generic.String protobuf");
    };
    let val = match runtime.get_json_setting(is_meta, &req.val) {
        Ok(v) => v,
        Err(e) => return text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let json_bytes = serde_json::to_vec(&val).unwrap_or_else(|_| b"null".to_vec());
    let mut out = Vec::new();
    generic::Json { json: json_bytes }.encode(&mut out).unwrap();
    binary_response(out)
}

fn handle_set_json_setting(runtime: &UserRuntime, is_meta: bool, body: &[u8]) -> Response {
    let Ok(req) = SetSettingJsonRequest::decode(body) else {
        return text_error(
            StatusCode::BAD_REQUEST,
            "Invalid SetSettingJsonRequest protobuf",
        );
    };
    let value: serde_json::Value = match serde_json::from_slice(&req.value_json) {
        Ok(v) => v,
        Err(e) => return text_error(StatusCode::BAD_REQUEST, format!("Invalid JSON value: {e}")),
    };
    if let Err(e) = runtime.set_json_setting(is_meta, &req.key, value) {
        return text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    binary_response(Vec::new())
}

fn handle_convert_pasted_image(body: &[u8]) -> Response {
    let Ok(req) = ConvertPastedImageRequest::decode(body) else {
        return text_error(
            StatusCode::BAD_REQUEST,
            "Invalid ConvertPastedImageRequest protobuf",
        );
    };
    let img = match image::load_from_memory(&req.data) {
        Ok(i) => i,
        Err(e) => return text_error(StatusCode::BAD_REQUEST, format!("Unsupported image: {e}")),
    };

    let mut encoded = Vec::new();
    let ext = req.ext.to_ascii_lowercase();
    let res = match ext.as_str() {
        "png" => {
            let rgba = img.to_rgba8();
            PngEncoder::new(&mut encoded).write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
        }
        _ => {
            let rgb = img.to_rgb8();
            let mut cursor = Cursor::new(&mut encoded);
            JpegEncoder::new_with_quality(&mut cursor, 80).write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
        }
    };
    if let Err(e) = res {
        return text_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Image encoding failed: {e}"),
        );
    }

    let mut out = Vec::new();
    ConvertPastedImageResponse { data: encoded }
        .encode(&mut out)
        .unwrap();
    binary_response(out)
}

async fn handle_get_csv_metadata(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    body: &[u8],
) -> Response {
    let Ok(mut req) = CsvMetadataRequest::decode(body) else {
        return text_error(StatusCode::BAD_REQUEST, "Invalid CsvMetadataRequest");
    };
    let safe_path = match runtime.resolve_safe_tmp_file(&req.path) {
        Ok(p) => p,
        Err(e) => return text_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    req.path = safe_path.to_string_lossy().to_string();
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();
    match call_rslib_method(state, runtime, "getCsvMetadata", buf).await {
        Ok(out) => binary_response(out),
        Err((s, m)) => text_error(s, m),
    }
}

async fn handle_import_csv(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    body: &[u8],
) -> Response {
    let Ok(mut req) = ImportCsvRequest::decode(body) else {
        return text_error(StatusCode::BAD_REQUEST, "Invalid ImportCsvRequest");
    };
    let safe_path = match runtime.resolve_safe_tmp_file(&req.path) {
        Ok(p) => p,
        Err(e) => return text_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    req.path = safe_path.to_string_lossy().to_string();
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();
    match call_rslib_method(state, runtime, "importCsv", buf).await {
        Ok(out) => binary_response(out),
        Err((s, m)) => text_error(s, m),
    }
}

async fn handle_import_apkg(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    body: &[u8],
) -> Response {
    let Ok(mut req) = ImportAnkiPackageRequest::decode(body) else {
        return text_error(StatusCode::BAD_REQUEST, "Invalid ImportAnkiPackageRequest");
    };
    let safe_path = match runtime.resolve_safe_tmp_file(&req.package_path) {
        Ok(p) => p,
        Err(e) => return text_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    req.package_path = safe_path.to_string_lossy().to_string();
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();
    match call_rslib_method(state, runtime, "importAnkiPackage", buf).await {
        Ok(out) => {
            runtime.media_sync.lock().unwrap().reconcile_directory();
            binary_response(out)
        }
        Err((s, m)) => text_error(s, m),
    }
}

async fn handle_import_json_file(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    body: &[u8],
) -> Response {
    let Ok(mut req) = generic::String::decode(body) else {
        return text_error(StatusCode::BAD_REQUEST, "Invalid generic.String");
    };
    let safe_path = match runtime.resolve_safe_tmp_file(&req.val) {
        Ok(p) => p,
        Err(e) => return text_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    req.val = safe_path.to_string_lossy().to_string();
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();
    match call_rslib_method(state, runtime, "importJsonFile", buf).await {
        Ok(out) => binary_response(out),
        Err((s, m)) => text_error(s, m),
    }
}

async fn handle_add_media_from_path(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    body: &[u8],
) -> Response {
    let Ok(mut req) = AddMediaFromPathRequest::decode(body) else {
        return text_error(StatusCode::BAD_REQUEST, "Invalid AddMediaFromPathRequest");
    };
    let safe_path = match runtime.resolve_safe_tmp_file(&req.path) {
        Ok(p) => p,
        Err(e) => return text_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    req.path = safe_path.to_string_lossy().to_string();
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();
    match call_rslib_method(state, runtime, "addMediaFromPath", buf).await {
        Ok(out) => {
            if let Ok(fname) = generic::String::decode(out.as_slice()) {
                if !fname.val.is_empty() {
                    runtime.media_sync.lock().unwrap().record_web_file_added(&fname.val);
                }
            }
            binary_response(out)
        }
        Err((s, m)) => text_error(s, m),
    }
}

async fn handle_create_backup_rpc(
    state: &AppState,
    runtime: &Arc<UserRuntime>,
    body: &[u8],
) -> Response {
    let mut req = CreateBackupRequest::decode(body).unwrap_or_default();
    // Always enforce the user's isolated backups directory.
    req.backup_folder = runtime.backups_dir.to_string_lossy().to_string();
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();
    match call_rslib_method(state, runtime, "createBackup", buf).await {
        Ok(out) => {
            crate::files::prune_old_backups(&runtime.backups_dir, 50);
            binary_response(out)
        }
        Err((s, m)) => text_error(s, m),
    }
}

// ============================================================================
// High-throughput batch helper for Browser table (Phase 2)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BrowserRowsRequest {
    pub ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct BrowserCellJson {
    pub text: String,
    pub is_rtl: bool,
    pub elide_mode: i32,
}

#[derive(Debug, Serialize)]
pub struct BrowserRowJson {
    pub id: i64,
    pub cells: Vec<BrowserCellJson>,
    pub color: i32,
    pub font_name: String,
    pub font_size: u32,
}

#[derive(Debug, Serialize)]
pub struct BrowserRowsResponse {
    pub rows: Vec<BrowserRowJson>,
}

pub async fn browser_rows_batch_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserRowsRequest>,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return text_error(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    if !has_valid_api_token(&headers, &claims) {
        return text_error(StatusCode::FORBIDDEN, "Missing or invalid Cardly API token");
    }
    if req.ids.len() > 500 {
        return text_error(StatusCode::BAD_REQUEST, "Batch size limit is 500 rows");
    }
    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let Some(target) = state.rpc.lookup("browserRowForId") else {
        return text_error(StatusCode::INTERNAL_SERVER_ERROR, "browserRowForId not found");
    };
    let _mode_guard = match runtime.acquire_web_backend().await {
        Ok(g) => g,
        Err(e) => return text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let backend = runtime.backend.clone();
    let ids = req.ids;
    let rows_res = tokio::task::spawn_blocking(move || -> Result<Vec<BrowserRowJson>, String> {
        let mut rows = Vec::with_capacity(ids.len());
        let mut in_buf = Vec::new();
        for id in ids {
            in_buf.clear();
            generic::Int64 { val: id }.encode(&mut in_buf).unwrap();
            match backend.run_service_method(target.service, target.method, &in_buf) {
                Ok(out) => {
                    let row = BrowserRow::decode(out.as_slice())
                        .map_err(|e| format!("decode BrowserRow: {e}"))?;
                    rows.push(BrowserRowJson {
                        id,
                        cells: row
                            .cells
                            .into_iter()
                            .map(|c| BrowserCellJson {
                                text: c.text,
                                is_rtl: c.is_rtl,
                                elide_mode: c.elide_mode,
                            })
                            .collect(),
                        color: row.color,
                        font_name: row.font_name,
                        font_size: row.font_size,
                    });
                }
                Err(err_bytes) => {
                    warn!(id, err = %decode_backend_error(&err_bytes), "browserRowForId failed");
                }
            }
        }
        Ok(rows)
    })
    .await;

    match rows_res {
        Ok(Ok(rows)) => Json(BrowserRowsResponse { rows }).into_response(),
        Ok(Err(e)) => text_error(StatusCode::INTERNAL_SERVER_ERROR, e),
        Err(e) => text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// Syncs the user's browser timezone offset (`new Date().getTimezoneOffset()`) into `localOffset`
/// so `server: true` scheduling calculates day rollovers in the user's local timezone.
#[derive(Debug, Deserialize)]
pub struct TimezoneRequest {
    pub utc_offset_mins: i32,
}

pub async fn set_timezone_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TimezoneRequest>,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return text_error(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    if !has_valid_api_token(&headers, &claims) {
        return text_error(StatusCode::FORBIDDEN, "Missing or invalid Cardly API token");
    }
    if req.utc_offset_mins < -840 || req.utc_offset_mins > 840 {
        return text_error(StatusCode::BAD_REQUEST, "UTC offset out of range");
    }
    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let mut payload = Vec::new();
    anki_proto::config::SetConfigJsonRequest {
        key: "localOffset".to_string(),
        value_json: req.utc_offset_mins.to_string().into_bytes(),
        undoable: false,
    }
    .encode(&mut payload)
    .unwrap();

    match call_rslib_method(&state, &runtime, "setConfigJson", payload).await {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err((s, m)) => text_error(s, m),
    }
}
