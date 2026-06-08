//! Reconcile local ↔ cloud task lists right after sign-in (Tahap 3).
//!
//! Smart-merge policy (chosen to avoid duplicates on re-login / multi-device):
//!   - cloud has tasks            → adopt cloud (server is canonical)
//!   - cloud empty + local tasks  → push local up once, then adopt the result
//!   - both empty                 → no-op
//!
//! So a user who worked locally and *then* signs in keeps their tasks, while a
//! second sign-in (cloud already populated) never re-uploads and never dupes.
//!
//! Cross-account safety: the local mirror is tagged with the `user_id` that owns
//! it (`sync_owner.json`). The "cloud empty → push local" branch ONLY fires when
//! the mirror belongs to this same user or to the never-signed-in/anonymous state
//! (no owner). Without this, signing out of account A (which keeps A's tasks in
//! the local mirror by design) and then into a fresh account B would push A's
//! tasks into B — they'd "replicate" with new ids. The owner tag blocks that.

use crate::models::FoTask;
use crate::storage;
use crate::supabase::{self, SupabaseConfig};
use std::path::Path;

const OWNER_FILE: &str = "sync_owner.json";

/// `user_id` of the account that currently owns `tasks.json` ("" if never signed
/// in / anonymous local tasks).
fn mirror_owner(data_dir: &Path) -> String {
    storage::read_json::<String>(data_dir, OWNER_FILE)
}

fn set_mirror_owner(data_dir: &Path, user_id: &str) {
    let _ = storage::write_json(data_dir, OWNER_FILE, &user_id.to_string());
}

pub async fn reconcile_on_login(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    token: &str,
    user_id: &str,
    data_dir: &Path,
) -> Result<(), String> {
    let server = supabase::get_tasks(http, cfg, token).await?;

    // Cloud already has data → it wins; just refresh the local mirror.
    if !server.is_empty() {
        let _ = storage::write_json(data_dir, "tasks.json", &server);
        set_mirror_owner(data_dir, user_id);
        return Ok(());
    }

    // Cloud empty. Only push the local mirror up if it actually belongs to THIS
    // user (or to the anonymous, never-signed-in state). If it belongs to another
    // account (e.g. signed out of A, now signing into fresh B), discard it so A's
    // tasks don't leak into B.
    let local: Vec<FoTask> = storage::read_json(data_dir, "tasks.json");
    let owner = mirror_owner(data_dir);
    let belongs_to_other = !owner.is_empty() && owner != user_id;

    if local.is_empty() || belongs_to_other {
        if belongs_to_other {
            let _ = storage::write_json(data_dir, "tasks.json", &Vec::<FoTask>::new());
        }
        set_mirror_owner(data_dir, user_id);
        return Ok(());
    }

    // First-ever sign-in for this account with genuine local tasks. Upload them
    // (server assigns id + task_number per-user), then adopt the canonical state.
    for task in &local {
        if let Err(e) = supabase::insert_task(http, cfg, token, task).await {
            eprintln!("[sync] failed to upload local task '{}': {e}", task.title);
        }
    }
    let merged = supabase::get_tasks(http, cfg, token).await.unwrap_or(local);
    let _ = storage::write_json(data_dir, "tasks.json", &merged);
    set_mirror_owner(data_dir, user_id);
    Ok(())
}
