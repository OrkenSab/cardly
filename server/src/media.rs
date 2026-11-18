// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl.html

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio_util::io::ReaderStream;

use crate::auth::authenticate_request;
use crate::AppState;

/// Exact CSP from upstream `qt/aqt/mediasrv.py` (`UNTRUSTED_MEDIA_CSP`).
/// Prevents user-uploaded HTML/SVG media from executing scripts or accessing the parent origin.
pub const UNTRUSTED_MEDIA_CSP: &str = "\
default-src 'none'; \
script-src 'none'; \
connect-src 'none'; \
object-src 'none'; \
frame-src 'none'; \
child-src 'none'; \
base-uri 'none'; \
form-action 'none'; \
style-src 'self' 'unsafe-inline'; \
img-src 'self'; \
font-src 'self'; \
media-src 'self'; \
sandbox allow-same-origin";

pub fn mime_for_path(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());

    match ext.as_deref() {
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        Some("json") => "application/json",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("ogg") | Some("oga") => "audio/ogg",
        Some("ogv") => "video/ogg",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub fn ensure_safe_path(base_dir: &Path, rel_path: &str) -> Option<PathBuf> {
    if rel_path.is_empty() || rel_path.contains('\0') {
        return None;
    }
    let clean = rel_path.trim_start_matches('/');
    let mut joined = base_dir.to_path_buf();
    for component in Path::new(clean).components() {
        match component {
            std::path::Component::Normal(part) => joined.push(part),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    let canon_base = base_dir.canonicalize().ok()?;
    let canon_target = joined.canonicalize().ok()?;
    if !canon_target.starts_with(&canon_base) {
        return None;
    }
    if canon_target.is_dir() {
        return None;
    }
    Some(canon_target)
}

/// Serves a file from the authenticated user's media folder with `UNTRUSTED_MEDIA_CSP`
/// and HTTP `Range` support (required for smooth HTML5 audio/video seeking).
pub async fn serve_user_media(state: &Arc<AppState>, headers: &HeaderMap, rel_path: &str) -> Response {
    let Some(claims) = authenticate_request(state, headers) else {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    };
    let Ok(runtime) = state.users.get_or_init(&claims.u) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to open user runtime").into_response();
    };

    let Some(full_path) = ensure_safe_path(&runtime.media_dir, rel_path) else {
        return (StatusCode::NOT_FOUND, "Media file not found").into_response();
    };

    serve_file_with_range(&full_path, headers, true).await
}

pub async fn serve_file_with_range(
    full_path: &Path,
    headers: &HeaderMap,
    untrusted_media: bool,
) -> Response {
    let Ok(meta) = tokio::fs::metadata(full_path).await else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    let file_size = meta.len();
    let mime = mime_for_path(&full_path.to_string_lossy());

    // Handle single byte-range request (e.g. `Range: bytes=0-1023` or `bytes=100-`).
    if let Some(range_val) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if let Some((start, end)) = parse_single_range(range_val, file_size) {
            let len = end - start + 1;
            let Ok(mut file) = File::open(full_path).await else {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response();
            };
            if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Seek failed").into_response();
            }
            let limited = file.take(len);
            let stream = ReaderStream::new(limited);
            let mut builder = Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, mime)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_LENGTH, len.to_string())
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{file_size}"),
                )
                .header(header::CACHE_CONTROL, "private, max-age=3600");
            if untrusted_media {
                builder = builder.header(header::CONTENT_SECURITY_POLICY, UNTRUSTED_MEDIA_CSP);
            }
            return builder.body(Body::from_stream(stream)).unwrap();
        } else {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{file_size}"))
                .body(Body::empty())
                .unwrap();
        }
    }

    let Ok(file) = File::open(full_path).await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response();
    };
    let stream = ReaderStream::new(file);
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, file_size.to_string())
        .header(header::CACHE_CONTROL, "private, max-age=3600");
    if untrusted_media {
        builder = builder.header(header::CONTENT_SECURITY_POLICY, UNTRUSTED_MEDIA_CSP);
    }
    builder.body(Body::from_stream(stream)).unwrap()
}

fn parse_single_range(header_val: &str, file_size: u64) -> Option<(u64, u64)> {
    if file_size == 0 {
        return None;
    }
    let spec = header_val.strip_prefix("bytes=")?.trim();
    if spec.contains(',') {
        return None;
    }
    let (start_s, end_s) = spec.split_once('-')?;
    if start_s.is_empty() {
        let suffix_len: u64 = end_s.parse().ok()?;
        if suffix_len == 0 {
            return None;
        }
        let start = file_size.saturating_sub(suffix_len);
        Some((start, file_size - 1))
    } else {
        let start: u64 = start_s.parse().ok()?;
        if start >= file_size {
            return None;
        }
        let end = if end_s.is_empty() {
            file_size - 1
        } else {
            end_s.parse::<u64>().ok()?.min(file_size - 1)
        };
        if end < start {
            return None;
        }
        Some((start, end))
    }
}
