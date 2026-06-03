use crate::models::{PomodoroConfig, WindowSettings};
use crate::storage;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> PomodoroConfig {
    storage::read_json(&state.data_dir, "pomodoro_config.json")
}

#[tauri::command]
pub fn save_config(config: PomodoroConfig, state: State<'_, AppState>) -> Result<(), String> {
    storage::write_json(&state.data_dir, "pomodoro_config.json", &config)
}

#[tauri::command]
pub fn load_settings(state: State<'_, AppState>) -> WindowSettings {
    storage::read_json(&state.data_dir, "settings.json")
}

#[tauri::command]
pub fn save_settings(settings: WindowSettings, state: State<'_, AppState>) -> Result<(), String> {
    storage::write_json(&state.data_dir, "settings.json", &settings)
}

#[tauri::command]
pub fn set_click_through(enabled: bool, window: tauri::WebviewWindow) -> Result<(), String> {
    window.set_ignore_cursor_events(enabled).map_err(|e| e.to_string())
}
