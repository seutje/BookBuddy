//! Owned loopback fixtures exercise the actual SDK; no installer is executed.
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::Duration,
};
use tauri::test::mock_builder;
use tauri_plugin_updater::UpdaterExt;

#[tokio::test]
#[ignore = "Explicit public-feed acceptance after publishing; downloads but never executes an installer"]
async fn published_feed_verifies_the_actual_installer_with_bookbuddy_key() {
    let mut context = tauri::generate_context!();
    context.package_info_mut().version = "0.0.0".parse().unwrap();
    let app = mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let update = app
        .updater_builder()
        .timeout(Duration::from_secs(120))
        .no_proxy()
        .build()
        .unwrap()
        .check()
        .await
        .unwrap()
        .expect("Published updater feed");
    assert_eq!(update.version, env!("CARGO_PKG_VERSION"));
    assert!(bookbuddy_lib::services::updates::allowed_artifact(
        &update.download_url
    ));
    let bytes = update.download(|_, _| {}, || {}).await.unwrap();
    assert!(!bytes.is_empty());
}

#[tokio::test]
async fn recovery_archive_preserves_records_and_never_rotates_with_automatic_backups() {
    use bookbuddy_lib::{
        persistence,
        services::{backups, books, household, updates},
    };
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    household::create(&pool, "Synthetic update reader")
        .await
        .unwrap();
    books::create(
        &pool,
        bookbuddy_lib::domain::books::BookDraft {
            title: "Update recovery fixture".into(),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    let path = updates::recovery_backup(&pool, directory.path())
        .await
        .unwrap();
    assert!(std::path::Path::new(&path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("pre-update-"));
    let preview = backups::inspect(std::path::Path::new(&path), directory.path())
        .await
        .unwrap();
    assert_eq!(preview.counts.books, 1);
    assert_eq!(preview.counts.people, 1);
    backups::automatic(&pool, directory.path(), true)
        .await
        .unwrap();
    assert!(std::path::Path::new(&path).exists());
    let blocked = tempfile::tempdir().unwrap();
    std::fs::write(blocked.path().join("backups"), "blocks recovery folder").unwrap();
    assert!(updates::recovery_backup(&pool, blocked.path())
        .await
        .is_err());
    assert_eq!(
        books::list(&pool, Default::default()).await.unwrap().len(),
        1
    );
    pool.close().await;
}

#[test]
fn updater_feed_and_permissions_are_fixed_and_do_not_grant_plugin_ipc() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    assert_eq!(config["plugins"]["updater"]["requireSignedVersion"], true);
    assert_eq!(
        config["plugins"]["updater"]["endpoints"],
        json!(["https://github.com/seutje/BookBuddy/releases/latest/download/latest.json"])
    );
    assert_eq!(config["bundle"]["createUpdaterArtifacts"], true);
    let permissions: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
    assert!(permissions.get("remote").is_none());
    assert!(permissions["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|value| !value.as_str().unwrap().starts_with("updater:")));
}

#[test]
fn native_commands_enforce_local_origin_and_explicit_consent() {
    use bookbuddy_lib::{
        commands::{self, AppState},
        persistence,
        services::updates::UpdateState,
    };
    use tauri::{
        ipc::{CallbackFn, InvokeBody},
        test::{get_ipc_response, INVOKE_KEY},
        webview::InvokeRequest,
        Manager,
    };
    let directory = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let pool = runtime
        .block_on(persistence::open(directory.path()))
        .unwrap();
    let app = mock_builder()
        .manage(AppState {
            operations: tokio::sync::RwLock::new(()),
            restored: false.into(),
            automatic_problem: false.into(),
            restore_selection: tokio::sync::Mutex::new(None),
            directory_lock: None,
            pool: Some(pool.clone()),
            data_path: Some(directory.path().into()),
            data_directory: None,
            workbooks: Default::default(),
        })
        .manage(UpdateState::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::updates::check_app_update,
            commands::updates::install_app_update
        ])
        .build(tauri::generate_context!())
        .unwrap();
    let window = app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap()
    });
    let call = |command: &str, origin: &str, body: serde_json::Value| {
        get_ipc_response(
            &window,
            InvokeRequest {
                cmd: command.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: origin.parse().unwrap(),
                body: InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.into(),
            },
        )
    };
    for command in [
        "check_app_update",
        "install_app_update",
        "plugin:updater|check",
    ] {
        assert!(call(command, "https://example.com", json!({})).is_err());
    }
    assert!(call("plugin:updater|check", "http://tauri.localhost", json!({})).is_err());
    let check: serde_json::Value = call("check_app_update", "http://tauri.localhost", json!({}))
        .unwrap()
        .deserialize()
        .unwrap();
    assert_eq!(check, json!({"enabled":false,"update":null}));
    for confirmed in [false, true] {
        let result = call(
            "install_app_update",
            "http://tauri.localhost",
            json!({"version":"0.2.0", "confirmed":confirmed, "progress":"__CHANNEL__:0"}),
        );
        assert!(result.is_err());
    }
    assert!(!directory.path().join("backups").exists());
    runtime.block_on(pool.close());
}

#[tokio::test]
async fn signed_download_rejects_tampering_wrong_version_and_network_failure() {
    for (version, tamper, fail, expected) in [
        ("0.2.0", false, false, true),
        ("0.2.0", true, false, false),
        ("0.3.0", false, false, false),
        ("0.2.0", false, true, false),
        ("0.1.0", false, false, false),
        ("0.0.9", false, false, false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let manifest = json!({"version":version,"notes":"Fixture only", "url":format!("{base}/payload"),"signature":include_str!("../../tests/fixtures/updater-payload.txt.sig").trim()}).to_string();
        let stopped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop = stopped.clone();
        let server = std::thread::spawn(move || {
            while !stop.load(std::sync::atomic::Ordering::Acquire) {
                if let Ok((mut stream, _)) = listener.accept() {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    let mut request = [0; 4096];
                    let n = stream.read(&mut request).unwrap();
                    let payload =
                        String::from_utf8_lossy(&request[..n]).starts_with("GET /payload ");
                    let body = if payload {
                        if tamper {
                            "tampered"
                        } else {
                            include_str!("../../tests/fixtures/updater-payload.txt")
                        }
                    } else {
                        &manifest
                    };
                    let status = if payload && fail {
                        "500 Internal Server Error"
                    } else {
                        "200 OK"
                    };
                    write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                } else {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        });
        let mut context = tauri::generate_context!();
        // Keep the mock older than the signed 0.2.0 fixture across app releases.
        context.package_info_mut().version = "0.1.0".parse().unwrap();
        let public = context.config().plugins.0["updater"]["pubkey"].clone();
        context.config_mut().plugins.0.insert("updater".into(), json!({"pubkey":public,"requireSignedVersion":true,"dangerousInsecureTransportProtocol":true}));
        let app = mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        let update = app
            .updater_builder()
            .endpoints(vec![format!("{base}/manifest").parse().unwrap()])
            .unwrap()
            .timeout(Duration::from_secs(3))
            .no_proxy()
            .build()
            .unwrap()
            .check()
            .await
            .unwrap();
        if version == "0.1.0" || version == "0.0.9" {
            assert!(update.is_none());
        } else {
            assert_eq!(
                update.unwrap().download(|_, _| {}, || {}).await.is_ok(),
                expected
            );
        }
        stopped.store(true, std::sync::atomic::Ordering::Release);
        server.join().unwrap();
    }
}
