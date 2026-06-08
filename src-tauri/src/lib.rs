mod auth;
mod models;
mod storage;
mod supabase;
mod sync;
mod commands;
mod time_utils;

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_window_state::StateFlags;

pub struct AppState {
    pub data_dir: PathBuf,
    pub supabase: Option<supabase::SupabaseConfig>,
    pub http: reqwest::Client,
    // Some(..) once the user signs in via Email OTP. Cloud sync is gated on this:
    // None ⇒ purely local. Persisted to disk and reloaded on startup.
    pub auth: Mutex<Option<auth::Session>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the project root / cwd (dev-local). No-op if absent.
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        // Only remember WHERE the overlay was dragged; size is content-driven
        // (the frontend ResizeObserver is the authoritative sizer).
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::POSITION)
                .build(),
        )
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

            // Restore a prior sign-in (if any). The access token may be stale; it
            // is refreshed lazily before the first cloud call (Tahap 2).
            let session = auth::load_session(&data_dir);

            app.manage(AppState {
                data_dir,
                supabase,
                http,
                auth: Mutex::new(session),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::load_settings,
            commands::save_settings,
            commands::get_tasks,
            commands::insert_task,
            commands::update_task,
            commands::delete_task,
            commands::record_session,
            commands::load_progress,
            commands::save_progress,
            auth::auth_request_otp,
            auth::auth_verify_otp,
            auth::auth_status,
            auth::auth_sign_out,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
