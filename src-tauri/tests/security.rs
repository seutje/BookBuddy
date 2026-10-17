use bookbuddy_lib::{
    domain::books::*,
    persistence,
    services::{books, household},
};
use std::{collections::BTreeSet, path::Path};

#[test]
fn capabilities_csp_and_frontend_sinks_keep_a_narrow_local_boundary() {
    let manifest = include_str!("windows.manifest");
    assert!(manifest.contains("requestedExecutionLevel level=\"asInvoker\""));
    assert!(include_str!("../build.rs").contains("include_str!(\"tests/windows.manifest\")"));
    let capabilities: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
    assert!(capabilities.get("remote").is_none());
    let permissions: BTreeSet<String> = capabilities["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|permission| {
            permission
                .as_str()
                .unwrap()
                .strip_prefix("allow-")
                .expect("No generic plugin or core permissions")
                .replace('-', "_")
        })
        .collect();
    let commands: BTreeSet<String> = include_str!("../build.rs")
        .split(".commands(&[")
        .nth(1)
        .expect("Explicit application command list")
        .split("])")
        .next()
        .unwrap()
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix('"')?
                .strip_suffix("\",")
                .map(str::to_owned)
        })
        .collect();
    assert_eq!(permissions, commands);
    for command in &permissions {
        assert!(
            !["execute", "query", "sql", "shell", "filesystem", "http"]
                .iter()
                .any(|term| command.contains(term)),
            "Unexpected broad command"
        );
    }
    let configuration: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let csp = configuration["app"]["security"]["csp"].as_str().unwrap();
    for required in [
        "script-src 'self'",
        "connect-src ipc: http://ipc.localhost",
        "object-src 'none'",
        "frame-src 'none'",
        "base-uri 'none'",
    ] {
        assert!(csp.contains(required));
    }
    assert!(!csp.contains("unsafe-eval"));
    assert!(!csp.contains("https:"));
    assert_eq!(
        configuration["bundle"]["windows"]["webviewInstallMode"]["type"],
        "skip"
    );
    fn inspect(directory: &Path) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                inspect(&path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "ts" || extension == "tsx")
            {
                let source = std::fs::read_to_string(path).unwrap();
                for sink in [
                    "dangerouslySetInnerHTML",
                    ".innerHTML",
                    ".outerHTML",
                    "eval(",
                    "new Function(",
                    "fetch(",
                    "XMLHttpRequest",
                    "WebSocket(",
                ] {
                    assert!(
                        !source.contains(sink),
                        "Unreviewed HTML/execution/network sink: {sink}"
                    );
                }
            }
        }
    }
    inspect(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("src"),
    );
}

#[tokio::test]
async fn external_strings_remain_literal_and_invalid_ids_cannot_select_paths_or_sql() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    household::create(&pool, "Synthetic reader").await.unwrap();
    let title = "<img src=x onerror=alert(1)>";
    let id = books::create(
        &pool,
        BookDraft {
            title: title.into(),
            notes: Some("'; DROP TABLE book; --".into()),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(books::get(&pool, &id).await.unwrap().book.data.title, title);
    assert_eq!(
        books::list(
            &pool,
            BookFilter {
                search: "'; DROP TABLE book; --".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .len(),
        1
    );
    assert!(books::list(
        &pool,
        BookFilter {
            search: "%' OR 1=1 --".into(),
            ..Default::default()
        }
    )
    .await
    .unwrap()
    .is_empty());
    for invalid in [
        "../bookbuddy.sqlite3",
        "SELECT * FROM book",
        "https://example.invalid",
    ] {
        assert!(books::get(&pool, invalid).await.is_err());
    }
    assert!(
        serde_json::from_value::<bookbuddy_lib::domain::import::ImportReviewDraft>(
            serde_json::json!({"path":"../private.xlsx"})
        )
        .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM book")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    pool.close().await;
}
