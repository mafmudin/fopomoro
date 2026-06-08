<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import type { AuthStatus } from "../types";

  // Parent reloads tasks after sign-in/out — the local mirror changes when the
  // backend reconciles with the cloud.
  let { onAuthChanged }: { onAuthChanged?: () => void } = $props();

  // idle → email (enter address) → code (enter OTP) → idle (signed in)
  type Stage = "idle" | "email" | "code";
  let stage = $state<Stage>("idle");
  let email = $state("");
  let code = $state("");
  let busy = $state(false);
  let error = $state("");
  let status = $state<AuthStatus>({ signed_in: false, email: null });

  onMount(async () => {
    try {
      status = await api.authStatus();
    } catch (e) {
      console.warn("[fopomoro] authStatus failed:", e);
    }
  });

  async function sendCode() {
    error = "";
    if (!email.trim()) {
      error = "Enter your email";
      return;
    }
    busy = true;
    try {
      await api.authRequestOtp(email.trim());
      stage = "code";
    } catch (e) {
      error = friendly(e);
    }
    busy = false;
  }

  async function verify() {
    error = "";
    if (!code.trim()) {
      error = "Enter the 6-digit code";
      return;
    }
    busy = true;
    try {
      status = await api.authVerifyOtp(email.trim(), code.trim());
      reset();
      onAuthChanged?.();
    } catch (e) {
      error = friendly(e);
    }
    busy = false;
  }

  async function signOut() {
    busy = true;
    try {
      await api.authSignOut();
      status = { signed_in: false, email: null };
      onAuthChanged?.();
    } catch (e) {
      error = friendly(e);
    }
    busy = false;
  }

  function reset() {
    stage = "idle";
    email = "";
    code = "";
    error = "";
  }

  function friendly(e: unknown): string {
    const s = String(e);
    return s.length > 90 ? s.slice(0, 90) + "…" : s;
  }
</script>

<section class="sync">
  <div class="header">
    <span class="section-header">SYNC</span>
    {#if status.signed_in}
      <span class="badge on" title={status.email ?? ""}>● synced</span>
    {:else}
      <span class="badge">local only</span>
    {/if}
  </div>

  {#if status.signed_in}
    <div class="row">
      <span class="email" title={status.email ?? ""}>{status.email}</span>
      <button class="link" disabled={busy} onclick={signOut}>Sign out</button>
    </div>
  {:else if stage === "idle"}
    <button class="wide" disabled={busy} onclick={() => (stage = "email")}>
      Sign in to sync
    </button>
  {:else if stage === "email"}
    <div class="row">
      <input
        type="email"
        placeholder="you@email.com"
        value={email}
        oninput={(e) => (email = (e.target as HTMLInputElement).value)}
        onkeydown={(e) => e.key === "Enter" && sendCode()}
      />
      <button class="go" disabled={busy} onclick={sendCode}>
        {busy ? "…" : "Send"}
      </button>
    </div>
    <button class="link sm" onclick={reset}>Cancel</button>
  {:else if stage === "code"}
    <div class="hint">Code sent to {email}</div>
    <div class="row">
      <input
        type="text"
        inputmode="numeric"
        placeholder="6-digit code"
        value={code}
        oninput={(e) => (code = (e.target as HTMLInputElement).value)}
        onkeydown={(e) => e.key === "Enter" && verify()}
      />
      <button class="go" disabled={busy} onclick={verify}>
        {busy ? "…" : "Verify"}
      </button>
    </div>
    <button class="link sm" onclick={reset}>Cancel</button>
  {/if}

  {#if error}
    <div class="error">{error}</div>
  {/if}
</section>

<style>
  .sync { margin-top: 8px; }
  .header { display: flex; align-items: center; justify-content: space-between; }
  .badge { font-size: 10px; color: var(--subtext); }
  .badge.on { color: var(--green, #a6e3a1); }
  .row { display: flex; align-items: center; gap: 6px; margin-top: 6px; }
  .row input { flex: 1; }
  .email { flex: 1; font-size: 12px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .wide { width: 100%; margin-top: 6px; font-size: 12px; }
  .go { width: 56px; padding: 0; }
  .hint { font-size: 10px; color: var(--subtext); margin-top: 6px; }
  .link { background: none; border: none; color: var(--accent); font-size: 11px; padding: 2px 4px; cursor: pointer; }
  .link.sm { margin-top: 4px; }
  .error { font-size: 10px; color: var(--red, #f38ba8); margin-top: 6px; }
</style>
