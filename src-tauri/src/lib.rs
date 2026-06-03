mod models;
mod storage;
mod supabase;
mod commands;
mod time_utils;

use std::path::PathBuf;
use tauri::Manager;

pub struct AppState {
    pub data_dir: PathBuf,
    pub supabase: Option<supabase::SupabaseConfig>,
    pub http: reqwest::Client,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the project root / cwd (dev-local). No-op if absent.
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            let supabase = supabase::SupabaseConfig::from_env();
            if supabase.is_none() {
                eprintln!("[supabase] .env not found or incomplete — running offline.");
            }

            let http = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client");

            app.manage(AppState {
                data_dir,
                supabase,
                http,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::load_settings,
            commands::save_settings,
            commands::set_click_through,
            commands::get_tasks,
            commands::insert_task,
            commands::update_task,
            commands::delete_task,
            commands::record_session,
            commands::load_progress,
            commands::save_progress,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
