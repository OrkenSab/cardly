// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl.html

use std::fs;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anki_proto::collection::CreateBackupRequest;
use anki_proto::import_export::export_limit;
use anki_proto::import_export::ExportAnkiPackageOptions;
use anki_proto::import_export::ExportAnkiPackageRequest;
use anki_proto::import_export::ExportCardCsvRequest;
use anki_proto::import_export::ExportCollectionPackageRequest;
use anki_proto::import_export::ExportLimit;
use anki_proto::import_export::ExportNoteCsvRequest;
use anki_proto::import_export::ImportCollectionPackageRequest;
use axum::body::Body;
use axum::extract::Multipart;
use axum::extract::Path;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use prost::Message;
use serde::Deserialize;
use serde::Serialize;

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

fn sanitize_ext(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin");
    let clean: String = ext
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(12)
        .collect();
    if clean.is_empty() {
        "bin".to_string()
    } else {
        clean.to_ascii_lowercase()
    }
}

fn random_token() -> String {
    let mut bytes = [0u8; 16];
    rand::fill(&mut bytes);
    hex::encode(bytes)
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub upload_id: String,
    pub filename: String,
    pub size: usize,
    pub kind: String,
}

/// Uploads a file (`.apkg`, `.colpkg`, `.csv`, `.tsv`, `.txt`, or media) into the user's
/// isolated `tmp/` directory and returns an opaque `upload_id`.
pub async fn upload_file_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
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
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/uploads", None)
    {
        return text_err(status, msg);
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    while let Ok(Some(field)) = multipart.next_field().await {
        let orig_name = field
            .file_name()
            .map(str::to_string)
            .unwrap_or_else(|| "upload.bin".to_string());
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => return text_err(StatusCode::BAD_REQUEST, format!("Upload read error: {e}")),
        };
        let max_allowed = if is_demo {
            crate::demo::DEMO_MAX_UPLOAD_BYTES
        } else {
            state.config.max_upload_bytes
        };
        if data.len() > max_allowed {
            return text_err(
                StatusCode::PAYLOAD_TOO_LARGE,
                if is_demo {
                    "Demo mode upload size limit is 10 MiB"
                } else {
                    "Uploaded file exceeds limit"
                },
            );
        }
        let ext = sanitize_ext(&orig_name);
        let upload_id = format!("up_{}.{}", random_token(), ext);
        let dest = runtime.tmp_dir.join(&upload_id);
        if let Err(e) = tokio::fs::write(&dest, &data).await {
            return text_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to save upload: {e}"),
            );
        }
        let kind = match ext.as_str() {
            "apkg" => "apkg",
            "colpkg" => "colpkg",
            "csv" | "tsv" | "txt" => "csv",
            _ => "other",
        };
        return Json(UploadResponse {
            upload_id,
            filename: orig_name,
            size: data.len(),
            kind: kind.to_string(),
        })
        .into_response();
    }

    text_err(StatusCode::BAD_REQUEST, "No file field provided in multipart form")
}

#[derive(Debug, Deserialize)]
pub struct ImportColpkgRequest {
    pub upload_id: String,
}

