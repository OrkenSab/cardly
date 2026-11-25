// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl.html

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use anki::backend::Backend;
use anki::collection::Collection;
use anki::collection::CollectionBuilder;
use anki::prelude::Usn;
use sha1::Digest as _;
use sha1::Sha1;
use anki::services::BackendCollectionService;
use anki::sync::collection::start::ServerSyncState;
use anki::sync::error::HttpResult;
use anki::sync::error::OrHttpErr;
use anki::sync::media::changes::MediaChange;
use anki::sync::media::database::server::entry::upload::UploadedChangeResult;
use anki::sync::media::database::server::entry::MediaEntry;
use anki::sync::media::database::server::ServerMediaDatabase;
use anki::sync::media::sanity::MediaSanityCheckResponse;
use anki::sync::media::upload::MediaUploadResponse;
use anki::sync::media::zip::unzip_and_validate_files;
use anki::sync::media::zip::zip_files_for_download;
use anki::sync::media::zip::UploadedChange;
use anki::sync::media::zip::UploadedChangeKind;
use anki_i18n::I18n;
use anki_proto::collection::CloseCollectionRequest;
use anki_proto::collection::OpenCollectionRequest;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use serde_json::Map;
use serde_json::Value;
use tokio::sync::RwLock;
use tokio::sync::RwLockReadGuard;
use tracing::info;
use tracing::warn;

/// Holds the server-side media sync database (`media.db`) and shared `media/` folder.
pub struct MediaSyncStore {
    pub data_dir: PathBuf,
    pub media_folder: PathBuf,
    pub db: ServerMediaDatabase,
}

