use crate::auth;
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

/// Current tasks from the source of truth: cloud when signed in (and mirrored
/// locally), otherwise the local mirror. Shared by `get_tasks` and export.
pub async fn load_current_tasks(state: &AppState) -> Vec<FoTask> {
    if let Some((cfg, token)) = auth::active_session(state).await {
        match supabase::get_tasks(&state.http, &cfg, &token).await {
            Ok(tasks) => {
                let _ = storage::write_json(&state.data_dir, "tasks.json", &tasks);
                return tasks;
            }
            Err(e) => eprintln!("[supabase] get_tasks failed, falling back to local: {e}"),
        }
    }
    storage::read_json(&state.data_dir, "tasks.json")
}

#[tauri::command]
pub async fn get_tasks(state: State<'_, AppState>) -> Result<Vec<FoTask>, String> {
    Ok(load_current_tasks(state.inner()).await)
}

/// Write the current task list to `path` as pretty JSON. Returns the count.
#[tauri::command]
pub async fn export_tasks_to(path: String, state: State<'_, AppState>) -> Result<usize, String> {
    let tasks = load_current_tasks(state.inner()).await;
    let json = serde_json::to_string_pretty(&tasks).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(tasks.len())
}

/// Read a JSON task file from `path` and APPEND its tasks as new ones (fresh id +
/// FO-NN; pushed to cloud when signed in). Non-destructive. Returns the merged list.
#[tauri::command]
pub async fn import_tasks_from(path: String, state: State<'_, AppState>) -> Result<Vec<FoTask>, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imported: Vec<FoTask> =
        serde_json::from_str(&text).map_err(|e| format!("invalid task file: {e}"))?;

    let dir = state.data_dir.clone();
    let mut tasks: Vec<FoTask> = storage::read_json(&dir, "tasks.json");
    let session = auth::active_session(state.inner()).await;

    for imp in &imported {
        let n = next_task_number(&tasks);
        let mut task = FoTask {
            id: Uuid::new_v4().to_string(),
            task_id: format!("FO-{:02}", n),
            title: imp.title.trim().to_string(),
            is_completed: imp.is_completed,
            created_at: if imp.created_at.is_empty() { now_rfc3339() } else { imp.created_at.clone() },
            completed_at: imp.completed_at.clone(),
            pomodoro_count: imp.pomodoro_count,
        };
        if let Some((cfg, token)) = &session {
            match supabase::insert_task(&state.http, cfg, token, &task).await {
                Ok(saved) => task = saved, // adopt DB id + task_number
                Err(e) => eprintln!("[import] insert failed for '{}': {e}", task.title),
            }
        }
        tasks.push(task);
    }
    storage::write_json(&dir, "tasks.json", &tasks)?;
    Ok(tasks)
}

#[tauri::command]
pub async fn insert_task(title: String, state: State<'_, AppState>) -> Result<FoTask, String> {
    let dir = state.data_dir.clone();

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
    if let Some((cfg, token)) = auth::active_session(state.inner()).await {
        match supabase::insert_task(&state.http, &cfg, &token, &task).await {
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

    if let Some((cfg, token)) = auth::active_session(state.inner()).await {
        if let Err(e) = supabase::update_task(&state.http, &cfg, &token, &task).await {
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

    if let Some((cfg, token)) = auth::active_session(state.inner()).await {
        if let Err(e) = supabase::delete_task(&state.http, &cfg, &token, &id).await {
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
    if let Some((cfg, token)) = auth::active_session(state.inner()).await {
        if let Err(e) = supabase::insert_session(&state.http, &cfg, &token, task_id.as_deref(), duration_minutes, was_focused).await {
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
