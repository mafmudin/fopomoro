use std::env;
use serde::Deserialize;
use crate::models::FoTask;

#[derive(Clone, Debug)]
pub struct SupabaseConfig {
    pub base_url: String,
    pub key: String,
}

impl SupabaseConfig {
    pub fn from_env() -> Option<Self> {
        let url = env::var("SUPABASE_URL").ok().filter(|s| !s.is_empty())?;
        let key = env::var("SUPABASE_ANON_KEY").ok().filter(|s| !s.is_empty())?;
        Some(Self {
            base_url: format!("{}/rest/v1", url.trim_end_matches('/')),
            key,
        })
    }
}

#[derive(Deserialize)]
struct TaskRecord {
    id: String,
    task_number: i32,
    title: String,
    is_completed: bool,
    created_at: String,
    completed_at: Option<String>,
    pomodoro_count: i32,
}

impl From<TaskRecord> for FoTask {
    fn from(r: TaskRecord) -> Self {
        FoTask {
            id: r.id,
            task_id: format!("FO-{:02}", r.task_number),
            title: r.title,
            is_completed: r.is_completed,
            created_at: r.created_at,
            completed_at: r.completed_at,
            pomodoro_count: r.pomodoro_count,
        }
    }
}

fn auth(req: reqwest::RequestBuilder, cfg: &SupabaseConfig) -> reqwest::RequestBuilder {
    req.header("apikey", &cfg.key)
        .header("Authorization", format!("Bearer {}", cfg.key))
        .header("Content-Type", "application/json")
}

pub async fn get_tasks(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
) -> Result<Vec<FoTask>, String> {
    let url = format!("{}/tasks?select=*&order=task_number.asc", cfg.base_url);
    let resp = auth(http.get(&url), cfg).send().await.map_err(|e| e.to_string())?;
    let resp = resp.error_for_status().map_err(|e| e.to_string())?;
    let records: Vec<TaskRecord> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(records.into_iter().map(FoTask::from).collect())
}

pub async fn insert_task(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    task: &FoTask,
) -> Result<FoTask, String> {
    let url = format!("{}/tasks", cfg.base_url);
    let body = serde_json::json!({
        "title": task.title,
        "is_completed": task.is_completed,
        "created_at": task.created_at,
        "completed_at": task.completed_at,
        "pomodoro_count": task.pomodoro_count,
    });
    let resp = auth(http.post(&url), cfg)
        .header("Prefer", "return=representation")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?;
    let resp = resp.error_for_status().map_err(|e| e.to_string())?;
    let records: Vec<TaskRecord> = resp.json().await.map_err(|e| e.to_string())?;
    records.into_iter().next().map(FoTask::from).ok_or_else(|| "empty insert response".into())
}

pub async fn update_task(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    task: &FoTask,
) -> Result<(), String> {
    let url = format!("{}/tasks?id=eq.{}", cfg.base_url, task.id);
    let body = serde_json::json!({
        "title": task.title,
        "is_completed": task.is_completed,
        "completed_at": task.completed_at,
        "pomodoro_count": task.pomodoro_count,
    });
    let resp = auth(http.patch(&url), cfg).json(&body).send().await.map_err(|e| e.to_string())?;
    resp.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_task(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    id: &str,
) -> Result<(), String> {
    let url = format!("{}/tasks?id=eq.{}", cfg.base_url, id);
    let resp = auth(http.delete(&url), cfg).send().await.map_err(|e| e.to_string())?;
    resp.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn insert_session(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    task_id: Option<&str>,
    duration_minutes: i32,
    was_focused: bool,
) -> Result<(), String> {
    let url = format!("{}/pomodoro_sessions", cfg.base_url);
    let body = serde_json::json!({
        "task_id": task_id,
        "duration_minutes": duration_minutes,
        "was_focused": was_focused,
    });
    let resp = auth(http.post(&url), cfg).json(&body).send().await.map_err(|e| e.to_string())?;
    resp.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_record_maps_to_fo_task_with_padded_id() {
        let r = TaskRecord {
            id: "uuid-1".into(), task_number: 3, title: "Read".into(),
            is_completed: true, created_at: "2026-06-03T00:00:00Z".into(),
            completed_at: Some("2026-06-03T01:00:00Z".into()), pomodoro_count: 4,
        };
        let t: FoTask = r.into();
        assert_eq!(t.task_id, "FO-03");
        assert_eq!(t.id, "uuid-1");
        assert_eq!(t.pomodoro_count, 4);
    }
}
