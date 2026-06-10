//! Reconcile local ↔ cloud task lists right after sign-in (Tahap 3).
//!
//! Merge policy:
//!   - local tasks that are genuinely anonymous (created while never signed in →
//!     no owner tag) are PUSHED up and merged into this account, even if the
//!     cloud already has data. This is the "I built tasks locally, then signed
//!     in" case — they must not vanish.
//!   - a mirror already tagged with an account is NOT pushed: it's either ours
//!     (normally cleared on sign-out, so empty) or another account's leftover
//!     (must not leak into this one). In both cases we just adopt the server.
//!   - after any push, the canonical server state is adopted as the new mirror.
//!
//! Cross-account safety hinges on the owner tag (`sync_owner.json`): only the
//! untagged/anonymous state is ever uploaded, so signing out of A and into a
//! fresh B can never replicate A's tasks into B.

use crate::models::FoTask;
use crate::storage;
use crate::supabase::{self, SupabaseConfig};
use std::path::Path;

const OWNER_FILE: &str = "sync_owner.json";

/// `user_id` of the account that owns `tasks.json` ("" = never signed in /
/// anonymous local tasks).
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
    let local: Vec<FoTask> = storage::read_json(data_dir, "tasks.json");
    let owner = mirror_owner(data_dir);

    // Push only genuinely anonymous local-first tasks (no owner). A mirror tagged
    // with an account is either already synced (ours) or someone else's leftover —
    // pushing it would duplicate or leak, so we skip and adopt the server instead.
    if owner.is_empty() && !local.is_empty() {
        for task in &local {
            if let Err(e) = supabase::insert_task(http, cfg, token, task).await {
                eprintln!("[sync] failed to upload local task '{}': {e}", task.title);
            }
        }
    }

    // Adopt the canonical server state (now includes anything just pushed). On a
    // re-fetch failure, fall back to the server snapshot we already have.
    let merged = supabase::get_tasks(http, cfg, token).await.unwrap_or(server);
    let _ = storage::write_json(data_dir, "tasks.json", &merged);
    set_mirror_owner(data_dir, user_id);
    Ok(())
}