/// Imports a `.colpkg` backup/package, replacing the user's current collection (requires closing
/// the collection first, then reopening and reconciling `media.db`).
pub async fn import_colpkg_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ImportColpkgRequest>,
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
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/import/colpkg", None)
    {
        return text_err(status, msg);
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let safe_path = match runtime.resolve_safe_tmp_file(&req.upload_id) {
        Ok(p) => p,
        Err(e) => return text_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let Some(target) = state.rpc.lookup("importCollectionPackage") else {
        return text_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "importCollectionPackage RPC missing",
        );
    };

    let proto_req = ImportCollectionPackageRequest {
        col_path: runtime.col_path.to_string_lossy().to_string(),
        backup_path: safe_path.to_string_lossy().to_string(),
        media_folder: runtime.media_dir.to_string_lossy().to_string(),
        media_db: runtime.media_db_client.to_string_lossy().to_string(),
    };
    let mut in_bytes = Vec::new();
    proto_req.encode(&mut in_bytes).unwrap();

    let res = runtime
        .with_closed_collection(|backend| {
            backend
                .run_service_method(target.service, target.method, &in_bytes)
                .map_err(|_| anyhow::anyhow!("importCollectionPackage failed"))
        })
        .await;

    match res {
        Ok(_) => {
            runtime.media_sync.lock().unwrap().reconcile_directory();
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(e) => text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    /// "apkg" | "colpkg" | "notes_csv" | "cards_csv"
    pub format: String,
    pub deck_id: Option<i64>,
    pub note_ids: Option<Vec<i64>>,
    pub card_ids: Option<Vec<i64>>,
    #[serde(default)]
    pub include_scheduling: bool,
    #[serde(default)]
    pub include_deck_configs: bool,
    #[serde(default = "default_true")]
    pub include_media: bool,
    #[serde(default)]
    pub legacy: bool,
    #[serde(default = "default_true")]
    pub include_html: bool,
    #[serde(default)]
    pub include_tags: bool,
    #[serde(default)]
    pub include_deck: bool,
    #[serde(default)]
    pub include_notetype: bool,
    #[serde(default)]
    pub include_guid: bool,
    pub filename: Option<String>,
}

fn default_true() -> bool {
    true
}

fn build_export_limit(req: &ExportRequest) -> ExportLimit {
    if let Some(nids) = &req.note_ids {
        ExportLimit {
            limit: Some(export_limit::Limit::NoteIds(anki_proto::notes::NoteIds {
                note_ids: nids.clone(),
            })),
        }
    } else if let Some(cids) = &req.card_ids {
        ExportLimit {
            limit: Some(export_limit::Limit::CardIds(anki_proto::cards::CardIds {
                cids: cids.clone(),
            })),
        }
    } else if let Some(did) = req.deck_id {
        ExportLimit {
            limit: Some(export_limit::Limit::DeckId(did)),
        }
    } else {
        ExportLimit {
            limit: Some(export_limit::Limit::WholeCollection(
                anki_proto::generic::Empty {},
            )),
        }
    }
}

/// Runs an `rslib` export into the user's temp directory and streams the resulting file back.
pub async fn export_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ExportRequest>,
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
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/import/colpkg", None)
    {
        return text_err(status, msg);
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let (ext, mime, method_name) = match req.format.as_str() {
        "apkg" => ("apkg", "application/octet-stream", "exportAnkiPackage"),
        "colpkg" => ("colpkg", "application/octet-stream", "exportCollectionPackage"),
        "notes_csv" => ("txt", "text/plain; charset=utf-8", "exportNoteCsv"),
        "cards_csv" => ("txt", "text/plain; charset=utf-8", "exportCardCsv"),
        _ => return text_err(StatusCode::BAD_REQUEST, "Unsupported export format"),
    };

    let tmp_name = format!("exp_{}.{ext}", random_token());
    let out_path = runtime.tmp_dir.join(&tmp_name);
    let out_path_str = out_path.to_string_lossy().to_string();

    let mut payload = Vec::new();
    match req.format.as_str() {
        "apkg" => {
            ExportAnkiPackageRequest {
                out_path: out_path_str,
                options: Some(ExportAnkiPackageOptions {
                    with_scheduling: req.include_scheduling,
                    with_deck_configs: req.include_deck_configs,
                    with_media: req.include_media,
                    legacy: req.legacy,
                }),
                limit: Some(build_export_limit(&req)),
            }
            .encode(&mut payload)
            .unwrap();
        }
        "colpkg" => {
            ExportCollectionPackageRequest {
                out_path: out_path_str,
                include_media: req.include_media,
                legacy: req.legacy,
            }
            .encode(&mut payload)
            .unwrap();
        }
        "notes_csv" => {
            ExportNoteCsvRequest {
                out_path: out_path_str,
                with_html: req.include_html,
                with_tags: req.include_tags,
                with_deck: req.include_deck,
                with_notetype: req.include_notetype,
                with_guid: req.include_guid,
                limit: Some(build_export_limit(&req)),
            }
            .encode(&mut payload)
            .unwrap();
        }
        "cards_csv" => {
            ExportCardCsvRequest {
                out_path: out_path_str,
                with_html: req.include_html,
                limit: Some(build_export_limit(&req)),
            }
            .encode(&mut payload)
            .unwrap();
        }
        _ => unreachable!(),
    }

    if let Err((status, msg)) = call_rslib_method(&state, &runtime, method_name, payload).await {
        let _ = tokio::fs::remove_file(&out_path).await;
        return text_err(status, msg);
    }

    let bytes = match tokio::fs::read(&out_path).await {
        Ok(b) => b,
        Err(e) => {
            return text_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed reading exported file: {e}"),
            )
        }
    };
    let _ = tokio::fs::remove_file(&out_path).await;

    let download_name = req
        .filename
        .as_deref()
        .map(|s| {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ' '))
                .collect::<String>()
        })
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("cardly-export.{ext}"));
    let download_name = if download_name.ends_with(&format!(".{ext}")) {
        download_name
    } else {
        format!("{download_name}.{ext}")
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{download_name}\""),
        )
        .body(Body::from(bytes))
        .unwrap()
}

