// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

pub mod ai;
pub mod auth;
pub mod cas;
pub mod config;
pub mod demo;
pub mod files;
pub mod media;
pub mod rpc;
pub mod static_assets;
pub mod sync_server;
pub mod user_registry;
pub mod user_runtime;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::config::ServerConfig;
use crate::rpc::RpcRegistry;
use crate::user_registry::UserRegistry;
use crate::user_runtime::UserManager;

pub struct AppState {
    pub config: ServerConfig,
    pub user_registry: Arc<UserRegistry>,
    pub users: UserManager,
    pub rpc: RpcRegistry,
    pub ai_quota: Arc<ai::AiQuotaManager>,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Arc<Self> {
        let user_registry = Arc::new(
            UserRegistry::new(&config.data_dir, config.users.clone())
                .expect("initialize user registry"),
        );
        let users = UserManager::new(config.data_dir.clone());
        let rpc = RpcRegistry::build();
        let ai_quota = Arc::new(ai::AiQuotaManager::new(
            config.ai_server_key.clone(),
            config.ai_global_daily_limit,
            config.ai_ip_daily_limit,
        ));
        Arc::new(Self {
            config,
            user_registry,
            users,
            rpc,
            ai_quota,
        })
    }

    pub fn is_demo_user(&self, username: &str) -> bool {
        self.config.demo_mode
            && (self
                .config
                .demo_user
                .as_deref()
                .map(|u| u == username)
                .unwrap_or(false)
                || self.user_registry.is_guest(username))
    }

    pub async fn cleanup_stale_guests(&self, max_age_secs: u64) -> usize {
        let stale = self.user_registry.cleanup_stale_guests(max_age_secs);
        let count = stale.len();
        for uname in stale {
            if let Some(rt) = self.users.remove(&uname) {
                let _ = rt.with_closed_collection(|_| Ok(())).await;
                let _ = std::fs::remove_dir_all(&rt.folder);
            } else {
                let user_dir = self.config.data_dir.join("users").join(&uname);
                let _ = std::fs::remove_dir_all(user_dir);
            }
        }
        if count > 0 {
            let _ = crate::cas::prune_orphaned_cas_blobs(&self.config.data_dir);
        }
        count
    }
}

async fn healthz_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")],
        Json(serde_json::json!({ "status": "ok" })),
    )
}

async fn readyz_handler() -> impl IntoResponse {
    StatusCode::OK
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let max_upload = state.config.max_upload_bytes;
    let ip_ext = state.config.ip_source.clone().into_extension();

    Router::new()
        // Health checks (Docker / orchestrators / Anki sync client)
        .route("/healthz", get(healthz_handler))
        .route("/health", get(healthz_handler))
        .route("/_anki/readyz", get(readyz_handler))
        // Authentication
        .route("/api/auth/login", post(auth::login_handler))
        .route("/api/auth/logout", post(auth::logout_handler))
        .route("/api/auth/me", get(auth::me_handler))
        .route("/api/auth/register", post(auth::register_handler))
        .route("/api/auth/guest", post(auth::guest_handler))
        .route("/api/register", post(auth::register_handler))
        .route("/auth/register", post(auth::register_handler))
        .route("/auth/guest", post(auth::guest_handler))
        // Demo mode status
        .route("/api/demo/status", get(demo::demo_status_handler))
        // AI Card Generator & Batch Add APIs
        .route(
            "/api/ai/generate",
            post(ai::ai_generate_handler).layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        )
        .route("/api/ai/batch-add", post(ai::ai_batch_add_handler))
        // Cardly auxiliary APIs (uploads, exports, batch browser rows, timezone, backups)
        .route(
            "/api/uploads",
            post(files::upload_file_handler).layer(DefaultBodyLimit::max(max_upload)),
        )
        .route("/api/import/colpkg", post(files::import_colpkg_handler))
        .route("/api/export", post(files::export_handler))
        .route("/api/browser/rows", post(rpc::browser_rows_batch_handler))
        .route("/api/timezone", post(rpc::set_timezone_handler))
        .route("/api/backups", get(files::list_backups_handler))
        .route("/api/backups/create", post(files::create_backup_handler))
        .route("/api/backups/{name}", get(files::download_backup_handler))
        .route(
            "/api/backups/{name}/restore",
            post(files::restore_backup_handler),
        )
        // Upstream Anki protobuf-over-HTTP RPC bridge + injected web bridge script/theme
        .route(
            "/_anki/cardly-bridge.js",
            get(static_assets::cardly_bridge_js_handler),
        )
        .route(
            "/_anki/cardly-theme.css",
            get(static_assets::cardly_theme_css_handler),
        )
        .route(
            "/_anki/{method}",
            post(rpc::handle_anki_rpc).layer(DefaultBodyLimit::max(max_upload)),
        )
        // Built-in Anki-compatible sync server (`/sync/*` & `/msync/*`)
        .nest("/sync", sync_server::collection_sync_router())
        .nest("/msync", sync_server::media_sync_router())
        // App entry & fallback (upstream SvelteKit routes, static assets, per-user media)
        .route("/", get(static_assets::index_handler))
        .fallback(get(static_assets::fallback_get_handler))
        .with_state(state)
        .layer(ip_ext)
        .layer(TraceLayer::new_for_http())
}
