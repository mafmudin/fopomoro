# FoPoMoro Mac — Design Spec

**Date:** 2026-06-03
**Status:** Approved (design), pending spec review
**Source app:** FoPoMoro (Windows, WPF/.NET Framework 4.8.1) at `../fo-po-moro`

## Overview

A macOS port of FoPoMoro — a semi-transparent, always-on-top floating overlay
for productivity and study tracking. Full feature parity with the Windows
version: clock, Pomodoro timer, task list, progress statistics, and Supabase
cloud sync. Built with **Tauri v2** (Rust core) and a **Svelte** (Vite +
TypeScript) frontend, living in its own Git repository.

## Goals

- Full feature parity with the Windows WPF app.
- Native macOS `.app` bundle (Tauri), small footprint.
- Share the **same Supabase backend and schema** (`tasks`, `pomodoro_sessions`)
  so data syncs across Windows and Mac.
- Preserve the MVVM separation of concerns from the original, mapped onto the
  Tauri/Svelte stack.

## Non-Goals (v1)

- Code distribution / notarization / code signing (dev-local run only).
- A shared/abstracted codebase between Windows and Mac (separate repos, parity
  kept in sync manually).
- Charting libraries for progress (plain text stats, same as Windows).

## Tech Stack

- **Shell:** Tauri v2 (produces a native macOS `.app` bundle; UI rendered in the
  OS WKWebView, not Electron).
- **Core:** Rust — network (`reqwest`), file I/O (`serde_json` + `std::fs`).
- **Frontend:** Svelte + Vite + TypeScript (scaffolded via `create-tauri-app`).
- **Plugins:** `tauri-plugin-window-state` (position memory),
  `tauri-plugin-notification` (toast).
- **Backend:** existing Supabase project (REST), tables `tasks` and
  `pomodoro_sessions`.

## Architecture: WPF/MVVM → Tauri/Svelte mapping

The original MVVM structure is preserved; only the layer technology changes.

