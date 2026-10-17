mod support;
use bookbuddy_lib::{
    domain::{
        backups::*,
        books::{BookDraft, BookFilter},
        custom_fields::ValueTarget,
        readings::ReadingDraft,
    },
    persistence::{self, backups as repository},
    services::{
        backups, books, custom_fields, household, import_commit, import_preview, readings,
        xlsx_reader,
    },
};
use sqlx::{Row, SqlitePool};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

async fn records(pool: &SqlitePool) -> Vec<Vec<String>> {
    let mut output = Vec::new();
    for table in [
        "person",
        "book",
        "person_book",
        "app_settings",
        "custom_field_definition",
        "custom_field_option",
        "book_custom_field_value",
        "person_book_custom_field_value",
        "reading_event",
        "reading_event_participant",
        "import_source",
        "import_batch",
        "import_source_row",
        "library_borrow_event",
        "book_borrow_summary",
        "book_title_verification",
        "import_field_mapping",
        "import_option_mapping",
        "import_catalogue_identity",
    ] {
        // Test-only schema comparison. Production IPC never accepts SQL or names.
        let columns = sqlx::query(&format!("PRAGMA table_info({table})"))
            .fetch_all(pool)
            .await
            .unwrap()
            .iter()
            .map(|row| format!("\"{}\"", row.get::<String, _>("name")))
            .collect::<Vec<_>>()
            .join(",");
        output.push(
            sqlx::query_scalar(&format!(
                "SELECT json_array({columns}) AS record FROM {table} ORDER BY record"
            ))
            .fetch_all(pool)
            .await
            .unwrap(),
        );
    }
    output
}
async fn populated(directory: &Path) -> SqlitePool {
    let pool = persistence::open(directory).await.unwrap();
    let file = directory.join("synthetic.xlsx");
    support::xlsx::write(&file, &support::xlsx::family(), false);
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let mut draft = import_preview::initial_draft(&pool, &workbook.source_name, "synthetic")
        .await
        .unwrap();
    draft.acknowledge_warnings = true;
    let preview = import_preview::preview(&pool, &workbook, draft)
        .await
        .unwrap();
    let summary = import_commit::commit(
        &pool,
        directory,
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash,
    )
    .await
    .unwrap();
    let book = books::list(&pool, BookFilter::default())
        .await
        .unwrap()
        .remove(0)
        .book
        .id;
    readings::log(
        &pool,
        ReadingDraft {
            book_id: book.clone(),
            occurred_on: "2020-01-02".into(),
            note: Some("Synthetic shared reading".into()),
            participant_ids: vec![summary.carly_person_id.clone()],
            set_read: false,
        },
    )
    .await
    .unwrap();
    let field = custom_fields::list(&pool)
        .await
        .unwrap()
        .into_iter()
        .find(|field| field.draft.label == "Favourite?")
        .unwrap();
    custom_fields::set_value(
        &pool,
        ValueTarget::PersonBook {
            person_id: summary.carly_person_id,
            book_id: book,
        },
        &field.id,
        Some(serde_json::json!(false)),
    )
    .await
    .unwrap();
    let mom = household::list(&pool)
        .await
        .unwrap()
        .people
        .into_iter()
        .find(|person| person.display_name == "Mom")
        .unwrap();
    household::archive(&pool, &mom.id).await.unwrap();
    pool
}
fn rewrite_archive(
    source: &Path,
    destination: &Path,
    mutate: impl FnOnce(&mut Manifest, &mut Vec<u8>),
) {
    let mut archive = zip::ZipArchive::new(fs::File::open(source).unwrap()).unwrap();
    let mut manifest: Manifest =
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
    let mut database = Vec::new();
    archive
        .by_name("database.sqlite3")
        .unwrap()
        .read_to_end(&mut database)
        .unwrap();
    mutate(&mut manifest, &mut database);
    let mut output = zip::ZipWriter::new(fs::File::create(destination).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    output.start_file("manifest.json", options).unwrap();
    output
        .write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    output.start_file("database.sqlite3", options).unwrap();
    output.write_all(&database).unwrap();
    output.finish().unwrap();
}

#[tokio::test]
async fn a_complete_wal_library_round_trips_without_losing_any_records_or_provenance() {
    let directory = tempfile::tempdir().unwrap();
    let pool = populated(directory.path()).await;
    let before = records(&pool).await;
    let destination = directory.path().join("portable.bookbuddy");
    let manifest = backups::export(&pool, directory.path(), &destination)
        .await
        .unwrap();
    assert_eq!(manifest.version, 1);
    assert!(backups::export(&pool, directory.path(), &destination)
        .await
        .is_err());
    let selected = backups::inspect(&destination, directory.path())
        .await
        .unwrap();
    assert_eq!(before, records(&pool).await);
    assert_eq!(selected.counts.source_rows, 5);
    assert_eq!(selected.counts.readings, 1);
    books::create(
        &pool,
        BookDraft {
            title: "Later manual addition".into(),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    let preview = backups::preview(&pool, &selected, "token").await.unwrap();
    assert_eq!(preview.current_counts.books, 3);
    assert_eq!(preview.backup_counts.books, 2);
    let recovery = backups::restore(&pool, directory.path(), &selected, &preview.review_hash)
        .await
        .unwrap();
    assert!(pool.is_closed());
    let reopened = persistence::open(directory.path()).await.unwrap();
    assert_eq!(before, records(&reopened).await);
    let recovery = backups::inspect(
        &directory.path().join("backups").join(recovery),
        directory.path(),
    )
    .await
    .unwrap();
    assert_eq!(recovery.counts.books, 3);
    assert!(sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&reopened)
        .await
        .unwrap()
        .is_empty());
    reopened.close().await;
}

#[tokio::test]
async fn stale_review_changed_archives_and_incompatible_databases_leave_the_library_untouched() {
    let directory = tempfile::tempdir().unwrap();
    let pool = populated(directory.path()).await;
    let destination = directory.path().join("portable.bookbuddy");
    backups::export(&pool, directory.path(), &destination)
        .await
        .unwrap();
    let selected = backups::inspect(&destination, directory.path())
        .await
        .unwrap();
    let preview = backups::preview(&pool, &selected, "token").await.unwrap();
    books::create(
        &pool,
        BookDraft {
            title: "Newer data".into(),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    let before = records(&pool).await;
    assert!(
        backups::restore(&pool, directory.path(), &selected, &preview.review_hash)
            .await
            .is_err()
    );
    assert_eq!(before, records(&pool).await);
    for (name, mutation) in [
        ("future", 0),
        ("checksum", 1),
        ("identity", 2),
        ("schema", 3),
        ("links", 4),
    ] {
        let corrupt = directory.path().join(format!("{name}.bookbuddy"));
        if mutation < 2 {
            rewrite_archive(&destination, &corrupt, |manifest, bytes| {
                if mutation == 0 {
                    manifest.schema_version = 9999;
                } else {
                    bytes[100] ^= 1;
                }
            });
        } else {
            let unpacked = directory.path().join(format!("{name}.sqlite3"));
            let mut zip = zip::ZipArchive::new(fs::File::open(&destination).unwrap()).unwrap();
            let mut bytes = Vec::new();
            zip.by_name("database.sqlite3")
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            fs::write(&unpacked, bytes).unwrap();
            let changed = SqlitePool::connect_with(
                sqlx::sqlite::SqliteConnectOptions::new().filename(&unpacked),
            )
            .await
            .unwrap();
            sqlx::raw_sql(match mutation {2=>"PRAGMA application_id = 7",3=>"CREATE TRIGGER unexpected AFTER INSERT ON book BEGIN DELETE FROM person_book; END",_=>"PRAGMA foreign_keys=OFF; UPDATE person_book SET book_id='missing' WHERE id=(SELECT id FROM person_book LIMIT 1)"}).execute(&changed).await.unwrap();
            changed.close().await;
            rewrite_archive(&destination, &corrupt, |manifest, bytes| {
                *bytes = fs::read(&unpacked).unwrap();
                manifest.database_sha256 = xlsx_reader::hash(bytes);
                manifest.database_bytes = bytes.len() as u64;
            });
        }
        assert!(
            backups::inspect(&corrupt, directory.path()).await.is_err(),
            "{name}"
        );
        assert_eq!(before, records(&pool).await);
    }
    let preview = backups::preview(&pool, &selected, "token").await.unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&destination)
        .unwrap()
        .write_all(b"changed")
        .unwrap();
    assert!(
        backups::restore(&pool, directory.path(), &selected, &preview.review_hash)
            .await
            .is_err()
    );
    assert_eq!(before, records(&pool).await);
    pool.close().await;
}

#[tokio::test]
async fn incomplete_sqlite_replacement_rolls_back_and_automatic_rotation_keeps_recovery_files() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    fs::create_dir_all(&source).unwrap();
    let pool = populated(&source).await;
    let archive = directory.path().join("source.bookbuddy");
    backups::export(&pool, &source, &archive).await.unwrap();
    let selected = backups::inspect(&archive, directory.path()).await.unwrap();
    let target = directory.path().join("target");
    let current = persistence::open(&target).await.unwrap();
    books::create(
        &current,
        BookDraft {
            title: "Original target".into(),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    let before = records(&current).await;
    assert!(
        repository::replace_from_snapshot(&current, &selected.snapshot(), Some(1))
            .await
            .is_err()
    );
    assert_eq!(before, records(&current).await);
    let folder = target.join("backups");
    fs::create_dir_all(&folder).unwrap();
    fs::write(
        folder.join("pre-restore-retain.bookbuddy"),
        b"recovery marker",
    )
    .unwrap();
    fs::write(folder.join("manual-retain.bookbuddy"), b"manual marker").unwrap();
    fs::write(
        folder.join("automatic-unrecognized.bookbuddy"),
        b"unknown marker",
    )
    .unwrap();
    repository::save_settings(
        &current,
        &BackupSettings {
            enabled: true,
            keep: 2,
        },
    )
    .await
    .unwrap();
    for _ in 0..4 {
        backups::automatic(&current, &target, true).await.unwrap();
    }
    assert_eq!(
        backups::status(&current, &target, false)
            .await
            .unwrap()
            .automatic_count,
        2
    );
    assert!(folder.join("pre-restore-retain.bookbuddy").exists());
    assert!(folder.join("manual-retain.bookbuddy").exists());
    assert!(folder.join("automatic-unrecognized.bookbuddy").exists());
    let count = fs::read_dir(&folder).unwrap().count();
    backups::automatic(&current, &target, false).await.unwrap();
    assert_eq!(count, fs::read_dir(folder).unwrap().count());
    assert!(repository::save_settings(
        &current,
        &BackupSettings {
            enabled: true,
            keep: 0
        }
    )
    .await
    .is_err());
    current.close().await;
    pool.close().await;
}

#[tokio::test]
async fn unsafe_archives_and_exclusive_directory_access_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("unsafe.bookbuddy");
    let mut zip = zip::ZipWriter::new(fs::File::create(&file).unwrap());
    zip.start_file(
        "../bookbuddy.sqlite3",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"unsafe").unwrap();
    zip.finish().unwrap();
    assert!(backups::inspect(&file, directory.path()).await.is_err());
    let first = persistence::lock_directory(directory.path()).unwrap();
    #[cfg(windows)]
    assert!(persistence::lock_directory(directory.path()).is_err());
    drop(first);
    assert!(persistence::lock_directory(directory.path()).is_ok());
}

#[test]
#[ignore = "Subprocess worker used only with disposable synthetic paths by the crash-recovery test"]
fn abrupt_restore_worker() {
    let Ok(source) = std::env::var("BOOKBUDDY_CRASH_SOURCE") else {
        return;
    };
    let target = std::env::var("BOOKBUDDY_CRASH_TARGET").unwrap();
    let source = std::ffi::CString::new(source).unwrap();
    let target = std::ffi::CString::new(target).unwrap();
    // Test-only deliberate termination with an unfinished SQLite destination transaction.
    unsafe {
        let mut source_db = std::ptr::null_mut();
        let mut target_db = std::ptr::null_mut();
        assert_eq!(
            libsqlite3_sys::sqlite3_open_v2(
                source.as_ptr(),
                &mut source_db,
                libsqlite3_sys::SQLITE_OPEN_READONLY,
                std::ptr::null()
            ),
            0
        );
        assert_eq!(
            libsqlite3_sys::sqlite3_open(target.as_ptr(), &mut target_db),
            0
        );
        let backup = libsqlite3_sys::sqlite3_backup_init(
            target_db,
            c"main".as_ptr(),
            source_db,
            c"main".as_ptr(),
        );
        assert!(!backup.is_null());
        assert_eq!(
            libsqlite3_sys::sqlite3_backup_step(backup, 1),
            libsqlite3_sys::SQLITE_OK
        );
        std::process::exit(73);
    }
}
#[tokio::test]
async fn abrupt_process_exit_during_restore_recovers_the_original_wal_dataset() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    fs::create_dir_all(&source).unwrap();
    let pool = populated(&source).await;
    let snapshot = persistence::snapshots::snapshot(&pool, directory.path(), "manual")
        .await
        .unwrap();
    pool.close().await;
    let target = directory.path().join("target");
    let pool = persistence::open(&target).await.unwrap();
    books::create(
        &pool,
        BookDraft {
            title: "Preserve after process exit".into(),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    let before = records(&pool).await;
    pool.close().await;
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "abrupt_restore_worker", "--ignored"])
        .env("BOOKBUDDY_CRASH_SOURCE", snapshot)
        .env("BOOKBUDDY_CRASH_TARGET", target.join("bookbuddy.sqlite3"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
    let recovered = persistence::open(&target).await.unwrap();
    assert_eq!(before, records(&recovered).await);
    assert_eq!(
        sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
            .fetch_one(&recovered)
            .await
            .unwrap(),
        "ok"
    );
    recovered.close().await;
}

#[tokio::test]
async fn older_compatible_backups_upgrade_only_the_staged_copy_and_covers_are_never_silently_omitted(
) {
    let directory = tempfile::tempdir().unwrap();
    let legacy = directory.path().join("legacy");
    fs::create_dir_all(&legacy).unwrap();
    let pool = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(legacy.join("bookbuddy.sqlite3"))
            .create_if_missing(true),
    )
    .await
    .unwrap();
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.migrations = std::borrow::Cow::Owned(
        migrator
            .iter()
            .filter(|migration| migration.version <= 2)
            .cloned()
            .collect(),
    );
    migrator.run(&pool).await.unwrap();
    sqlx::query("INSERT INTO book (id, title) VALUES (?, 'Legacy synthetic title')")
        .bind(uuid::Uuid::new_v4().to_string())
        .execute(&pool)
        .await
        .unwrap();
    let file = directory.path().join("legacy.bookbuddy");
    backups::export(&pool, &legacy, &file).await.unwrap();
    let original = fs::read(&file).unwrap();
    let selected = backups::inspect(&file, directory.path()).await.unwrap();
    assert_eq!(selected.manifest.schema_version, 2);
    assert_eq!(selected.counts.books, 1);
    assert_eq!(selected.counts.source_rows, 0);
    assert_eq!(fs::read(file).unwrap(), original);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    sqlx::query("UPDATE book SET cover_path = 'unavailable-local-cover.png'")
        .execute(&pool)
        .await
        .unwrap();
    let unsupported = directory.path().join("covers.bookbuddy");
    assert!(backups::export(&pool, &legacy, &unsupported).await.is_err());
    assert!(!unsupported.exists());
    pool.close().await;
}
