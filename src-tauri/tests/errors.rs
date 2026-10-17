use bookbuddy_lib::domain::AppError;

#[test]
fn ipc_storage_errors_do_not_reveal_source_text_or_local_paths() {
    let error = AppError::from(sqlx::Error::Protocol(
        "private note at C:/household/library.sqlite3".into(),
    ));
    let serialized = serde_json::to_string(&error).unwrap();
    assert!(serialized.contains("\"code\":\"storage\""));
    assert!(!serialized.contains("private note"));
    assert!(!serialized.contains("C:/household"));
}
