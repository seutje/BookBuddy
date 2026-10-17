pub mod commands;
pub mod domain;
pub mod persistence;
pub mod services;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = if cfg!(debug_assertions) {
                app.path().app_local_data_dir()?.join("development")
            } else {
                app.path().app_local_data_dir()?
            };
            let pool = tauri::async_runtime::block_on(persistence::open(&directory))?;
            app.manage(pool);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("BookBuddy could not start; existing data has been preserved");
}