impl MediaSyncStore {
    pub fn open(data_dir: &Path, user_folder: &Path) -> Result<Self> {
        let media_folder = user_folder.join("media");
        fs::create_dir_all(&media_folder)
            .with_context(|| format!("creating media folder {}", media_folder.display()))?;
        let db = ServerMediaDatabase::new(&user_folder.join("media.db"))
            .map_err(|e| anyhow!("opening media.db: {e:?}"))?;
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            media_folder,
            db,
        })
    }

    pub fn last_usn(&self) -> HttpResult<Usn> {
        self.db.last_usn().or_internal_err("get last media usn")
    }

    pub fn media_changes_chunk(&self, after_usn: Usn) -> HttpResult<Vec<MediaChange>> {
        self.db
            .media_changes_chunk(after_usn)
            .or_internal_err("media changes chunk")
    }

    pub fn sanity_check(&self, client_file_count: u32) -> HttpResult<MediaSanityCheckResponse> {
        let server = self
            .db
            .nonempty_file_count()
            .or_internal_err("get nonempty media count")?;
        Ok(if server == client_file_count {
            MediaSanityCheckResponse::Ok
        } else {
            MediaSanityCheckResponse::SanityCheckFailed
        })
    }

    pub fn process_uploaded_changes(&mut self, zip_data: Vec<u8>) -> HttpResult<MediaUploadResponse> {
        let extracted = unzip_and_validate_files(&zip_data).or_bad_request("unzip media files")?;
        let folder = &self.media_folder;
        let mut processed = 0;
        let new_usn = self
            .db
            .with_transaction(|db, meta| {
                for change in extracted {
                    match db.register_uploaded_change(meta, change)? {
                        UploadedChangeResult::FileAlreadyDeleted { filename } => {
                            info!(filename, "media already deleted");
                        }
                        UploadedChangeResult::FileIdentical { filename, .. } => {
                            info!(filename, "media already identical");
                        }
                        UploadedChangeResult::Added { filename, data, .. }
                        | UploadedChangeResult::Replaced { filename, data, .. } => {
                            let dest = folder.join(&filename);
                            if let Err(e) = crate::cas::store_media_deduplicated(&self.data_dir, &dest, &data) {
                                warn!(filename, error = %e, "CAS deduplication fallback to direct write");
                                anki_io::write_file(&dest, data)?;
                            }
                        }
                        UploadedChangeResult::Removed { filename, .. } => {
                            let dest = folder.join(&filename);
                            if let Err(err) = fs::remove_file(&dest) {
                                if err.kind() != ErrorKind::NotFound {
                                    warn!(filename, error = %err, "failed removing media file");
                                }
                            }
                        }
                    }
                    processed += 1;
                }
                Ok(())
            })
            .or_internal_err("handle uploaded media changes")?;

        Ok(MediaUploadResponse {
            processed,
            current_usn: new_usn,
        })
    }

    pub fn zip_files_for_download(&mut self, files: Vec<String>) -> HttpResult<Vec<u8>> {
        let entries = self.db.get_entries_for_download(&files)?;
        let filenames_with_data = self.gather_file_data(&entries)?;
        zip_files_for_download(filenames_with_data).or_internal_err("zip media files")
    }

    fn gather_file_data(&mut self, entries: &[MediaEntry]) -> HttpResult<Vec<(String, Vec<u8>)>> {
        let mut out = Vec::with_capacity(entries.len());
        for entry in entries {
            let path = self.media_folder.join(&entry.nfc_filename);
            match fs::read(&path) {
                Ok(data) => out.push((entry.nfc_filename.clone(), data)),
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    self.db
                        .forget_missing_file(entry)
                        .or_internal_err("forget missing media")?;
                    None.or_conflict(format!(
                        "requested a file that doesn't exist: {}",
                        entry.nfc_filename
                    ))?;
                }
                Err(err) => {
                    return Err(err).or_internal_err("read media file for download");
                }
            }
        }
        Ok(out)
    }

    /// Registers a single file that was added/updated by the web Backend into `media.db`
    /// so syncing mobile/desktop clients receive it on their next media sync.
    pub fn record_web_file_added(&mut self, filename: &str) {
        let path = self.media_folder.join(filename);
        let Ok(data) = fs::read(&path) else {
            return;
        };
        if data.is_empty() {
            return;
        }
        let sha1 = Sha1::digest(&data).to_vec();
        let change = UploadedChange {
            nfc_filename: filename.to_string(),
            kind: UploadedChangeKind::AddOrReplace {
                nonempty_data: data,
                sha1,
            },
        };
        if let Err(err) = self.db.with_transaction(|db, meta| {
            let _ = db.register_uploaded_change(meta, change)?;
            Ok(())
        }) {
            warn!(filename, error = ?err, "failed to mirror web media addition to media.db");
        }
    }

    /// Registers a file removal from the web Backend into `media.db`.
    pub fn record_web_file_removed(&mut self, filename: &str) {
        let change = UploadedChange {
            nfc_filename: filename.to_string(),
            kind: UploadedChangeKind::Delete,
        };
        if let Err(err) = self.db.with_transaction(|db, meta| {
            let _ = db.register_uploaded_change(meta, change)?;
            Ok(())
        }) {
            warn!(filename, error = ?err, "failed to mirror web media deletion to media.db");
        }
    }

    /// Scans `media/` after bulk operations (like `.apkg` / `.colpkg` import) and mirrors any
    /// new or updated files into `media.db`.
    pub fn reconcile_directory(&mut self) {
        let Ok(entries) = fs::read_dir(&self.media_folder) else {
            return;
        };
        let mut pending = Vec::new();
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if !ft.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            // Skip Anki internal trash folder markers or hidden files
            if name.starts_with('.') {
                continue;
            }
            let Ok(data) = fs::read(entry.path()) else {
                continue;
            };
            if data.is_empty() {
                continue;
            }
            let sha1 = Sha1::digest(&data).to_vec();
            let needs_update = match self.db.get_nonempty_entry(&name) {
                Ok(Some(existing)) => existing.sha1 != sha1,
                _ => true,
            };
            if needs_update {
                pending.push(UploadedChange {
                    nfc_filename: name,
                    kind: UploadedChangeKind::AddOrReplace {
                        nonempty_data: data,
                        sha1,
                    },
                });
            }
        }
        if pending.is_empty() {
            return;
        }
        if let Err(err) = self.db.with_transaction(|db, meta| {
            for change in pending {
                let _ = db.register_uploaded_change(meta, change)?;
            }
            Ok(())
        }) {
            warn!(error = ?err, "failed to reconcile media directory into media.db");
        }
    }
}

