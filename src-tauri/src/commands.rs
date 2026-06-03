use crate::models::{FoTask, PomodoroConfig, WindowSettings};
use crate::storage;
use crate::time_utils::now_rfc3339;
use crate::AppState;
use tauri::State;
use uuid::Uuid;

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

/// Next FO-NN number = max existing FO-NN + 1 (1 if none).
pub fn next_task_number(tasks: &[FoTask]) -> i32 {
    let mut max = 0;
    for t in tasks {
        if let Some(rest) = t.task_id.strip_prefix("FO-") {
            if let Ok(n) = rest.parse::<i32>() {
                if n > max { max = n; }
            }
        }
    }
    max + 1
}

#[tauri::command]
pub fn get_tasks(state: State<'_, AppState>) -> Vec<FoTask> {
    storage::read_json(&state.data_dir, "tasks.json")
}

#[tauri::command]
pub fn insert_task(title: String, state: State<'_, AppState>) -> Result<FoTask, String> {
    let mut tasks: Vec<FoTask> = storage::read_json(&state.data_dir, "tasks.json");
    let n = next_task_number(&tasks);
    let task = FoTask {
        id: Uuid::new_v4().to_string(),
        task_id: format!("FO-{:02}", n),
        title: title.trim().to_string(),
        is_completed: false,
        created_at: now_rfc3339(),
        completed_at: None,
        pomodoro_count: 0,
    };
    tasks.push(task.clone());
    storage::write_json(&state.data_dir, "tasks.json", &tasks)?;
    Ok(task)
}

#[tauri::command]
pub fn update_task(task: FoTask, state: State<'_, AppState>) -> Result<(), String> {
    let mut tasks: Vec<FoTask> = storage::read_json(&state.data_dir, "tasks.json");
    if let Some(slot) = tasks.iter_mut().find(|t| t.id == task.id) {
        *slot = task;
    }
    storage::write_json(&state.data_dir, "tasks.json", &tasks)
}

#[tauri::command]
pub fn delete_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut tasks: Vec<FoTask> = storage::read_json(&state.data_dir, "tasks.json");
    tasks.retain(|t| t.id != id);
    storage::write_json(&state.data_dir, "tasks.json", &tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(task_id: &str) -> FoTask {
        FoTask {
            id: "x".into(), task_id: task_id.into(), title: "t".into(),
            is_completed: false, created_at: "".into(), completed_at: None, pomodoro_count: 0,
        }
    }

    #[test]
    fn next_number_empty_is_one() {
        assert_eq!(next_task_number(&[]), 1);
    }

    #[test]
    fn next_number_is_max_plus_one() {
        let tasks = vec![task("FO-01"), task("FO-07"), task("FO-03"), task("weird")];
        assert_eq!(next_task_number(&tasks), 8);
    }
}
