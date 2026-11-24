// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl.html

use std::net::IpAddr;
use std::net::SocketAddr;
use std::path::PathBuf;

use anki::sync::collection::meta::MetaRequest;
use anki::sync::collection::protocol::SyncProtocol;
use anki::sync::http_client::HttpSyncClient;
use anki::sync::login::sync_login;
use anki::sync::media::begin::SyncBeginRequest;
use anki::sync::media::changes::MediaChangesRequest;
use anki::sync::media::protocol::MediaSyncProtocol;
use anki::sync::request::IntoSyncRequest;
use anki::sync::version::SYNC_VERSION_MAX;
use anki_proto::decks::DeckTreeNode;
use anki_proto::decks::DeckTreeRequest;
use anki_proto::generic;
use cardly_server::rpc::ConvertPastedImageRequest;
use cardly_server::rpc::ConvertPastedImageResponse;
use cardly_server::rpc::SetSettingJsonRequest;
use anki_proto::import_export::CsvMetadata;
use anki_proto::import_export::CsvMetadataRequest;
use anki_proto::import_export::ImportCsvRequest;
use anki_proto::import_export::ImportResponse;
use anki_proto::media::AddMediaFileRequest;
use anki_proto::notes::AddNoteRequest;
use anki_proto::notes::Note;
use anki_proto::notetypes::NotetypeNames;
use anki_proto::scheduler::card_answer::Rating;
use anki_proto::scheduler::CardAnswer;
use anki_proto::scheduler::GetQueuedCardsRequest;
use anki_proto::scheduler::QueuedCards;
use anki_proto::search::SearchRequest;
use anki_proto::search::SearchResponse;
use axum::body::Body;
use axum::http::header;
use axum::http::Method;
use axum::http::Request;
use axum::http::StatusCode;
use axum::Router;
use axum_client_ip::ClientIpSource;
use cardly_server::auth::API_TOKEN_HEADER;
use cardly_server::build_router;
use cardly_server::config::parse_users;
use cardly_server::config::ServerConfig;
use cardly_server::media::UNTRUSTED_MEDIA_CSP;
use cardly_server::AppState;
use http_body_util::BodyExt;
use prost::Message;
use tempfile::TempDir;
use tower::ServiceExt;

fn resolve_test_upstream_web(tmp: &TempDir) -> PathBuf {
    let repo_upstream = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../anki-upstream/out/qt/_aqt/data/web");
    if repo_upstream.join("sveltekit/index.html").exists() {
        return repo_upstream;
    }

    let tarball = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("upstream-web-assets.tar.gz");
    if tarball.exists() {
        let target_dir = tmp.path().join("extracted_upstream_web");
        std::fs::create_dir_all(&target_dir).ok();
        let status = std::process::Command::new("tar")
            .args(["-xzf", tarball.to_str().unwrap(), "-C", target_dir.to_str().unwrap()])
            .status();
        if status.map(|s| s.success()).unwrap_or(false) && target_dir.join("web/sveltekit/index.html").exists() {
            return target_dir.join("web");
        }
    }

    let mock = tmp.path().join("mock_upstream_web");
    let sveltekit = mock.join("sveltekit");
    std::fs::create_dir_all(&sveltekit).expect("create mock sveltekit dir");
    std::fs::write(
        sveltekit.join("index.html"),
        b"<!DOCTYPE html><html><head><title>Mock Upstream</title></head><body>Graphs</body></html>",
    )
    .expect("write mock index.html");
    mock
}

struct TestHarness {
    _tmp: TempDir,
    app: Router,
}

impl TestHarness {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let upstream_web = resolve_test_upstream_web(&tmp);
        let users = parse_users(
            &[
                "alice:secret123".to_string(),
                "bob:otherpass".to_string(),
            ],
            false,
        )
        .expect("parse users");

        let config = ServerConfig::build_with_users(
            "127.0.0.1".parse::<IpAddr>().unwrap(),
            0,
            tmp.path().to_path_buf(),
            upstream_web,
            None,
            b"test-session-secret-key-32-bytes!".to_vec(),
            false,
            64 * 1024 * 1024,
            ClientIpSource::ConnectInfo,
            users,
        )
        .expect("build config");

