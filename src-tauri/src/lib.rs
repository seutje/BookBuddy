pub mod commands;
pub mod domain;
pub mod persistence;
pub mod services;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let test_run = if cfg!(debug_assertions) {
                std::env::var("BOOKBUDDY_TEST_RUN").ok()
            } else {
                None
            };
            let directory = app.path().app_local_data_dir().ok().and_then(|path| {
                services::data_directory(path, cfg!(debug_assertions), test_run.as_deref()).ok()
            });
            let pool = directory
                .as_ref()
                .and_then(|path| tauri::async_runtime::block_on(persistence::open(path)).ok());
            app.manage(commands::AppState {
                pool,
                data_directory: directory.map(|path| path.to_string_lossy().into_owned()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::startup_status,
            commands::list_people,
            commands::create_person,
            commands::rename_person,
            commands::archive_person,
            commands::select_person,
            commands::books::list_books,
            commands::books::get_book,
            commands::books::create_book,
            commands::books::update_book,
            commands::books::ensure_person_book,
            commands::books::update_person_book,
            commands::books::mark_read,
            commands::books::duplicate_book_hints
        ])
        .build(tauri::generate_context!())
        .expect("BookBuddy could not start; existing data has been preserved")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(pool) = &app.state::<commands::AppState>().pool {
                    tauri::async_runtime::block_on(pool.close());
                }
            }
        });
}