// ============================================================================
// Backups API (list / create / download / restore)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct BackupEntry {
    pub name: String,
    pub size: u64,
    pub modified_secs: u64,
}

pub fn prune_old_backups(backups_dir: &std::path::Path, max_keep: usize) {
    let Ok(read_dir) = fs::read_dir(backups_dir) else {
        return;
    };
    let mut files: Vec<(std::path::PathBuf, u64)> = Vec::new();
    for entry in read_dir.flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".colpkg") {
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    files.push((entry.path(), mtime));
                }
            }
        }
    }
    if files.len() > max_keep {
        files.sort_by(|a, b| b.1.cmp(&a.1)); // newest first
        for (path, _) in &files[max_keep..] {
            let _ = fs::remove_file(path);
        }
    }
}

pub async fn list_backups_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return text_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(&runtime.backups_dir) {
        for entry in read_dir.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if !name.ends_with(".colpkg") {
                continue;
            }
            let modified_secs = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            entries.push(BackupEntry {
                name,
                size: meta.len(),
                modified_secs,
            });
        }
    }
    entries.sort_by(|a, b| b.modified_secs.cmp(&a.modified_secs));
    Json(serde_json::json!({ "backups": entries })).into_response()
}

pub async fn create_backup_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
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
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/import/colpkg", None)
    {
        return text_err(status, msg);
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let mut payload = Vec::new();
    CreateBackupRequest {
        backup_folder: runtime.backups_dir.to_string_lossy().to_string(),
        force: true,
        wait_for_completion: true,
    }
    .encode(&mut payload)
    .unwrap();

    match call_rslib_method(&state, &runtime, "createBackup", payload).await {
        Ok(_) => {
            prune_old_backups(&runtime.backups_dir, 50);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            Json(serde_json::json!({ "ok": true, "timestamp": now })).into_response()
        }
        Err((s, m)) => text_err(s, m),
    }
}

pub async fn download_backup_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return text_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };
    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let safe_path = match runtime.resolve_safe_backup_file(&name) {
        Ok(p) => p,
        Err(e) => return text_err(StatusCode::NOT_FOUND, e.to_string()),
    };
    let Ok(bytes) = tokio::fs::read(&safe_path).await else {
        return text_err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to read backup");
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{name}\""),
        )
        .body(Body::from(bytes))
        .unwrap()
}

pub async fn restore_backup_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
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
        crate::demo::validate_demo_action(&claims.u, is_demo, "/api/backups/restore", None)
    {
        return text_err(status, msg);
    }

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let safe_path = match runtime.resolve_safe_backup_file(&name) {
        Ok(p) => p,
        Err(e) => return text_err(StatusCode::NOT_FOUND, e.to_string()),
    };
    let Some(target) = state.rpc.lookup("importCollectionPackage") else {
        return text_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "importCollectionPackage RPC missing",
        );
    };

    let proto_req = ImportCollectionPackageRequest {
        col_path: runtime.col_path.to_string_lossy().to_string(),
        backup_path: safe_path.to_string_lossy().to_string(),
        media_folder: runtime.media_dir.to_string_lossy().to_string(),
        media_db: runtime.media_db_client.to_string_lossy().to_string(),
    };
    let mut in_bytes = Vec::new();
    proto_req.encode(&mut in_bytes).unwrap();

    let res = runtime
        .with_closed_collection(|backend| {
            backend
                .run_service_method(target.service, target.method, &in_bytes)
                .map_err(|_| anyhow::anyhow!("restore backup failed"))
        })
        .await;

    match res {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => text_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
