// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::sync::Arc;

use anki::prelude::TimestampMillis;
use anki::sync::collection::changes::server_apply_changes;
use anki::sync::collection::changes::ApplyChangesRequest;
use anki::sync::collection::changes::UnchunkedChanges;
use anki::sync::collection::chunks::server_apply_chunk;
use anki::sync::collection::chunks::server_chunk;
use anki::sync::collection::chunks::ApplyChunkRequest;
use anki::sync::collection::chunks::Chunk;
use anki::sync::collection::download::server_download;
use anki::sync::collection::finish::server_finish;
use anki::sync::collection::graves::server_apply_graves;
use anki::sync::collection::graves::ApplyGravesRequest;
use anki::sync::collection::graves::Graves;
use anki::sync::collection::meta::server_meta;
use anki::sync::collection::meta::MetaRequest;
use anki::sync::collection::meta::SyncMeta;
use anki::sync::collection::protocol::EmptyInput;
use anki::sync::collection::protocol::SyncMethod;
use anki::sync::collection::protocol::SyncProtocol;
use anki::sync::collection::sanity::server_sanity_check;
use anki::sync::collection::sanity::SanityCheckRequest;
use anki::sync::collection::sanity::SanityCheckResponse;
use anki::sync::collection::sanity::SanityCheckStatus;
use anki::sync::collection::start::server_start;
use anki::sync::collection::start::StartRequest;
use anki::sync::collection::upload::handle_received_upload;
use anki::sync::collection::upload::UploadResponse;
use anki::sync::error::HttpResult;
use anki::sync::error::OrHttpErr;
use anki::sync::login::HostKeyRequest;
use anki::sync::login::HostKeyResponse;
use anki::sync::media;
use anki::sync::media::begin::SyncBeginQuery;
use anki::sync::media::begin::SyncBeginRequest;
use anki::sync::media::begin::SyncBeginResponse;
use anki::sync::media::changes::MediaChangesRequest;
use anki::sync::media::changes::MediaChangesResponse;
use anki::sync::media::download::DownloadFilesRequest;
use anki::sync::media::protocol::JsonResult;
use anki::sync::media::protocol::MediaSyncMethod;
use anki::sync::media::protocol::MediaSyncProtocol;
use anki::sync::media::sanity::MediaSanityCheckResponse;
use anki::sync::media::upload::MediaUploadResponse;
use anki::sync::request::IntoSyncRequest;
use anki::sync::request::SyncRequest;
use anki::sync::request::MAXIMUM_SYNC_PAYLOAD_BYTES;
use anki::sync::response::SyncResponse;
use anki::sync::version::SyncVersion;
use async_trait::async_trait;
use axum::extract::DefaultBodyLimit;
use axum::extract::FromRef;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use axum::Router;

use crate::auth::verify_credentials;
use crate::user_runtime::UserRuntime;
use crate::AppState;

/// Local newtype around `Arc<AppState>` so we satisfy Rust's orphan rules when implementing
/// upstream `SyncProtocol` and `MediaSyncProtocol`.
#[derive(Clone)]
pub struct SyncServerHandle(pub Arc<AppState>);

impl FromRef<Arc<AppState>> for SyncServerHandle {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self(state.clone())
    }
}

macro_rules! sync_method {
    ($server:ident, $req:ident, $method:ident) => {{
        let sync_version = $req.sync_version;
        let obj = $server.$method($req.into_output_type()).await?;
        obj.make_response(sync_version)
    }};
}

async fn sync_handler(
    Path(method): Path<SyncMethod>,
    State(server): State<SyncServerHandle>,
    request: SyncRequest<Vec<u8>>,
) -> HttpResult<Response> {
    Ok(match method {
        SyncMethod::HostKey => sync_method!(server, request, host_key),
        SyncMethod::Meta => sync_method!(server, request, meta),
        SyncMethod::Start => sync_method!(server, request, start),
        SyncMethod::ApplyGraves => sync_method!(server, request, apply_graves),
        SyncMethod::ApplyChanges => sync_method!(server, request, apply_changes),
        SyncMethod::Chunk => sync_method!(server, request, chunk),
        SyncMethod::ApplyChunk => sync_method!(server, request, apply_chunk),
        SyncMethod::SanityCheck2 => sync_method!(server, request, sanity_check),
        SyncMethod::Finish => sync_method!(server, request, finish),
        SyncMethod::Abort => sync_method!(server, request, abort),
        SyncMethod::Upload => sync_method!(server, request, upload),
        SyncMethod::Download => sync_method!(server, request, download),
    })
}