        let state = AppState::new(config);
        let app = build_router(state);
        Self { _tmp: tmp, app }
    }

    fn new_with_demo() -> (Self, std::sync::Arc<AppState>) {
        let tmp = TempDir::new().expect("tempdir");
        let upstream_web = resolve_test_upstream_web(&tmp);
        let users = parse_users(
            &[
                "demo:demo123".to_string(),
                "alice:secret123".to_string(),
            ],
            false,
        )
        .expect("parse users");

        let config = ServerConfig::build_with_all_options(
            "127.0.0.1".parse::<IpAddr>().unwrap(),
            0,
            tmp.path().to_path_buf(),
            upstream_web,
            None,
            b"test-session-secret-key-32-bytes!".to_vec(),
            false,
            64 * 1024 * 1024,
            ClientIpSource::ConnectInfo,
            users,
            true,
            Some("demo".to_string()),
            Some(3600),
            false,
            None,
            50,
            10,
        )
        .expect("build config");

        let state = AppState::new(config);
        let app = build_router(state.clone());
        (Self { _tmp: tmp, app }, state)
    }

    async fn send(&self, mut req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        req.extensions_mut().insert(axum::extract::ConnectInfo(
            SocketAddr::from(([127, 0, 0, 1], 12345)),
        ));
        let resp = self.app.clone().oneshot(req).await.expect("router oneshot");
        let status = resp.status();
        let headers = resp.headers().clone();
        let bytes = resp
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes()
            .to_vec();
        (status, headers, bytes)
    }

    async fn login(&self, username: &str, password: &str) -> (String, String) {
        let body = serde_json::json!({ "username": username, "password": password }).to_string();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();
        let (status, headers, bytes) = self.send(req).await;
        assert_eq!(status, StatusCode::OK, "login failed: {}", String::from_utf8_lossy(&bytes));
        let cookie = headers
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let token = json["api_token"].as_str().unwrap().to_string();
        (cookie, token)
    }

    async fn rpc_raw(
        &self,
        cookie: &str,
        token: &str,
        method: &str,
        payload: Vec<u8>,
    ) -> (StatusCode, Vec<u8>) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/_anki/{method}"))
            .header(header::COOKIE, cookie)
            .header(API_TOKEN_HEADER, token)
            .header(header::CONTENT_TYPE, "application/binary")
            .body(Body::from(payload))
            .unwrap();
        let (status, _headers, bytes) = self.send(req).await;
        (status, bytes)
    }
}

