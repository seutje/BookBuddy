fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "startup_status",
            "list_people",
            "create_person",
            "rename_person",
            "archive_person",
            "select_person",
        ]),
    ))
    .expect("Tauri build configuration failed");
}
