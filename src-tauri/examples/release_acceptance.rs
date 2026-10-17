//! Run only through smoke-signed-upgrade.ps1; never targets the household identity.
use bookbuddy_lib::{
    domain::{books::BookDraft, readings::ReadingDraft},
    persistence,
    services::{books, household, readings, updates},
};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri_plugin_updater::UpdaterExt;

fn main() {
    let id = uuid::Uuid::parse_str(
        &std::env::var("BOOKBUDDY_UPGRADE_RUN").expect("Explicit isolated run ID required"),
    )
    .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".local")
        .join("signed-upgrade");
    let data = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("local.bookbuddy.updater-acceptance")
        .join("acceptance")
        .join(id.to_string());
    let mode = std::env::args().nth(1).expect("seed, verify or install");
    tauri::async_runtime::block_on(async {
        let pool = persistence::open(&data).await.unwrap();
        if mode == "seed" {
            assert!(books::list(&pool, Default::default())
                .await
                .unwrap()
                .is_empty());
            household::create(&pool, "Synthetic signed upgrade reader")
                .await
                .unwrap();
            let person = household::list(&pool)
                .await
                .unwrap()
                .current_person_id
                .unwrap();
            let book = books::create(
                &pool,
                BookDraft {
                    title: "Survives signed upgrade".into(),
                    notes: Some("Synthetic recovery notes".into()),
                    ..Default::default()
                },
                Some(&person),
            )
            .await
            .unwrap();
            for _ in 0..2 {
                readings::log(
                    &pool,
                    ReadingDraft {
                        book_id: book.clone(),
                        occurred_on: "2020-01-01".into(),
                        participant_ids: vec![person.clone()],
                        note: None,
                        set_read: true,
                    },
                )
                .await
                .unwrap();
            }
            std::fs::write(
                root.join("before.hash"),
                persistence::backups::state_hash(&pool).await.unwrap(),
            )
            .unwrap();
            println!("PASS: isolated book, person, notes and repeated readings seeded");
        } else if mode == "verify" {
            assert_eq!(
                persistence::backups::state_hash(&pool).await.unwrap(),
                std::fs::read_to_string(root.join("before.hash")).unwrap()
            );
            assert_eq!(
                books::list(&pool, Default::default()).await.unwrap().len(),
                1
            );
            println!("PASS: every SQLite record preserved after signed installer upgrade/restart");
        } else {
            assert_eq!(mode, "install");
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let bytes =
                std::fs::read(root.join("BookBuddyUpdaterAcceptance_0.1.1_x64-setup.exe")).unwrap();
            let signature = std::fs::read_to_string(
                root.join("BookBuddyUpdaterAcceptance_0.1.1_x64-setup.exe.sig"),
            )
            .unwrap();
            let manifest = json!({"version":"0.1.1","url":format!("{base}/installer"),"signature":signature.trim()}).to_string();
            let stopped = Arc::new(AtomicBool::new(false));
            let stop = stopped.clone();
            let server = std::thread::spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    if let Ok((mut stream, _)) = listener.accept() {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut request = [0; 4096];
                        let n = stream.read(&mut request).unwrap();
                        let body = if String::from_utf8_lossy(&request[..n])
                            .starts_with("GET /installer ")
                        {
                            bytes.as_slice()
                        } else {
                            manifest.as_bytes()
                        };
                        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
                        stream.write_all(body).unwrap();
                    } else {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
            });
            let mut context = tauri::generate_context!();
            context.package_info_mut().version = "0.1.0".parse().unwrap();
            let public = context.config().plugins.0["updater"]["pubkey"].clone();
            // Loopback HTTP and quiet installation are confined to this fixture context.
            context.config_mut().plugins.0.insert("updater".into(), json!({"pubkey":public,"requireSignedVersion":true,"dangerousInsecureTransportProtocol":true,"windows":{"installMode":"quiet"}}));
            let app = tauri::test::mock_builder()
                .plugin(tauri_plugin_updater::Builder::new().build())
                .build(context)
                .unwrap();
            let update = app
                .updater_builder()
                .endpoints(vec![format!("{base}/manifest").parse().unwrap()])
                .unwrap()
                .timeout(Duration::from_secs(15))
                .no_proxy()
                .installer_arg(format!("/D={}", root.join("installed").display()))
                .build()
                .unwrap()
                .check()
                .await
                .unwrap()
                .unwrap()
                .restart_after_install(false);
            let bytes = update.download(|_, _| {}, || {}).await.unwrap();
            stopped.store(true, Ordering::Release);
            server.join().unwrap();
            let recovery = updates::recovery_backup(&pool, &data).await.unwrap();
            assert!(std::path::Path::new(&recovery).exists());
            pool.close().await;
            println!(
                "PASS: signed version and installer verified; recovery saved before SDK handoff"
            );
            update.install(bytes).unwrap();
            unreachable!("SDK exits after successful Windows installer launch");
        }
        pool.close().await;
    });
}
