use std::env;
use serde::Deserialize;
use crate::models::FoTask;

#[derive(Clone, Debug)]
pub struct SupabaseConfig {
    pub base_url: String, // {project}/rest/v1 — PostgREST data API
    pub auth_url: String, // {project}/auth/v1 — GoTrue auth API
    pub key: String,      // anon key (used as `apikey` header; NOT for data RLS)
}

impl SupabaseConfig {
    pub fn from_env() -> Option<Self> {
        // Runtime env (dev: loaded from .env via dotenvy) takes precedence; fall back
        // to values baked at compile time — the CI release build sets these from
        // GitHub secrets so the distributed bundle can sync without a local .env.
        let url = read_secret("SUPABASE_URL", option_env!("SUPABASE_URL"))?;
        let key = read_secret("SUPABASE_ANON_KEY", option_env!("SUPABASE_ANON_KEY"))?;
        let root = url.trim_end_matches('/');
        Some(Self {
            base_url: format!("{}/rest/v1", root),
            auth_url: format!("{}/auth/v1", root),
            key,
        })
    }
}

/// Prefer the runtime env var; fall back to a value baked in at compile time.
fn read_secret(var: &str, baked: Option<&str>) -> Option<String> {
    env::var(var)
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| baked.filter(|s| !s.is_empty()).map(str::to_string))
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

// Data calls authenticate as the signed-in USER (Bearer = their access token).
// RLS keys off this JWT — the anon `apikey` alone grants no row access. `token`
// is always a user access token here (these fns only run when signed in).
fn auth(req: reqwest::RequestBuilder, cfg: &SupabaseConfig, token: &str) -> reqwest::RequestBuilder {
    req.header("apikey", &cfg.key)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
}

pub async fn get_tasks(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    token: &str,
) -> Result<Vec<FoTask>, String> {
    let url = format!("{}/tasks?select=*&order=task_number.asc", cfg.base_url);
    let resp = auth(http.get(&url), cfg, token).send().await.map_err(|e| e.to_string())?;
    let resp = resp.error_for_status().map_err(|e| e.to_string())?;
    let records: Vec<TaskRecord> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(records.into_iter().map(FoTask::from).collect())
}

pub async fn insert_task(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    token: &str,
    task: &FoTask,
) -> Result<FoTask, String> {
    // Omit user_id (DB default = auth.uid()) and task_number (DB trigger assigns
    // it per-user) — the server is authoritative for both.
    let url = format!("{}/tasks", cfg.base_url);
    let body = serde_json::json!({
        "title": task.title,
        "is_completed": task.is_completed,
        "created_at": task.created_at,
        "completed_at": task.completed_at,
        "pomodoro_count": task.pomodoro_count,
    });
    let resp = auth(http.post(&url), cfg, token)
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
    token: &str,
    task: &FoTask,
) -> Result<(), String> {
    let url = format!("{}/tasks?id=eq.{}", cfg.base_url, task.id);
    let body = serde_json::json!({
        "title": task.title,
        "is_completed": task.is_completed,
        "completed_at": task.completed_at,
        "pomodoro_count": task.pomodoro_count,
    });
    let resp = auth(http.patch(&url), cfg, token).json(&body).send().await.map_err(|e| e.to_string())?;
    resp.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_task(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    token: &str,
    id: &str,
) -> Result<(), String> {
    let url = format!("{}/tasks?id=eq.{}", cfg.base_url, id);
    let resp = auth(http.delete(&url), cfg, token).send().await.map_err(|e| e.to_string())?;
    resp.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn insert_session(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    token: &str,
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
    let resp = auth(http.post(&url), cfg, token).json(&body).send().await.map_err(|e| e.to_string())?;
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
