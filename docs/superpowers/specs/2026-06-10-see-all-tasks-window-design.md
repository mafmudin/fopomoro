# Design — "See all" Tasks Window

**Status:** approved (design phase)
**Date:** 2026-06-10
**Scope:** Compact task list (top 5 incomplete) in the main overlay + a separate
native window listing all tasks, kept in sync with the main window.

## Problem & goal

The main overlay panel currently renders *all* tasks in a 200px scroll area.
For an always-on-top overlay meant to stay small, that's noisy. Goal:

- Main panel shows only the **top 5 incomplete** tasks (focus view).
- A **"See all"** button opens a separate, resizable **management window** listing
  every task (incomplete + completed), with full add/toggle/delete.
- Both windows stay **consistent** in real time.

## Decisions (locked)

- **Separate native window** (not an in-app overlay) → independent sizing.
- **Normal management window:** `decorations: true`, `resizable: true`,
  `alwaysOnTop: false`, shows in taskbar, solid theme background, default 520×640.
- **Full cross-window sync** of data, active task, and timer-lock state.

## Architecture

### Window creation
- One bundle / one `index.html`. `main.ts` branches on the **window label**:
  - `getCurrentWebviewWindow().label === "all-tasks"` → `mount(AllTasks)`
  - otherwise → `mount(App)` (the main overlay, unchanged entry)
- "See all" handler (in the main panel):
  1. `WebviewWindow.getByLabel("all-tasks")` → if it exists, `.setFocus()` (no duplicate).
  2. else `new WebviewWindow("all-tasks", { url: "index.html", title: "FoPoMoro — All Tasks", width: 520, height: 640, resizable: true, decorations: true, alwaysOnTop: false })`.
- Custom app commands (`get_tasks`, `insert_task`, …) need no extra ACL in Tauri 2,
  so the new window calls `api.*` as-is.

### Cross-window sync (3 global Tauri events)
Emit/listen are added inside `createTasksStore()` so both windows get them for free
(each window mounts its own store instance; events bridge them).

| Event | Emitted when | Listener action |
|-------|--------------|-----------------|
| `tasks:changed` | after any successful add/toggle/delete/pomodoro mutation | `store.load()` (refetch from backend = source of truth) |
| `task:active-changed` | `setActive` in either window | both stores set `activeTaskId` to the payload id |
| `timer:running-changed` | main starts/stops the timer | other window applies the same lock (`setTimerRunning`) |

- Backstop: each window also calls `store.load()` on window **focus**.
- Self-emit is harmless (refetch is idempotent); no self-filtering needed.
- The timer lives only in the main window; it is the sole emitter of
  `timer:running-changed`. `activeTaskId` and task data can change from either side.
- Listeners are registered in each view's `onMount` and torn down in `onDestroy`.

## Components

### `TaskRow.svelte` (new — extracted)
The per-task row markup + styles, currently inline in `TaskList.svelte`, is extracted
so both the compact list and the All Tasks window render identical rows.
- Props: `task: FoTask`, `store: TasksStore`, `activeId: string | null`, `timerRunning: boolean`.
- Renders: active dot (→ `store.setActive`), FO badge, checkbox + title (→ `store.toggle`),
  `🍅×n` badge, delete button (→ `store.remove`).

### `TaskList.svelte` (compact view — modified)
- Shows up to **5** tasks where `is_completed === false`, ordered by FO-NN (current order).
- Renders rows via `TaskRow`.
- **"See all (N)"** button:
  - Visible only when `total > 5 || completedCount > 0`.
  - `N` = total task count.
  - Styled with existing tokens (`--accent`/`--subtext`, small-button pattern) — no new visual style.
  - onclick → open/focus the `all-tasks` window.
- Keeps the count display and "Add a task…" input.

### `AllTasks.svelte` (new — the window view)
- Header: "All Tasks" + `done / total` count.
- Two sections: **Incomplete** (FO-NN asc) then **Completed** (FO-NN asc), each via `TaskRow`.
- "Add a task…" input (same `store.add`).
- Uses the same theme; solid background; comfortable width for long titles.
- No search/filter in v1 (future enhancement).

### store (`tasks.ts` — modified)
- Add event emit after successful mutations; expose helpers to apply remote
  `active-changed` / `timer:running-changed`.
- Add derived selectors usable by views: incomplete-only list, completed list, total.

## Data flow

```
[main] toggle task → api.updateTask → emit tasks:changed
                                   ↘ (listener) all-tasks store.load()
[all-tasks] add task → api.insertTask → emit tasks:changed
                                     ↘ (listener) main store.load()
[main] start timer → emit timer:running-changed → all-tasks applies lock
[either] setActive → emit task:active-changed → both update activeTaskId
```

Backend (`get_tasks` etc.) remains the single source of truth (cloud when signed in,
local mirror otherwise). Sync = "refetch on signal", not direct state copying.

## Capabilities

- `main` window capability: add `core:webview:allow-create-webview-window` and event
  emit/listen permissions.
- New capability for the `all-tasks` window: `core:default`, `core:window:allow-close`,
  `core:window:allow-minimize`, and event emit/listen permissions.

## Error handling & edge cases

- "See all" when window already open → focus it, never spawn a duplicate.
- Closing All Tasks → main keeps running; its listeners are cleaned up in `onDestroy`.
- Opening All Tasks while signed out → works (local data, same as main).
- Mutation while offline/cloud-error → handled by existing `api` fallbacks; the
  `tasks:changed` refetch reflects whatever the backend returns.

## Testing

- **Unit (vitest):**
  - compact list keeps only the first 5 incomplete tasks;
  - "See all" visibility rule (`total > 5 || completedCount > 0`);
  - incomplete/completed grouping + ordering selectors.
- **Manual:**
  - toggle/add/delete in one window reflects in the other;
  - lock enforced in All Tasks while a focus session runs;
  - reopening "See all" focuses the existing window (no duplicate);
  - active-task dot consistent across both windows.

## Out of scope (future)

- Search/filter in All Tasks.
- Syncing `pomodoro_config` / `settings` across devices (separate effort).