/// Exclusive mode state protecting SQLite's `locking_mode = exclusive` between Web Backend and Sync.
/// `sync_col` is wrapped in `Mutex` so `CollectionMode` is `Sync` (SQLite `Connection` is `Send`, not `Sync`).
pub struct CollectionMode {
    pub web_open: bool,
    pub sync_col: Mutex<Option<Collection>>,
    pub sync_state: Option<ServerSyncState>,
}

pub struct UserRuntime {
    pub username: String,
    pub base_data_dir: PathBuf,
    pub folder: PathBuf,
    pub col_path: PathBuf,
    pub media_dir: PathBuf,
    pub media_db_client: PathBuf,
    pub backups_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub profile_path: PathBuf,
    pub meta_path: PathBuf,
    /// Upstream `rslib` Backend running in `server: true` mode so web mutations emit positive USNs.
    pub backend: Backend,
    /// Coordinates exclusive access between `backend` (Web mode, shared read lock) and
    /// `sync_col` (Sync mode, exclusive write lock).
    pub mode: RwLock<CollectionMode>,
    pub media_sync: Mutex<MediaSyncStore>,
    pub settings_lock: Mutex<()>,
    pub plugins_lock: Mutex<()>,
}

impl UserRuntime {
    pub fn new(base_data_dir: &Path, username: &str) -> Result<Self> {
        let folder = base_data_dir.join("users").join(username);
        let media_dir = folder.join("media");
        let backups_dir = folder.join("backups");
        let tmp_dir = folder.join("tmp");
        let plugins_dir = folder.join("plugins");
        fs::create_dir_all(&media_dir)?;
        fs::create_dir_all(&backups_dir)?;
        fs::create_dir_all(&tmp_dir)?;
        fs::create_dir_all(&plugins_dir)?;

        let col_path = folder.join("collection.anki2");
        let media_db_client = folder.join("collection.media.db2");
        let profile_path = folder.join("profile.json");
        let meta_path = folder.join("meta.json");

        let tr = I18n::new(&["en-US".to_string()]);
        let backend = Backend::new(tr, true);
        let media_sync = MediaSyncStore::open(base_data_dir, &folder)?;

        Ok(Self {
            username: username.to_string(),
            base_data_dir: base_data_dir.to_path_buf(),
            folder,
            col_path,
            media_dir,
            media_db_client,
            backups_dir,
            tmp_dir,
            plugins_dir,
            profile_path,
            meta_path,
            backend,
            mode: RwLock::new(CollectionMode {
                web_open: false,
                sync_col: Mutex::new(None),
                sync_state: None,
            }),
            media_sync: Mutex::new(media_sync),
            settings_lock: Mutex::new(()),
            plugins_lock: Mutex::new(()),
        })
    }

    fn open_web_collection_inner(&self) -> Result<()> {
        let req = OpenCollectionRequest {
            collection_path: self.col_path.to_string_lossy().to_string(),
            media_folder_path: self.media_dir.to_string_lossy().to_string(),
            media_db_path: self.media_db_client.to_string_lossy().to_string(),
        };
        BackendCollectionService::open_collection(&self.backend, req)
            .map_err(|e| anyhow!("open_collection failed: {e:?}"))
    }

    fn close_web_collection_inner(&self) {
        let _ = BackendCollectionService::close_collection(
            &self.backend,
            CloseCollectionRequest {
                downgrade_to_schema11: false,
            },
        );
    }