async fn media_begin_get(
    Query(req): Query<SyncBeginQuery>,
    server: State<SyncServerHandle>,
) -> HttpResult<Response> {
    let host_key = req.host_key;
    let mut req = SyncBeginRequest {
        client_version: req.client_version,
    }
    .try_into_sync_request()
    .or_bad_request("convert begin")?;
    req.sync_key = host_key;
    req.sync_version = SyncVersion::multipart();
    media_begin_post(server, req).await
}

async fn media_begin_post(
    server: State<SyncServerHandle>,
    mut req: SyncRequest<SyncBeginRequest>,
) -> HttpResult<Response> {
    if let Some(ver) = &req.media_client_version {
        req.data = serde_json::to_vec(&SyncBeginRequest {
            client_version: ver.clone(),
        })
        .or_internal_err("serialize begin request")?;
    }
    media_sync_handler(Path(MediaSyncMethod::Begin), server, req.into_output_type()).await
}

async fn media_sync_handler(
    Path(method): Path<MediaSyncMethod>,
    State(server): State<SyncServerHandle>,
    request: SyncRequest<Vec<u8>>,
) -> HttpResult<Response> {
    Ok(match method {
        MediaSyncMethod::Begin => sync_method!(server, request, begin),
        MediaSyncMethod::MediaChanges => sync_method!(server, request, media_changes),
        MediaSyncMethod::UploadChanges => sync_method!(server, request, upload_changes),
        MediaSyncMethod::DownloadFiles => sync_method!(server, request, download_files),
        MediaSyncMethod::MediaSanity => sync_method!(server, request, media_sanity_check),
    })
}

pub fn collection_sync_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{method}", post(sync_handler))
        .layer(DefaultBodyLimit::max(*MAXIMUM_SYNC_PAYLOAD_BYTES))
}

pub fn media_sync_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/begin", get(media_begin_get).post(media_begin_post))
        .route("/{method}", post(media_sync_handler))
        .layer(DefaultBodyLimit::max(*MAXIMUM_SYNC_PAYLOAD_BYTES))
}

impl SyncServerHandle {
    fn authenticate_sync_user(&self, hkey: &str) -> HttpResult<Arc<UserRuntime>> {
        let username = self
            .0
            .user_registry
            .get_by_hkey(hkey)
            .map(|u| u.username)
            .or_else(|| {
                let idx = self.0.config.users_by_hkey.get(hkey).copied()?;
                Some(self.0.config.users[idx].username.clone())
            })
            .or_forbidden("invalid hkey")?;
        self.0
            .users
            .get_or_init(&username)
            .or_internal_err("init user runtime for sync")
    }
}

#[async_trait]
impl SyncProtocol for SyncServerHandle {
    async fn host_key(
        &self,
        req: SyncRequest<HostKeyRequest>,
    ) -> HttpResult<SyncResponse<HostKeyResponse>> {
        let input = req.json()?;
        match verify_credentials(&self.0, &input.username, &input.password) {
            Some(user) => SyncResponse::try_from_obj(HostKeyResponse {
                key: user.hkey.clone(),
            }),
            None => None.or_forbidden("invalid user/pass in host_key"),
        }
    }

