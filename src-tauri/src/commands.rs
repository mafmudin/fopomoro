use crate::models::{FoSession, FoTask, PomodoroConfig, WindowSettings};
use crate::storage;
use crate::supabase;
use crate::time_utils::{now_rfc3339, today_string};
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
pub fn load_progress(state: State<'_, AppState>) -> Result<FoSession, String> {
    let mut session: FoSession = storage::read_json(&state.data_dir, "session.json");
    let today = today_string();
    if session.date != today {
        session = FoSession { date: today, ..Default::default() };
        storage::write_json(&state.data_dir, "session.json", &session)?;
    }
    Ok(session)
}

#[tauri::command]
pub fn save_progress(session: FoSession, state: State<'_, AppState>) -> Result<(), String> {
    storage::write_json(&state.data_dir, "session.json", &session)
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
pub async fn get_tasks(state: State<'_, AppState>) -> Result<Vec<FoTask>, String> {
    let dir = state.data_dir.clone();
    let cfg = state.supabase.clone();
    let http = state.http.clone();
    if let Some(cfg) = cfg {
        match supabase::get_tasks(&http, &cfg).await {
            Ok(tasks) => {
                let _ = storage::write_json(&dir, "tasks.json", &tasks);
                return Ok(tasks);
            }
            Err(e) => eprintln!("[supabase] get_tasks failed, falling back to local: {e}"),
        }
    }
    Ok(storage::read_json(&dir, "tasks.json"))
}

#[tauri::command]
pub async fn insert_task(title: String, state: State<'_, AppState>) -> Result<FoTask, String> {
    let dir = state.data_dir.clone();
    let cfg = state.supabase.clone();
    let http = state.http.clone();

    let mut tasks: Vec<FoTask> = storage::read_json(&dir, "tasks.json");
    let n = next_task_number(&tasks);
    let mut task = FoTask {
        id: Uuid::new_v4().to_string(),
        task_id: format!("FO-{:02}", n),
        title: title.trim().to_string(),
        is_completed: false,
        created_at: now_rfc3339(),
        completed_at: None,
        pomodoro_count: 0,
    };
    if let Some(cfg) = cfg {
        match supabase::insert_task(&http, &cfg, &task).await {
            Ok(saved) => task = saved, // adopt DB id + task_number
            Err(e) => eprintln!("[supabase] insert_task failed: {e}"),
        }
    }
    // NOTE (v1 limitation): if Supabase is configured but this insert failed,
    // the task keeps its local UUID. A later get_tasks will overwrite tasks.json
    // with server state, dropping this task. Robust offline-first needs a sync
    // queue (out of scope for v1).
    tasks.push(task.clone());
    storage::write_json(&dir, "tasks.json", &tasks)?;
    Ok(task)
}

#[tauri::command]
pub async fn update_task(task: FoTask, state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.data_dir.clone();
    let cfg = state.supabase.clone();
    let http = state.http.clone();

    if let Some(cfg) = cfg {
        if let Err(e) = supabase::update_task(&http, &cfg, &task).await {
            eprintln!("[supabase] update_task failed: {e}");
        }
    }
    let mut tasks: Vec<FoTask> = storage::read_json(&dir, "tasks.json");
    if let Some(slot) = tasks.iter_mut().find(|t| t.id == task.id) {
        *slot = task;
    }
    storage::write_json(&dir, "tasks.json", &tasks)
}

#[tauri::command]
pub async fn delete_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.data_dir.clone();
    let cfg = state.supabase.clone();
    let http = state.http.clone();

    if let Some(cfg) = cfg {
        if let Err(e) = supabase::delete_task(&http, &cfg, &id).await {
            eprintln!("[supabase] delete_task failed: {e}");
        }
    }
    let mut tasks: Vec<FoTask> = storage::read_json(&dir, "tasks.json");
    tasks.retain(|t| t.id != id);
    storage::write_json(&dir, "tasks.json", &tasks)
}

#[tauri::command]
pub async fn record_session(
    task_id: Option<String>,
    duration_minutes: i32,
    was_focused: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cfg = state.supabase.clone();
    let http = state.http.clone();
    if let Some(cfg) = cfg {
        if let Err(e) = supabase::insert_session(&http, &cfg, task_id.as_deref(), duration_minutes, was_focused).await {
            eprintln!("[supabase] record_session failed: {e}");
        }
    }
    Ok(())
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
