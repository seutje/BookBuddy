fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "startup_status",
            "list_people",
            "create_person",
            "rename_person",
            "archive_person",
            "select_person",
            "list_books",
            "get_book",
            "create_book",
            "update_book",
            "ensure_person_book",
            "update_person_book",
            "mark_read",
            "duplicate_book_hints",
        ]),
    ))
    .expect("Tauri build configuration failed");
}
