# Windows Build Pipeline — Short Plan

**Goal:** Extend the existing macOS release pipeline to also build a **Windows** installer (`.msi`/`.nsis`) from the same codebase, uploading both to one GitHub Release per tag.

**Status:** Planned — execute next session. Verify Windows transparency BEFORE committing to this path.

---

## Context (already done)

- `.github/workflows/release.yml` exists: `workflow_dispatch` with a `bump` choice (patch/minor/major) → bumps version in `tauri.conf.json` + `package.json`, commits to `main`, then `tauri-action` builds macOS `.dmg` + creates tag `vX.Y.Z` + Release.
- Supabase creds baked at compile time via `option_env!` (`supabase.rs::read_secret`) from the **`SUPABASE`** GitHub Environment secrets. `build.rs` has `rerun-if-env-changed`.
- Window APIs are guarded: `setVisibleOnAllWorkspaces` is in `try/catch` (no-op on Windows); `macOSPrivateApi` is macOS-only (ignored on Windows). Drag uses `data-tauri-drag-region` + `core:window:allow-start-dragging` (cross-platform).
- `tauri.conf.json` `bundle.icon` already includes `icons/icon.ico` (Windows needs it ✓). `bundle.targets: "all"` → Windows produces NSIS + MSI.

## ⚠️ Gate: verify Windows transparency FIRST

The overlay relies on a transparent, rounded, always-on-top window. macOS (WKWebView) does this cleanly; **Windows (WebView2) transparency is finicky** — transparent/rounded areas may render black/opaque. This is partly why the original is native WPF.

**Decision gate (do at start of session):** build Windows once (Task 1 below), run the `.exe` on a Windows machine, and look at the overlay.
- **Looks good** → keep the matrix pipeline (Task 1).
- **Looks bad** → either (a) try `window-vibrancy` / Windows acrylic+rounded tweak, or (b) abandon Windows-via-Tauri and keep WPF as the Windows client. Don't sink time into the pipeline before this is settled.

---

## Task 1 — Refactor workflow into matrix (macOS + Windows)

**File:** `.github/workflows/release.yml`

Split into two jobs so the version is bumped **once**, then built on both OSes:

1. **`prepare`** (runs-on `ubuntu-latest`, `environment: SUPABASE` not needed here):
   - checkout, setup-node
   - compute new version from `bump` input, write `tauri.conf.json` + `package.json`, `git commit -am "chore: release vX.Y.Z"` + `git push`
   - `outputs.version` = new version
2. **`build`** (`needs: prepare`, `strategy.matrix.os: [macos-latest, windows-latest]`, `environment: SUPABASE`):
   - `checkout` with **`ref: main`** (to get the bumped commit, NOT the pre-bump triggering SHA)
   - setup-node, dtolnay/rust-toolchain@stable, swatinem/rust-cache (`workspaces: src-tauri`)
   - `npm ci`
   - `tauri-apps/tauri-action@v0` with `tagName: v__VERSION__`, env `GITHUB_TOKEN` + `SUPABASE_URL` + `SUPABASE_ANON_KEY`

**Gotchas:**
- `environment: SUPABASE` must be on the **build** job (both OSes) so secrets bake on each.
- The `build` job MUST checkout `main` (post-bump), or `__VERSION__`/tags will be wrong.
- Both matrix legs use the same `tagName` → first creates the Release, second uploads to it. Minor race possible; if it bites, switch to `prepare` creating a draft release + build jobs uploading via `releaseId`.
- Windows runner has WebView2 preinstalled — no extra setup.

**Verify:** run the workflow → one Release `vX.Y.Z` with **both** `.dmg` and `.msi`/`-setup.exe`. Download the Windows installer, install, run.

## Task 2 — (only if transparency is bad) Windows transparency tweak

- Try `tauri-plugin-window-state` is unrelated; for transparency add the `window-vibrancy` crate (Windows: `apply_acrylic`/`apply_blur`) in `lib.rs` setup, guarded `#[cfg(target_os = "windows")]`.
- Or set a Windows-specific opaque-ish background fallback in the panel CSS.
- Re-test. If still poor, stop and keep WPF for Windows (update README to say macOS-only Tauri).

## Out of scope (note in README, don't implement)

- Windows code signing (Authenticode) — unsigned → SmartScreen warning, same posture as unsigned macOS.
- Universal macOS binary (Intel+ARM) — separate optional follow-up.
- Consolidating/retiring the WPF Windows app — product decision, not this task.

---

## Quick checklist for tomorrow

- [ ] Build Windows once, eyeball transparency on a real Windows machine (the gate).
- [ ] If OK: refactor `release.yml` to `prepare` + `build` matrix (Task 1).
- [ ] Confirm `environment: SUPABASE` on build job; `checkout ref: main`.
- [ ] Run workflow; confirm one Release with `.dmg` + `.msi`/`.exe`.
- [ ] If transparency bad: Task 2 or keep WPF; update README.
