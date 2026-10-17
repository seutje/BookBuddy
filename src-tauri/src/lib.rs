pub mod commands;
pub mod domain;
pub mod persistence;
pub mod services;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
                workbooks: std::sync::Mutex::new(std::collections::HashMap::new()),
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
            commands::books::duplicate_book_hints,
            commands::custom_fields::list_custom_fields,
            commands::custom_fields::create_custom_field,
            commands::custom_fields::preview_custom_field_change,
            commands::custom_fields::update_custom_field,
            commands::custom_fields::reorder_custom_fields,
            commands::custom_fields::get_custom_values,
            commands::custom_fields::set_custom_value,
            commands::readings::local_today_date,
            commands::readings::list_readings,
            commands::readings::log_reading,
            commands::readings::log_reading_today,
            commands::readings::edit_reading,
            commands::readings::delete_reading,
            commands::import::choose_library_xlsx,
            commands::import::preview_library_xlsx_mapping
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