| Windows (WPF/C#) | Mac (Tauri/Svelte) | Role |
|---|---|---|
| `Models/*` (FoTask, FoSession, PomodoroConfig, WindowSettings) | Rust `struct` + `serde` **and** TS `type` | Data models (source of truth in Rust, mirrored in TS) |
| `DataService` (local JSON) | Rust module `storage` (serde_json + fs) | Read/write JSON in app data dir |
| `SupabaseService` (HttpClient REST) | Rust module `supabase` (`reqwest`) | REST to Supabase; anon key stays in Rust |
| `NotificationService` (toast + SoundPlayer) | `tauri-plugin-notification` + Web Audio (`<audio>`) | Session-end notification + sound |
| `ViewModels/*` (Timer/Task/Main) | Svelte stores (`writable`/`derived`) | Reactive state = "ViewModel" |
| `Views/*Panel.xaml` | Svelte components (Clock/Pomodoro/TaskList/Progress) | Stateless views bound to stores |
| `RelayCommand` | Svelte event handlers | (no longer needed) |
| `MainWindow.xaml` + `NativeMethods` | `tauri.conf.json` window config + overlay commands | Window chrome & overlay behavior |

**Principle:** all network and file I/O lives in **Rust**, exposed as Tauri
commands (`#[tauri::command]`). The Svelte frontend stays thin — view + state
only. This avoids CORS, keeps the Supabase anon key out of the webview, and
mirrors the original Service/ViewModel/View separation.

## Repository Structure

```
fopomoro-mac/
├── src/                       # Svelte frontend
│   ├── lib/
│   │   ├── components/        # Clock.svelte, Pomodoro.svelte, TaskList.svelte, Progress.svelte
│   │   ├── stores/            # timer.ts, tasks.ts, progress.ts, settings.ts  (= ViewModels)
│   │   ├── api.ts             # invoke() wrappers to Rust commands
│   │   └── types.ts           # mirrored model types
│   ├── App.svelte             # = MainWindow (overlay panel, expand/collapse, opacity)
│   └── main.ts
├── src-tauri/
│   ├── src/
│   │   ├── main.rs / lib.rs   # register commands + plugins
│   │   ├── models.rs          # FoTask, FoSession, PomodoroConfig, WindowSettings
│   │   ├── storage.rs         # = DataService
│   │   ├── supabase.rs        # = SupabaseService
│   │   └── commands.rs        # #[tauri::command] surface
│   ├── tauri.conf.json        # window: transparent, decorations:false, alwaysOnTop
│   ├── Cargo.toml
│   └── icons/
├── .env.example               # SUPABASE_URL, SUPABASE_ANON_KEY (same as Windows)
├── package.json
└── README.md
```

## Data Models

Mirror the Windows models. Rust structs (serde, snake_case JSON to match
Supabase columns) with TS equivalents.

- **FoTask** — `id` (UUID), `task_id` (display, e.g. `FO-01`), `title`,
  `is_completed`, `created_at`, `completed_at?`, `pomodoro_count`. Transient
  UI-only fields (`is_active`, display helpers) live in the Svelte store.
- **FoSession** — `date`, `focus_sessions_count`, `total_minutes_studied`,
  `tasks_completed_count`.
- **PomodoroConfig** — `focus_minutes` (25), `short_break_minutes` (5),
  `long_break_minutes` (15).
- **WindowSettings** — opacity, click-through, expand/collapse state, etc.
  (position/size handled by `tauri-plugin-window-state`).

Supabase table columns (unchanged from Windows): `tasks(id, task_number, title,
is_completed, created_at, completed_at, pomodoro_count)`,
`pomodoro_sessions(task_id, duration_minutes, was_focused)`.

## Rust Command Surface (frontend ↔ core contract)

```
get_tasks() -> Vec<FoTask>
insert_task(title: String) -> FoTask
update_task(task: FoTask) -> ()
delete_task(id: Uuid) -> ()
record_session(task_id: Option<Uuid>, duration_minutes: i32, was_focused: bool) -> ()
load_progress() -> FoSession           // with daily-reset check (stored date vs today)
load_settings() -> WindowSettings
save_settings(settings: WindowSettings) -> ()
set_click_through(enabled: bool) -> ()  // window.set_ignore_cursor_events
```

**Sync strategy** (mirrors Windows): if `.env` is present and complete →
Supabase is the source of truth for tasks (flag equivalent to `IsAvailable`);
otherwise fall back to **offline** local JSON. Progress and settings are always
local.

## Overlay Behavior Mapping

| Feature | Tauri implementation |
|---|---|
| Always-on-top | `tauri.conf.json` `alwaysOnTop: true` + `set_visible_on_all_workspaces` (show on all Spaces) |
| Transparent | `transparent: true`, `decorations: false`, body CSS `background: transparent` |
| Draggable | `data-tauri-drag-region` on the panel header |
| Click-through | command `set_click_through` → `window.set_ignore_cursor_events(true)` |
| Opacity slider | CSS `opacity` on the panel container (0.3–1.0) — most portable |
| Position memory | `tauri-plugin-window-state` (auto save/restore position + size) |
| Expand/collapse | CSS height transition in Svelte |

## Pomodoro & Clock

Countdown and clock run in the **frontend** (`setInterval` inside a store) —
sufficient for an overlay and avoids IPC churn. State machine
`Idle → Focus → ShortBreak → LongBreak`; after 4 focus sessions → long break
(same as the C# version). On session end: play `timer-end.wav` (Web Audio) +
fire a notification + call `record_session` (inserts to Supabase + updates local
progress).

## Build & Run

Prerequisites: Rust toolchain, Node, Xcode Command Line Tools.
- Scaffold via `create-tauri-app` (Svelte-TS template).
- Dev: `npm run tauri dev`.
- Build `.app`/`.dmg`: `npm run tauri build`.

## Incremental Build Plan (each phase independently runnable)

1. Scaffold Tauri + Svelte; overlay window (transparent / draggable / topmost) — skeleton.
2. Clock panel.
3. Pomodoro timer + sound + notification.
4. Task list + local storage (Rust storage commands).
5. Supabase sync (Rust supabase module + offline fallback).
6. Progress stats + daily reset.
7. Polish: opacity slider, click-through toggle, expand/collapse, position memory.

## Testing

- **Rust:** unit tests for `storage` (JSON round-trip), `.env` parsing, and
  `TaskRecord → FoTask` mapping.
- **Frontend:** store logic (Pomodoro state machine, daily-reset) via Vitest.
- **Overlay behavior:** manual verification per phase (`tauri dev`).

## Risks / macOS Nuances

- **Floating above other fullscreen apps** may need a specific NSWindow level;
  if standard `alwaysOnTop` is insufficient, an objc tweak may be required
  (follow-up, not a v1 blocker).
- **Notifications** require a first-run permission prompt (standard macOS).
- **Bundle identifier & signing** are out of scope for v1 (dev-local run);
  notarization needed only for later distribution.