    /// Acquires a shared read guard guaranteeing the `Backend` collection is open in Web mode.
    /// Multiple web calls (e.g. long FSRS optimization + concurrent `latestProgress` polling)
    /// can hold this read guard simultaneously; `Backend` internally synchronizes SQLite vs progress.
    pub async fn acquire_web_backend(&self) -> Result<RwLockReadGuard<'_, CollectionMode>> {
        {
            let read = self.mode.read().await;
            if read.web_open && read.sync_col.lock().unwrap().is_none() {
                return Ok(read);
            }
        }
        {
            let mut write = self.mode.write().await;
            if write.sync_state.is_some() {
                info!(user = %self.username, "aborting active sync for web request");
                write.sync_state = None;
            }
            *write.sync_col.get_mut().unwrap() = None;
            if !write.web_open {
                self.open_web_collection_inner()?;
                write.web_open = true;
            }
        }
        Ok(self.mode.read().await)
    }

    /// Temporarily closes the Web collection while running a closure that needs closed collection
    /// (e.g. `importCollectionPackage` or backup restore), then reopens it.
    pub async fn with_closed_collection<F, T>(&self, op: F) -> Result<T>
    where
        F: FnOnce(&Backend) -> Result<T>,
    {
        let mut write = self.mode.write().await;
        write.sync_state = None;
        *write.sync_col.get_mut().unwrap() = None;
        if write.web_open {
            self.close_web_collection_inner();
            write.web_open = false;
        }
        let res = op(&self.backend);
        // Reopen immediately so subsequent calls see the new collection.
        if let Err(err) = self.open_web_collection_inner() {
            warn!(user = %self.username, error = ?err, "failed to reopen collection after closed op");
        } else {
            write.web_open = true;
        }
        res
    }

    /// Switches into Sync mode (exclusive write lock) and runs `op` on the raw `Collection`.
    pub async fn with_sync_col<F, T>(&self, op: F) -> HttpResult<T>
    where
        F: FnOnce(&mut Collection) -> HttpResult<T>,
    {
        let mut write = self.mode.write().await;
        if write.web_open {
            self.close_web_collection_inner();
            write.web_open = false;
        }
        if write.sync_state.is_some() {
            info!(user = %self.username, "aborting active sync");
            write.sync_state = None;
            *write.sync_col.get_mut().unwrap() = None;
        }
        let col_opt = write.sync_col.get_mut().unwrap();
        if col_opt.is_none() {
            *col_opt = Some(self.open_raw_sync_col()?);
        }
        op(col_opt.as_mut().unwrap())
    }

    pub async fn start_new_sync(&self, skey: &str) -> HttpResult<()> {
        let mut write = self.mode.write().await;
        if write.web_open {
            self.close_web_collection_inner();
            write.web_open = false;
        }
        if write.sync_state.is_some() {
            info!(user = %self.username, "aborting previous sync before starting new one");
            write.sync_state = None;
            *write.sync_col.get_mut().unwrap() = None;
        }
        write.sync_state = Some(ServerSyncState::new(skey));
        Ok(())
    }

    pub async fn with_sync_state<F, T>(&self, skey: &str, op: F) -> HttpResult<T>
    where
        F: FnOnce(&mut Collection, &mut ServerSyncState) -> anki::error::Result<T>,
    {
        let mut write = self.mode.write().await;
        if write.web_open {
            self.close_web_collection_inner();
            write.web_open = false;
        }
        match &write.sync_state {
            None => None.or_conflict("no active sync")?,
            Some(state) if state.skey != skey => {
                None.or_conflict("active sync with different key")?
            }
            _ => {}
        }
        let CollectionMode {
            sync_col,
            sync_state,
            ..
        } = &mut *write;
        let col_opt = sync_col.get_mut().unwrap();
        if col_opt.is_none() {
            *col_opt = Some(self.open_raw_sync_col()?);
        }
        let col = col_opt.as_mut().or_internal_err("open sync col")?;
        let state = sync_state.as_mut().unwrap();
        let res = op(col, state);
        match res {
            Ok(val) => Ok(val),
            Err(err) => {
                *col_opt = None;
                *sync_state = None;
                Err(err).or_bad_request("op failed in sync_state")
            }
        }
    }

    pub async fn finish_sync_state(&self) {
        let mut write = self.mode.write().await;
        write.sync_state = None;
        *write.sync_col.get_mut().unwrap() = None;
    }

    pub async fn abort_sync_state(&self) {
        let mut write = self.mode.write().await;
        let CollectionMode {
            sync_col,
            sync_state,
            ..
        } = &mut *write;
        let col_opt = sync_col.get_mut().unwrap();
        if sync_state.is_some() || col_opt.is_some() {
            info!(user = %self.username, "aborting active sync");
            *sync_state = None;
            *col_opt = None;
        }
    }

    pub async fn with_sync_col_option<F, T>(&self, op: F) -> HttpResult<T>
    where
        F: FnOnce(&mut Option<Collection>) -> HttpResult<T>,
    {
        let mut write = self.mode.write().await;
        if write.web_open {
            self.close_web_collection_inner();
            write.web_open = false;
        }
        write.sync_state = None;
        let col_opt = write.sync_col.get_mut().unwrap();
        if col_opt.is_none() {
            *col_opt = Some(self.open_raw_sync_col()?);
        }
        let res = op(col_opt);
        // Close sync_col after upload/download so web mode can reopen cleanly.
        *col_opt = None;
        res
    }

    fn open_raw_sync_col(&self) -> HttpResult<Collection> {
        CollectionBuilder::new(&self.col_path)
            .set_media_paths(&self.media_dir, &self.media_db_client)
            .set_server(true)
            .build()
            .or_internal_err("open sync collection")
    }

    pub fn get_json_setting(&self, is_meta: bool, key: &str) -> Result<Value> {
        let _guard = self.settings_lock.lock().unwrap();
        let path = if is_meta {
            &self.meta_path
        } else {
            &self.profile_path
        };
        let map = read_json_map(path);
        Ok(map.get(key).cloned().unwrap_or(Value::Null))
    }

    pub fn set_json_setting(&self, is_meta: bool, key: &str, value: Value) -> Result<()> {
        let _guard = self.settings_lock.lock().unwrap();
        let path = if is_meta {
            &self.meta_path
        } else {
            &self.profile_path
        };
        let mut map = read_json_map(path);
        map.insert(key.to_string(), value);
        let serialized = serde_json::to_vec_pretty(&Value::Object(map))?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Resolves a client-supplied upload token strictly inside `self.tmp_dir` (no traversal).
    pub fn resolve_safe_tmp_file(&self, token_or_name: &str) -> Result<PathBuf> {
        let name = token_or_name.trim().trim_start_matches('/');
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains("..")
            || name.starts_with('.')
        {
            bail!("Invalid temporary file identifier");
        }
        let candidate = self.tmp_dir.join(name);
        if !candidate.is_file() {
            bail!("Temporary file not found: {name}");
        }
        let canon_base = self.tmp_dir.canonicalize()?;
        let canon_file = candidate.canonicalize()?;
        if !canon_file.starts_with(&canon_base) {
            bail!("Path escapes temporary directory");
        }
        Ok(canon_file)
    }

    /// Resolves a backup filename strictly inside `self.backups_dir`.
    pub fn resolve_safe_backup_file(&self, filename: &str) -> Result<PathBuf> {
        let name = filename.trim();
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains("..")
            || !name.ends_with(".colpkg")
        {
            bail!("Invalid backup filename");
        }
        let candidate = self.backups_dir.join(name);
        if !candidate.is_file() {
            bail!("Backup file not found");
        }
        let canon_base = self.backups_dir.canonicalize()?;
        let canon_file = candidate.canonicalize()?;
        if !canon_file.starts_with(&canon_base) {
            bail!("Backup path escapes backups directory");
        }
        Ok(canon_file)
    }
}

fn read_json_map(path: &Path) -> Map<String, Value> {
    let Ok(bytes) = fs::read(path) else {
        return Map::new();
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

pub struct UserManager {
    base_data_dir: PathBuf,
    runtimes: Mutex<HashMap<String, Arc<UserRuntime>>>,
}

impl UserManager {
    pub fn new(base_data_dir: PathBuf) -> Self {
        Self {
            base_data_dir,
            runtimes: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_or_init(&self, username: &str) -> Result<Arc<UserRuntime>> {
        let mut guard = self.runtimes.lock().unwrap();
        if let Some(rt) = guard.get(username) {
            return Ok(rt.clone());
        }
        let rt = Arc::new(UserRuntime::new(&self.base_data_dir, username)?);
        guard.insert(username.to_string(), rt.clone());
        Ok(rt)
    }

    pub fn remove(&self, username: &str) -> Option<Arc<UserRuntime>> {
        let mut guard = self.runtimes.lock().unwrap();
        guard.remove(username)
    }
}
