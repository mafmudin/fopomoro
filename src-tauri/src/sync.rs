//! Reconcile local ↔ cloud task lists right after sign-in (Tahap 3).
//!
//! Smart-merge policy (chosen to avoid duplicates on re-login / multi-device):
//!   - cloud has tasks            → adopt cloud (server is canonical)
//!   - cloud empty + local tasks  → push local up once, then adopt the result
//!   - both empty                 → no-op
//!
//! So a user who worked locally and *then* signs in keeps their tasks, while a
//! second sign-in (cloud already populated) never re-uploads and never dupes.

use crate::models::FoTask;
use crate::storage;
use crate::supabase::{self, SupabaseConfig};
use std::path::Path;

pub async fn reconcile_on_login(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    token: &str,
    data_dir: &Path,
) -> Result<(), String> {
    let server = supabase::get_tasks(http, cfg, token).await?;

    // Cloud already has data → it wins; just refresh the local mirror.
    if !server.is_empty() {
        let _ = storage::write_json(data_dir, "tasks.json", &server);
        return Ok(());
    }

    // Cloud empty: first-ever sign-in for this account. Upload local tasks (if
    // any) so they're not lost. The server assigns id + task_number per-user.
    let local: Vec<FoTask> = storage::read_json(data_dir, "tasks.json");
    if local.is_empty() {
        return Ok(());
    }
    for task in &local {
        if let Err(e) = supabase::insert_task(http, cfg, token, task).await {
            eprintln!("[sync] failed to upload local task '{}': {e}", task.title);
        }
    }

    // Adopt the canonical server state (with freshly assigned ids/numbers).
    let merged = supabase::get_tasks(http, cfg, token).await.unwrap_or(local);
    let _ = storage::write_json(data_dir, "tasks.json", &merged);
    Ok(())
}