#[tokio::test]
async fn healthcheck_and_auth_guards() {
    let h = TestHarness::new();

    // Health endpoints
    let (status, _, bytes) = h
        .send(Request::get("/healthz").body(Body::empty()).unwrap())
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&bytes).contains("ok"));

    // Unauthenticated RPC must be rejected
    let (status, _, _) = h
        .send(
            Request::post("/_anki/deckTree")
                .header(header::CONTENT_TYPE, "application/binary")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Wrong password rejected
    let (status, _, _) = h
        .send(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"alice","password":"wrong"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Valid login succeeds and /api/auth/me works
    let (cookie, token) = h.login("alice", "secret123").await;
    let (status, _, bytes) = h
        .send(
            Request::get("/api/auth/me")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let me: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(me["username"], "alice");

    // Missing API token on mutating/non-whitelisted RPC must be 403 (card JS isolation)
    let (status, _, _) = h
        .send(
            Request::post("/_anki/deckTree")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/binary")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Blocked server-path RPC must be 403 even with valid API token
    let (status, _) = h.rpc_raw(&cookie, &token, "openCollection", vec![]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn end_to_end_add_card_review_and_multiuser_isolation() {
    let h = TestHarness::new();
    let (alice_cookie, alice_tok) = h.login("alice", "secret123").await;
    let (bob_cookie, bob_tok) = h.login("bob", "otherpass").await;

    // 1. Fetch Basic notetype ID for Alice
    let (status, bytes) = h
        .rpc_raw(&alice_cookie, &alice_tok, "getNotetypeNames", vec![])
        .await;
    assert_eq!(status, StatusCode::OK);
    let nt_names = NotetypeNames::decode(bytes.as_slice()).unwrap();
    let basic = nt_names
        .entries
        .iter()
        .find(|e| e.name == "Basic")
        .expect("Basic notetype exists");

    // 2. Add a note to Alice's Default deck (id = 1)
    let add_req = AddNoteRequest {
        note: Some(Note {
            id: 0,
            guid: String::new(),
            notetype_id: basic.id,
            mtime_secs: 0,
            usn: 0,
            tags: vec!["phase1".to_string()],
            fields: vec!["Capital of France?".to_string(), "Paris".to_string()],
        }),
        deck_id: 1,
    };
    let (status, bytes) = h
        .rpc_raw(&alice_cookie, &alice_tok, "addNote", add_req.encode_to_vec())
        .await;
    assert_eq!(status, StatusCode::OK, "addNote failed: {}", String::from_utf8_lossy(&bytes));

    // 3. Search cards for Alice vs Bob (verify strict per-user isolation)
    let search_req = SearchRequest {
        search: "tag:phase1".to_string(),
        order: None,
    };
    let (status, alice_search) = h
        .rpc_raw(
            &alice_cookie,
            &alice_tok,
            "searchCards",
            search_req.encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let alice_ids = SearchResponse::decode(alice_search.as_slice()).unwrap().ids;
    assert_eq!(alice_ids.len(), 1);

    let (status, bob_search) = h
        .rpc_raw(
            &bob_cookie,
            &bob_tok,
            "searchCards",
            search_req.encode_to_vec(),
        )
        .await;
    assert!(
        status == StatusCode::OK || status == StatusCode::NO_CONTENT,
        "unexpected status for empty bob search: {status}"
    );
    let bob_ids = SearchResponse::decode(bob_search.as_slice()).unwrap().ids;
    assert_eq!(bob_ids.len(), 0, "Bob must not see Alice's cards");

    // 4. Fetch queued card and answer it with GOOD via real rslib v3 scheduler
    let queue_req = GetQueuedCardsRequest {
        fetch_limit: 5,
        intraday_learning_only: false,
    };
    let (status, q_bytes) = h
        .rpc_raw(
            &alice_cookie,
            &alice_tok,
            "getQueuedCards",
            queue_req.encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let queued = QueuedCards::decode(q_bytes.as_slice()).unwrap();
    assert_eq!(queued.cards.len(), 1);
    assert_eq!(queued.new_count, 1);

    let top = &queued.cards[0];
    let card = top.card.as_ref().unwrap();
    let next_good = top.states.as_ref().unwrap().good.clone();
    let now_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let answer = CardAnswer {
        card_id: card.id,
        current_state: top.states.as_ref().unwrap().current.clone(),
        new_state: next_good,
        rating: Rating::Good as i32,
        answered_at_millis: now_millis,
        milliseconds_taken: 2500,
    };
    let (status, _) = h
        .rpc_raw(
            &alice_cookie,
            &alice_tok,
            "answerCard",
            answer.encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 5. Verify deck tree counts updated (0 new, 1 learning on Default deck)
    let now_secs = now_millis / 1000;
    let (status, tree_bytes) = h
        .rpc_raw(
            &alice_cookie,
            &alice_tok,
            "deckTree",
            DeckTreeRequest { now: now_secs }.encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let tree = DeckTreeNode::decode(tree_bytes.as_slice()).unwrap();
    let default_deck = tree
        .children
        .iter()
        .find(|d| d.deck_id == 1)
        .expect("Default deck present in deckTree");
    assert_eq!(default_deck.new_count, 0);
    assert_eq!(default_deck.learn_count, 1);

    // 6. Configure active browser columns and verify batch browser rows endpoint works
    let cols_req = generic::StringList {
        vals: vec![
            "noteFld".to_string(),
            "deck".to_string(),
            "cardDue".to_string(),
        ],
    };
    let (status, _) = h
        .rpc_raw(
            &alice_cookie,
            &alice_tok,
            "setActiveBrowserColumns",
            cols_req.encode_to_vec(),
        )
        .await;
    assert!(status == StatusCode::OK || status == StatusCode::NO_CONTENT);

    let batch_req = Request::builder()
        .method(Method::POST)
        .uri("/api/browser/rows")
        .header(header::COOKIE, &alice_cookie)
        .header(API_TOKEN_HEADER, &alice_tok)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({ "ids": alice_ids }).to_string(),
        ))
        .unwrap();
    let (status, _, bytes) = h.send(batch_req).await;
    assert_eq!(status, StatusCode::OK);
    let rows_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let rows = rows_json["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["cells"][0]["text"], "Capital of France?");
}

#[tokio::test]
async fn media_serving_csp_range_and_sync_protocol_interop() {
    let h = TestHarness::new();
    let (alice_cookie, alice_tok) = h.login("alice", "secret123").await;
    let (bob_cookie, _) = h.login("bob", "otherpass").await;

    // 1. Add a media file via rslib `addMediaFile` RPC
    let add_media = AddMediaFileRequest {
        desired_name: "hello.txt".to_string(),
        data: b"0123456789abcdef".to_vec(),
    };
    let (status, out) = h
        .rpc_raw(
            &alice_cookie,
            &alice_tok,
            "addMediaFile",
            add_media.encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let saved_name = generic::String::decode(out.as_slice()).unwrap().val;
    assert_eq!(saved_name, "hello.txt");

    // 2. Serve media to Alice -> 200 OK with UNTRUSTED_MEDIA_CSP
    let (status, headers, body) = h
        .send(
            Request::get("/hello.txt")
                .header(header::COOKIE, &alice_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"0123456789abcdef");
    assert_eq!(
        headers
            .get(header::CONTENT_SECURITY_POLICY)
            .unwrap()
            .to_str()
            .unwrap(),
        UNTRUSTED_MEDIA_CSP
    );

    // 3. HTTP Range request -> 206 Partial Content
    let (status, headers, body) = h
        .send(
            Request::get("/hello.txt")
                .header(header::COOKIE, &alice_cookie)
                .header(header::RANGE, "bytes=2-5")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(body, b"2345");
    assert_eq!(
        headers.get(header::CONTENT_RANGE).unwrap().to_str().unwrap(),
        "bytes 2-5/16"
    );

    // 4. Bob cannot read Alice's media
    let (status, _, _) = h
        .send(
            Request::get("/hello.txt")
                .header(header::COOKIE, &bob_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 5. Verify real Anki Sync Client interoperability against a live TCP listener
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app_for_serve = h.app.clone();
    let server_task = tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app_for_serve.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });

    let endpoint = format!("http://{addr}/");
    let http_client = reqwest::Client::builder().http1_only().build().unwrap();

    // Real rslib `sync_login` hits `/sync/hostKey` with Anki's exact zstd wire format
    let mut auth = sync_login(
        "alice",
        "secret123",
        Some(endpoint.parse().unwrap()),
        http_client.clone(),
    )
    .await
    .expect("rslib sync_login against Cardly failed");
    assert!(!auth.hkey.is_empty());
    // `sync_login` clears endpoint back to None (AnkiWeb default), so re-attach our custom server URL.
    auth.endpoint = Some(endpoint.parse().unwrap());

    let sync_client = HttpSyncClient::new(auth, http_client);

    // `/msync/begin` should report media_usn >= 1 because Alice added `hello.txt` via web!
    let begin_resp = sync_client
        .begin(
            SyncBeginRequest {
                client_version: "anki-test,25.0".to_string(),
            }
            .try_into_sync_request()
            .unwrap(),
        )
        .await
        .expect("msync/begin failed")
        .json_result()
        .unwrap();
    assert!(begin_resp.usn.0 >= 1);

    // `/msync/mediaChanges` should include `hello.txt` added via the web app!
    let changes = sync_client
        .media_changes(
            MediaChangesRequest {
                last_usn: anki::prelude::Usn(0),
            }
            .try_into_sync_request()
            .unwrap(),
        )
        .await
        .expect("msync/mediaChanges failed")
        .json_result()
        .unwrap();
    assert!(
        changes.iter().any(|c| c.fname == "hello.txt"),
        "expected hello.txt in mediaChanges, got: {changes:?}"
    );

    // `/sync/meta` switches collection cleanly from Web -> Sync and returns matching media_usn
    let meta = sync_client
        .meta(
            MetaRequest {
                sync_version: SYNC_VERSION_MAX,
                client_version: "anki-test,25.0".to_string(),
            }
            .try_into_sync_request()
            .unwrap(),
        )
        .await
        .expect("sync/meta failed")
        .json()
        .unwrap();
    assert!(meta.media_usn.0 >= 1);

    server_task.abort();

    // Switching back to Web RPC after sync/meta must seamlessly reopen Web collection
    let (status, _) = h
        .rpc_raw(&alice_cookie, &alice_tok, "deckTree", vec![])
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn frontend_settings_image_convert_csv_import_export_and_backups() {
    let h = TestHarness::new();
    let (cookie, token) = h.login("alice", "secret123").await;

    // 1. Profile JSON round-trip
    let set_req = SetSettingJsonRequest {
        key: "theme".to_string(),
        value_json: br#""dark""#.to_vec(),
    };
    let (status, _) = h
        .rpc_raw(&cookie, &token, "setProfileConfigJson", set_req.encode_to_vec())
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, out) = h
        .rpc_raw(
            &cookie,
            &token,
            "getProfileConfigJson",
            generic::String {
                val: "theme".to_string(),
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let val = generic::Json::decode(out.as_slice()).unwrap();
    assert_eq!(val.json, br#""dark""#);

    // 2. Convert pasted image (1x1 PNG -> JPEG)
    let mut png_bytes = Vec::new();
    {
        use image::ImageEncoder;
        let enc = image::codecs::png::PngEncoder::new(&mut png_bytes);
        enc.write_image(&[255, 0, 0, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
    }
    let (status, out) = h
        .rpc_raw(
            &cookie,
            &token,
            "convertPastedImage",
            ConvertPastedImageRequest {
                data: png_bytes,
                ext: "jpg".to_string(),
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let converted = ConvertPastedImageResponse::decode(out.as_slice()).unwrap();
    assert!(converted.data.starts_with(&[0xFF, 0xD8])); // JPEG SOI magic bytes

    // 3. Upload a CSV via multipart, inspect metadata, import it, and export .apkg
    let boundary = "----CardlyTestBoundary";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"vocab.csv\"\r\nContent-Type: text/csv\r\n\r\nbonjour\thello\nmerci\tthank you\n\r\n--{boundary}--\r\n"
    );
    let (status, _, bytes) = h
        .send(
            Request::post("/api/uploads")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(multipart_body))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "upload failed: {}", String::from_utf8_lossy(&bytes));
    let up: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let upload_id = up["upload_id"].as_str().unwrap().to_string();

    let (status, meta_bytes) = h
        .rpc_raw(
            &cookie,
            &token,
            "getCsvMetadata",
            CsvMetadataRequest {
                path: upload_id.clone(),
                ..Default::default()
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "getCsvMetadata failed: {}", String::from_utf8_lossy(&meta_bytes));
    let csv_meta = CsvMetadata::decode(meta_bytes.as_slice()).unwrap();

    let (status, imp_bytes) = h
        .rpc_raw(
            &cookie,
            &token,
            "importCsv",
            ImportCsvRequest {
                path: upload_id,
                metadata: Some(csv_meta),
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "importCsv failed: {}", String::from_utf8_lossy(&imp_bytes));
    let imp = ImportResponse::decode(imp_bytes.as_slice()).unwrap();
    assert_eq!(imp.log.unwrap().new.len(), 2);

    // Export .apkg
    let (status, headers, apkg_bytes) = h
        .send(
            Request::post("/api/export")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"format":"apkg","filename":"french"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("french.apkg"));
    assert!(apkg_bytes.starts_with(b"PK")); // ZIP header

    // Create and list backups
    let (status, _, _) = h
        .send(
            Request::post("/api/backups/create")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, bytes) = h
        .send(
            Request::get("/api/backups")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let backups: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(!backups["backups"].as_array().unwrap().is_empty());

    // Upstream SvelteKit page served with cardly-bridge.js injected
    let (status, _, html) = h
        .send(
            Request::get("/graphs")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let html_str = String::from_utf8_lossy(&html);
    assert!(html_str.contains("/_anki/cardly-bridge.js"));
}

#[tokio::test]
async fn demo_mode_seeding_reset_status_and_abuse_protection() {
    let (h, state) = TestHarness::new_with_demo();

    // 1. Verify /api/demo/status
    let (status, _, bytes) = h
        .send(
            Request::get("/api/demo/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let demo_status: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(demo_status["demo_mode"], true);
    assert_eq!(demo_status["demo_username"], "demo");
    assert_eq!(demo_status["reset_interval_minutes"], 60);
    assert!(demo_status["resets_in_seconds"].as_u64().unwrap() > 0);

    // 2. Login as demo user
    let (demo_cookie, demo_tok) = h.login("demo", "demo123").await;

    // 3. Verify /api/auth/me returns demo_mode: true & is_demo_user: true
    let (status, _, bytes) = h
        .send(
            Request::get("/api/auth/me")
                .header(header::COOKIE, &demo_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let me: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(me["authenticated"], true);
    assert_eq!(me["username"], "demo");
    assert_eq!(me["demo_mode"], true);
    assert_eq!(me["is_demo_user"], true);

    // 4. Seed sample decks for demo user
    let runtime = state.users.get_or_init("demo").unwrap();
    cardly_server::demo::seed_demo_collection_if_empty(&state, &runtime)
        .await
        .expect("seed demo collection");

    // 5. Verify decks created (General Knowledge, Languages, Science)
    let (status, bytes) = h
        .rpc_raw(
            &demo_cookie,
            &demo_tok,
            "getDeckNames",
            anki_proto::decks::GetDeckNamesRequest {
                skip_empty_default: false,
                include_filtered: true,
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let deck_names = anki_proto::decks::DeckNames::decode(bytes.as_slice()).unwrap();
    let names: Vec<&str> = deck_names.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"General Knowledge"), "missing General Knowledge deck");
    assert!(names.contains(&"Languages::Spanish & Kazakh"), "missing Languages deck");
    assert!(names.contains(&"Science & Mathematics"), "missing Science deck");

    // 6. Verify notes exist in collection
    let (status, bytes) = h
        .rpc_raw(
            &demo_cookie,
            &demo_tok,
            "searchNotes",
            anki_proto::search::SearchRequest {
                search: "".to_string(),
                order: None,
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let notes = anki_proto::search::SearchResponse::decode(bytes.as_slice()).unwrap();
    assert!(notes.ids.len() >= 10, "expected at least 10 sample notes, got {}", notes.ids.len());

    // 7. Verify abuse protection blocks dangerous actions for demo user:
    // 7a. Block /api/import/colpkg
    let (status, _, _) = h
        .send(
            Request::post("/api/import/colpkg")
                .header(header::COOKIE, &demo_cookie)
                .header(API_TOKEN_HEADER, &demo_tok)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"upload_id":"fake.colpkg"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "demo user must not import .colpkg");

    // 7b. Block /api/backups/{name}/restore
    let (status, _, _) = h
        .send(
            Request::post("/api/backups/test.colpkg/restore")
                .header(header::COOKIE, &demo_cookie)
                .header(API_TOKEN_HEADER, &demo_tok)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "demo user must not restore backups");

    // 8. Test reset_demo_collection
    cardly_server::demo::reset_demo_collection(&state)
        .await
        .expect("reset demo collection");

    // Re-verify notes still present after reset (since reset re-seeds)
    let (status, bytes) = h
        .rpc_raw(
            &demo_cookie,
            &demo_tok,
            "searchNotes",
            anki_proto::search::SearchRequest {
                search: "".to_string(),
                order: None,
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let notes_after = anki_proto::search::SearchResponse::decode(bytes.as_slice()).unwrap();
    assert!(notes_after.ids.len() >= 10, "expected sample notes re-seeded after reset");

    // 9. Verify regular user (alice) is NOT demo user
    let (alice_cookie, _) = h.login("alice", "secret123").await;
    let (status, _, bytes) = h
        .send(
            Request::get("/api/auth/me")
                .header(header::COOKIE, &alice_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let alice_me: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(alice_me["username"], "alice");
    assert_eq!(alice_me["demo_mode"], true);
    assert_eq!(alice_me["is_demo_user"], false);
}

#[tokio::test]
async fn ai_card_generation_and_batch_add() {
    let h = TestHarness::new();

    // 1. Unauthenticated requests to AI endpoints are rejected
    let (status, _, _) = h
        .send(
            Request::post("/api/ai/generate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"text":"test"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = h
        .send(
            Request::post("/api/ai/batch-add")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"deck_id":1,"notetype_id":1,"cards":[]}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 2. Login as alice
    let (cookie, token) = h.login("alice", "secret123").await;

    // 3. Invalid API token is rejected (403 Forbidden)
    let (status, _, _) = h
        .send(
            Request::post("/api/ai/generate")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, "invalid-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"text":"test"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 4. Mock AI generation: Basic cards
    let (status, _, bytes) = h
        .send(
            Request::post("/api/ai/generate")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{
                        "text": "Spaced repetition and FSRS algorithm overview.",
                        "mode": "basic",
                        "target_count": 2,
                        "language": "en",
                        "api_key": "test-mock-key"
                    }"#,
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let gen_basic: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let cards = gen_basic["cards"].as_array().expect("cards array");
    assert_eq!(cards.len(), 2);
    assert!(!cards[0]["front"].as_str().unwrap().is_empty());
    assert!(!cards[0]["back"].as_str().unwrap().is_empty());
    assert!(!cards[0]["tags"].as_array().unwrap().is_empty());

    // 5. Mock AI generation: Cloze cards
    let (status, _, bytes) = h
        .send(
            Request::post("/api/ai/generate")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{
                        "text": "Spaced repetition uses increasing intervals.",
                        "mode": "cloze",
                        "target_count": 2,
                        "language": "en",
                        "api_key": "test-mock-key"
                    }"#,
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let gen_cloze: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let cloze_cards = gen_cloze["cards"].as_array().expect("cloze cards");
    assert_eq!(cloze_cards.len(), 2);
    assert!(cloze_cards[0]["cloze"].as_str().unwrap().contains("{{c1::"));

    // 6. Multilingual mock generation: Russian
    let (status, _, bytes) = h
        .send(
            Request::post("/api/ai/generate")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{
                        "text": "Интервальное повторение",
                        "mode": "basic",
                        "target_count": 2,
                        "language": "ru",
                        "api_key": "test-mock-key"
                    }"#,
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let gen_ru: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let ru_cards = gen_ru["cards"].as_array().unwrap();
    assert_eq!(ru_cards.len(), 2);
    assert!(ru_cards[0]["front"].as_str().unwrap().contains("повторение"));

    // 7. Get deck ID and notetype IDs via rslib
    let (status, bytes) = h
        .rpc_raw(
            &cookie,
            &token,
            "getDeckNames",
            anki_proto::decks::GetDeckNamesRequest {
                skip_empty_default: false,
                include_filtered: true,
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let deck_names = anki_proto::decks::DeckNames::decode(bytes.as_slice()).unwrap();
    let deck_id = deck_names.entries[0].id;

    let (status, bytes) = h
        .rpc_raw(&cookie, &token, "getNotetypeNames", vec![])
        .await;
    assert_eq!(status, StatusCode::OK);
    let nt_names = anki_proto::notetypes::NotetypeNames::decode(bytes.as_slice()).unwrap();
    let basic_id = nt_names
        .entries
        .iter()
        .find(|e| e.name == "Basic")
        .expect("Basic notetype")
        .id;
    let cloze_id = nt_names
        .entries
        .iter()
        .find(|e| e.name == "Cloze")
        .expect("Cloze notetype")
        .id;

    // 8. Batch add Basic cards
    let (status, _, bytes) = h
        .send(
            Request::post("/api/ai/batch-add")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "deck_id": deck_id,
                        "notetype_id": basic_id,
                        "cards": [
                            {
                                "front": "What is Cardly?",
                                "back": "An open-source Anki-compatible web client and sync server.",
                                "tags": ["ai-test", "cardly"]
                            },
                            {
                                "front": "Which scheduler does Cardly support?",
                                "back": "FSRS and SM-2.",
                                "tags": ["ai-test", "scheduler"]
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let batch_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(batch_resp["added_count"], 2);
    let note_ids = batch_resp["note_ids"].as_array().unwrap();
    assert_eq!(note_ids.len(), 2);

    // Verify notes exist via searchNotes
    let (status, bytes) = h
        .rpc_raw(
            &cookie,
            &token,
            "searchNotes",
            anki_proto::search::SearchRequest {
                search: "tag:ai-test".to_string(),
                order: None,
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let search_res = anki_proto::search::SearchResponse::decode(bytes.as_slice()).unwrap();
    assert_eq!(search_res.ids.len(), 2);

    // 9. Batch add Cloze card with undo verification
    let (status, _, bytes) = h
        .send(
            Request::post("/api/ai/batch-add")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "deck_id": deck_id,
                        "notetype_id": cloze_id,
                        "cards": [
                            {
                                "cloze": "Cardly is powered by official {{c1::rslib}} core.",
                                "tags": ["ai-undo-test"]
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let cloze_batch_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cloze_batch_resp["added_count"], 1);

    // Verify cloze note exists
    let (status, bytes) = h
        .rpc_raw(
            &cookie,
            &token,
            "searchNotes",
            anki_proto::search::SearchRequest {
                search: "tag:ai-undo-test".to_string(),
                order: None,
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let cloze_search = anki_proto::search::SearchResponse::decode(bytes.as_slice()).unwrap();
    assert_eq!(cloze_search.ids.len(), 1);

    // Undo the cloze batch add operation via rpc "undo"
    let (status, _) = h.rpc_raw(&cookie, &token, "undo", vec![]).await;
    assert_eq!(status, StatusCode::OK);

    // Verify note is undone
    let (status, bytes) = h
        .rpc_raw(
            &cookie,
            &token,
            "searchNotes",
            anki_proto::search::SearchRequest {
                search: "tag:ai-undo-test".to_string(),
                order: None,
            }
            .encode_to_vec(),
        )
        .await;
    assert!(status == StatusCode::OK || status == StatusCode::NO_CONTENT);
    let after_undo_search = anki_proto::search::SearchResponse::decode(bytes.as_slice()).unwrap();
    assert_eq!(after_undo_search.ids.len(), 0, "cloze note should be undone");

    // 10. Security & Abuse Testing: Large payload rejection (413)
    let giant_text = "A".repeat(150_000); // 150 KB > 100 KB limit
    let (status, _, _) = h
        .send(
            Request::post("/api/ai/generate")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "text": giant_text,
                        "mode": "basic",
                        "api_key": "mock"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

    // 11. Security & Abuse Testing: Missing API key with no server key (400)
    let (status, _, _) = h
        .send(
            Request::post("/api/ai/generate")
                .header(header::COOKIE, &cookie)
                .header(API_TOKEN_HEADER, &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "text": "Photosynthesis process",
                        "mode": "basic"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 12. Security & Abuse Testing: Rate Limiter exhaustion (429)
    // Send 12 rapid requests to exceed the 10-token bucket
    let mut hit_429 = false;
    for _ in 0..12 {
        let (st, _, _) = h
            .send(
                Request::post("/api/ai/generate")
                    .header(header::COOKIE, &cookie)
                    .header(API_TOKEN_HEADER, &token)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "text": "Quick test note",
                            "mode": "basic",
                            "api_key": "mock"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await;
        if st == StatusCode::TOO_MANY_REQUESTS {
            hit_429 = true;
            break;
        }
    }
    assert!(hit_429, "Expected rate limiter to trigger 429 Too Many Requests");
}

#[tokio::test]
async fn user_registration_guest_session_and_media_cas() {
    let (h, state) = TestHarness::new_with_demo();

    // 1. Unauthenticated request to /api/auth/me returns 401 Unauthorized (does NOT trap visitor in demo)
    let (status, _, bytes) = h
        .send(
            Request::get("/api/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let unauth: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(unauth["authenticated"], false);
    assert_eq!(unauth["demo_mode"], true);

    // 2. Register new user "charlie"
    let (status, headers, bytes) = h
        .send(
            Request::post("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": "charlie",
                        "password": "strongpassword123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let cookie_hdr = headers.get(header::SET_COOKIE).expect("set-cookie on register");
    let reg_cookie = cookie_hdr.to_str().unwrap().split(';').next().unwrap().to_string();
    let reg_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(reg_resp["authenticated"], true);
    assert_eq!(reg_resp["username"], "charlie");
    assert_eq!(reg_resp["is_demo_user"], false);
    let charlie_token = reg_resp["api_token"].as_str().unwrap().to_string();

    // 3. Validation: Duplicate username registration is rejected
    let (status, _, bytes) = h
        .send(
            Request::post("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": "charlie",
                        "password": "anotherpassword123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let dup_err: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(dup_err["error"].as_str().unwrap().contains("already taken"));

    // 4. Validation: Short password is rejected
    let (status, _, bytes) = h
        .send(
            Request::post("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": "david",
                        "password": "123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let short_err: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(short_err["error"].as_str().unwrap().contains("at least 6 characters"));

    // 5. Validation: Reserved name is rejected
    let (status, _, bytes) = h
        .send(
            Request::post("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": "demo",
                        "password": "somepassword123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let res_err: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(res_err["error"].as_str().unwrap().contains("reserved"));

    // 6. Charlie can use authenticated RPC to create a deck
    let (status, _bytes) = h
        .rpc_raw(
            &reg_cookie,
            &charlie_token,
            "addDeck",
            anki_proto::decks::Deck {
                id: 0,
                name: "Charlie Deck".to_string(),
                mtime_secs: 0,
                usn: 0,
                common: None,
                kind: Some(anki_proto::decks::deck::Kind::Normal(
                    anki_proto::decks::deck::Normal::default(),
                )),
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 7. Test Logout and Login back as Charlie
    let (status, headers, _) = h
        .send(
            Request::post("/api/auth/logout")
                .header(header::COOKIE, &reg_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let clear_cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(clear_cookie.contains("Max-Age=0") || clear_cookie.contains("cardly_session=;"));

    // Login back with Charlie's credentials
    let (status, _, bytes) = h
        .send(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": "charlie",
                        "password": "strongpassword123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let login_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(login_resp["username"], "charlie");
    assert_eq!(login_resp["authenticated"], true);

    // 8. Ephemeral Guest Session
    let (status, guest_headers, bytes) = h
        .send(
            Request::post("/api/auth/guest")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let guest_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(guest_resp["authenticated"], true);
    assert_eq!(guest_resp["is_demo_user"], true);
    let guest_uname = guest_resp["username"].as_str().unwrap();
    assert!(guest_uname.starts_with("guest_"));
    let guest_cookie = guest_headers
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap();

    // Verify guest has sample decks seeded
    let guest_tok = guest_resp["api_token"].as_str().unwrap();
    let (status, bytes) = h
        .rpc_raw(
            guest_cookie,
            guest_tok,
            "getDeckNames",
            anki_proto::decks::GetDeckNamesRequest {
                skip_empty_default: false,
                include_filtered: true,
            }
            .encode_to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let deck_names = anki_proto::decks::DeckNames::decode(bytes.as_slice()).unwrap();
    assert!(deck_names.entries.iter().any(|d| d.name == "General Knowledge"));

    // 9. Media CAS deduplication test
    let u1_dir = state.config.data_dir.join("users/charlie/media");
    let u2_dir = state.config.data_dir.join(format!("users/{guest_uname}/media"));
    let test_media_data = b"Shared deck audio sound byte [sound:pronounce.mp3]";
    let f1 = u1_dir.join("pronounce.mp3");
    let f2 = u2_dir.join("pronounce.mp3");

    cardly_server::cas::store_media_deduplicated(&state.config.data_dir, &f1, test_media_data).unwrap();
    cardly_server::cas::store_media_deduplicated(&state.config.data_dir, &f2, test_media_data).unwrap();

    assert_eq!(std::fs::read(&f1).unwrap(), test_media_data);
    assert_eq!(std::fs::read(&f2).unwrap(), test_media_data);

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let m1 = std::fs::metadata(&f1).unwrap();
        let m2 = std::fs::metadata(&f2).unwrap();
        assert_eq!(m1.ino(), m2.ino(), "Deduplicated files across users must share the same inode");
    }
}