    async fn meta(&self, req: SyncRequest<MetaRequest>) -> HttpResult<SyncResponse<SyncMeta>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let input = req.json()?;
        let mut meta = rt.with_sync_col(|col| server_meta(input, col)).await?;
        meta.media_usn = rt.media_sync.lock().unwrap().last_usn()?;
        SyncResponse::try_from_obj(meta)
    }

    async fn start(&self, req: SyncRequest<StartRequest>) -> HttpResult<SyncResponse<Graves>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let skey = req.skey()?.to_string();
        let input = req.json()?;
        rt.start_new_sync(&skey).await?;
        let graves = rt
            .with_sync_state(&skey, |col, state| server_start(input, col, state))
            .await?;
        SyncResponse::try_from_obj(graves)
    }

    async fn apply_graves(
        &self,
        req: SyncRequest<ApplyGravesRequest>,
    ) -> HttpResult<SyncResponse<()>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let skey = req.skey()?.to_string();
        let input = req.json()?;
        rt.with_sync_state(&skey, |col, state| server_apply_graves(input, col, state))
            .await?;
        SyncResponse::try_from_obj(())
    }

    async fn apply_changes(
        &self,
        req: SyncRequest<ApplyChangesRequest>,
    ) -> HttpResult<SyncResponse<UnchunkedChanges>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let skey = req.skey()?.to_string();
        let input = req.json()?;
        let out = rt
            .with_sync_state(&skey, |col, state| server_apply_changes(input, col, state))
            .await?;
        SyncResponse::try_from_obj(out)
    }

    async fn chunk(&self, req: SyncRequest<EmptyInput>) -> HttpResult<SyncResponse<Chunk>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let skey = req.skey()?.to_string();
        let _ = req.json()?;
        let chunk = rt.with_sync_state(&skey, server_chunk).await?;
        SyncResponse::try_from_obj(chunk)
    }

    async fn apply_chunk(
        &self,
        req: SyncRequest<ApplyChunkRequest>,
    ) -> HttpResult<SyncResponse<()>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let skey = req.skey()?.to_string();
        let input = req.json()?;
        rt.with_sync_state(&skey, |col, state| server_apply_chunk(input, col, state))
            .await?;
        SyncResponse::try_from_obj(())
    }

    async fn sanity_check(
        &self,
        req: SyncRequest<SanityCheckRequest>,
    ) -> HttpResult<SyncResponse<SanityCheckResponse>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let skey = req.skey()?.to_string();
        let input = req.json()?;
        let resp = rt
            .with_sync_state(&skey, |col, _state| server_sanity_check(input, col))
            .await?;
        if resp.status == SanityCheckStatus::Bad {
            rt.abort_sync_state().await;
        }
        SyncResponse::try_from_obj(resp)
    }

    async fn finish(
        &self,
        req: SyncRequest<EmptyInput>,
    ) -> HttpResult<SyncResponse<TimestampMillis>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let skey = req.skey()?.to_string();
        let _ = req.json()?;
        let now = rt
            .with_sync_state(&skey, |col, _state| server_finish(col))
            .await?;
        rt.finish_sync_state().await;
        SyncResponse::try_from_obj(now)
    }

    async fn abort(&self, req: SyncRequest<EmptyInput>) -> HttpResult<SyncResponse<()>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let _ = req.json()?;
        rt.abort_sync_state().await;
        SyncResponse::try_from_obj(())
    }

    async fn upload(&self, req: SyncRequest<Vec<u8>>) -> HttpResult<SyncResponse<UploadResponse>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let data = req.data;
        let resp = rt
            .with_sync_col_option(|col_opt| handle_received_upload(col_opt, data))
            .await?;
        Ok(SyncResponse::from_upload_response(resp))
    }

    async fn download(&self, req: SyncRequest<EmptyInput>) -> HttpResult<SyncResponse<Vec<u8>>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let schema_version = req.sync_version.collection_schema();
        let _ = req.json()?;
        let bytes = rt
            .with_sync_col_option(|col_opt| server_download(col_opt, schema_version))
            .await?;
        Ok(SyncResponse::from_vec(bytes))
    }
}

#[async_trait]
impl MediaSyncProtocol for SyncServerHandle {
    async fn begin(
        &self,
        req: SyncRequest<SyncBeginRequest>,
    ) -> HttpResult<SyncResponse<JsonResult<SyncBeginResponse>>> {
        let hkey = req.sync_key.clone();
        let rt = self.authenticate_sync_user(&hkey)?;
        let input = req.json()?;
        if input.client_version.is_empty() {
            None.or_bad_request("missing client version")?;
        }
        let usn = rt.media_sync.lock().unwrap().last_usn()?;
        SyncResponse::try_from_obj(JsonResult::ok(SyncBeginResponse {
            usn,
            host_key: hkey,
        }))
    }

    async fn media_changes(
        &self,
        req: SyncRequest<MediaChangesRequest>,
    ) -> HttpResult<SyncResponse<JsonResult<MediaChangesResponse>>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let last_usn = req.json()?.last_usn;
        let changes = rt
            .media_sync
            .lock()
            .unwrap()
            .media_changes_chunk(last_usn)?;
        SyncResponse::try_from_obj(JsonResult::ok(changes))
    }

    async fn upload_changes(
        &self,
        req: SyncRequest<Vec<u8>>,
    ) -> HttpResult<SyncResponse<JsonResult<MediaUploadResponse>>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let data = req.data;
        let res = rt
            .media_sync
            .lock()
            .unwrap()
            .process_uploaded_changes(data)?;
        SyncResponse::try_from_obj(JsonResult::ok(res))
    }

    async fn download_files(
        &self,
        req: SyncRequest<DownloadFilesRequest>,
    ) -> HttpResult<SyncResponse<Vec<u8>>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let files = req.json()?.files;
        let zip = rt
            .media_sync
            .lock()
            .unwrap()
            .zip_files_for_download(files)?;
        Ok(SyncResponse::from_vec(zip))
    }

    async fn media_sanity_check(
        &self,
        req: SyncRequest<media::sanity::SanityCheckRequest>,
    ) -> HttpResult<SyncResponse<JsonResult<MediaSanityCheckResponse>>> {
        let rt = self.authenticate_sync_user(&req.sync_key)?;
        let local = req.json()?.local;
        let res = rt.media_sync.lock().unwrap().sanity_check(local)?;
        SyncResponse::try_from_obj(JsonResult::ok(res))
    }
}
