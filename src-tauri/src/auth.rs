//! Email-OTP authentication against Supabase GoTrue (`/auth/v1/*`).
//!
//! Flow: `auth_request_otp(email)` mails a 6-digit code → `auth_verify_otp(email,
//! code)` exchanges it for a session (JWT). The session is held in `AppState` and
//! persisted so the user stays signed in across restarts.
//!
//! Only the auth endpoints use the anon `apikey`. Once signed in, data calls use
//! the user's `access_token` (see commands gating, Tahap 2), which is what RLS
//! keys off — the anon key alone can no longer touch any user's rows.

use crate::supabase::SupabaseConfig;
use crate::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::State;

const SESSION_FILE: &str = "auth_session.json";

/// Persisted + in-memory auth session.
///
/// NOTE (v1): stored as plaintext JSON in the app data dir. The refresh token is
/// long-lived and sensitive — a hardened build should keep it in the OS keychain
/// (e.g. tauri-plugin-stronghold). Tracked in docs/multiuser-auth-plan.md (Tahap 1).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64, // unix seconds
    pub user_id: String,
    pub email: String,
}

impl Session {
    /// True once the access token is within `skew` seconds of expiry.
    pub fn is_expired(&self, skew: i64) -> bool {
        Utc::now().timestamp() + skew >= self.expires_at
    }
}

/// What the frontend needs to render the auth UI.
#[derive(Serialize)]
pub struct AuthStatus {
    pub signed_in: bool,
    pub email: Option<String>,
}

// ── Persistence ────────────────────────────────────────────────────────────

pub fn load_session(dir: &Path) -> Option<Session> {
    let text = fs::read_to_string(dir.join(SESSION_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_session(dir: &Path, session: &Session) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(dir.join(SESSION_FILE), text).map_err(|e| e.to_string())
}

fn clear_session_file(dir: &Path) {
    let _ = fs::remove_file(dir.join(SESSION_FILE));
}

// ── GoTrue wire types ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    user: GoTrueUser,
}

#[derive(Deserialize)]
struct GoTrueUser {
    id: String,
    email: Option<String>,
}

impl TokenResponse {
    fn into_session(self, fallback_email: &str) -> Session {
        Session {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: Utc::now().timestamp() + self.expires_in,
            user_id: self.user.id,
            email: self.user.email.unwrap_or_else(|| fallback_email.to_string()),
        }
    }
}

/// Surface a useful message from a non-2xx GoTrue response body.
async fn check(resp: reqwest::Response) -> Result<reqwest::Response, String> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(format!("auth request failed ({status}): {body}"))
}

// ── GoTrue calls ─────────────────────────────────────────────────────────────

/// Mail a 6-digit OTP, creating the user if they don't exist yet.
pub async fn request_otp(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    email: &str,
) -> Result<(), String> {
    let resp = http
        .post(format!("{}/otp", cfg.auth_url))
        .header("apikey", &cfg.key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "email": email, "create_user": true }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check(resp).await?;
    Ok(())
}

/// Exchange an emailed OTP for a session.
pub async fn verify_otp(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    email: &str,
    code: &str,
) -> Result<Session, String> {
    let resp = http
        .post(format!("{}/verify", cfg.auth_url))
        .header("apikey", &cfg.key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "type": "email", "email": email, "token": code }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let resp = check(resp).await?;
    let token: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(token.into_session(email))
}

/// Trade a refresh token for a fresh access token.
pub async fn refresh(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    refresh_token: &str,
) -> Result<Session, String> {
    let resp = http
        .post(format!("{}/token?grant_type=refresh_token", cfg.auth_url))
        .header("apikey", &cfg.key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let resp = check(resp).await?;
    let token: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(token.into_session(""))
}

// ── Cloud gating ─────────────────────────────────────────────────────────────

/// The single gate for cloud access. Returns `(config, access_token)` only when
/// the user is signed in AND cloud is configured; `None` otherwise (→ caller
/// stays purely local). Refreshes the access token in place if it's near expiry.
pub async fn active_session(state: &AppState) -> Option<(SupabaseConfig, String)> {
    let cfg = state.supabase.clone()?;
    let current = state.auth.lock().unwrap().clone()?;

    if !current.is_expired(60) {
        return Some((cfg, current.access_token));
    }

    // Access token stale → swap the refresh token for a new session.
    match refresh(&state.http, &cfg, &current.refresh_token).await {
        Ok(mut next) => {
            // The refresh response may omit user fields; carry them over.
            if next.user_id.is_empty() {
                next.user_id = current.user_id.clone();
            }
            if next.email.is_empty() {
                next.email = current.email.clone();
            }
            let token = next.access_token.clone();
            let _ = save_session(&state.data_dir, &next);
            *state.auth.lock().unwrap() = Some(next);
            Some((cfg, token))
        }
        Err(e) => {
            // Refresh failed (offline / revoked): fall back to local-only for now,
            // keeping the stored session so a later call can retry.
            eprintln!("[auth] token refresh failed, using local data: {e}");
            None
        }
    }
}

// ── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn auth_request_otp(email: String, state: State<'_, AppState>) -> Result<(), String> {
    let cfg = state.supabase.clone().ok_or("cloud sync not configured")?;
    request_otp(&state.http, &cfg, email.trim()).await
}

#[tauri::command]
pub async fn auth_verify_otp(
    email: String,
    code: String,
    state: State<'_, AppState>,
) -> Result<AuthStatus, String> {
    let cfg = state.supabase.clone().ok_or("cloud sync not configured")?;
    let session = verify_otp(&state.http, &cfg, email.trim(), code.trim()).await?;
    save_session(&state.data_dir, &session)?;
    let token = session.access_token.clone();
    let email = session.email.clone();
    *state.auth.lock().unwrap() = Some(session);

    // Smart-merge local ↔ cloud now that we're signed in (Tahap 3). Non-fatal:
    // a failure here still leaves the user signed in.
    if let Err(e) =
        crate::sync::reconcile_on_login(&state.http, &cfg, &token, &state.data_dir).await
    {
        eprintln!("[sync] reconcile after login failed: {e}");
    }

    Ok(AuthStatus { signed_in: true, email: Some(email) })
}

#[tauri::command]
pub fn auth_status(state: State<'_, AppState>) -> AuthStatus {
    match &*state.auth.lock().unwrap() {
        Some(s) => AuthStatus { signed_in: true, email: Some(s.email.clone()) },
        None => AuthStatus { signed_in: false, email: None },
    }
}

#[tauri::command]
pub async fn auth_sign_out(state: State<'_, AppState>) -> Result<(), String> {
    // Drop the session locally regardless of whether the server logout succeeds;
    // the local copy of tasks is preserved (we revert to local-only mode).
    let token = state.auth.lock().unwrap().take();
    clear_session_file(&state.data_dir);

    if let (Some(session), Some(cfg)) = (token, state.supabase.clone()) {
        let _ = state
            .http
            .post(format!("{}/logout", cfg.auth_url))
            .header("apikey", &cfg.key)
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()
            .await; // best-effort
    }
    Ok(())
}
