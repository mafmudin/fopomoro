# See-all Tasks Window — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show only the top 5 incomplete tasks in the main overlay, with a "See all" button that opens a separate, resizable window listing every task, kept in sync with the main window in real time.

**Architecture:** One Svelte bundle / one `index.html`; `main.ts` branches on the Tauri window label to mount either the main overlay (`App`) or the all-tasks view (`AllTasks`). The two windows are separate JS contexts kept consistent via three global Tauri events (`tasks:changed`, `task:active-changed`, `timer:running-changed`) plus a focus backstop. Pure list logic lives in a tested `taskFilters.ts`; Tauri-event plumbing is isolated in a tested `sync.ts`.

**Tech Stack:** Tauri 2, Svelte 5 (runes), TypeScript, vitest.

> **Environment note:** `npm`/`cargo` run on **Windows** (no cargo in WSL). All `npm test` / `npm run check` / `npm run tauri` commands in this plan are executed on Windows PowerShell (use the `!` prefix in-session, or the user runs them and reports output). Edits are made from the repo working tree.

**Spec:** `docs/superpowers/specs/2026-06-10-see-all-tasks-window-design.md`

---

## File structure

| File | Responsibility |
|------|----------------|
| `src/lib/taskFilters.ts` (new) | Pure list helpers: incomplete/completed split, top-N, "See all" visibility rule |
| `src/lib/taskFilters.test.ts` (new) | Unit tests for the pure helpers |
| `src/lib/sync.ts` (new) | Thin, error-swallowing wrappers over Tauri `emit`/`listen` for cross-window events + event-name constants |
| `src/lib/sync.test.ts` (new) | Unit tests that the wrappers no-op safely outside Tauri |
| `src/lib/stores/tasks.ts` (modify) | Broadcast `tasks:changed` on mutations; broadcast `task:active-changed` on `setActive`; add `applyActiveId`; expose nothing else new (filtering lives in `taskFilters.ts`) |
| `src/lib/stores/tasks.sync.test.ts` (new) | Unit tests for the new broadcast/apply behavior (mocks `../api` and `../sync`) |
| `src/lib/components/TaskRow.svelte` (new) | One task row (active dot, FO badge, checkbox+title, 🍅 badge, delete) shared by both views |
| `src/lib/components/TaskList.svelte` (modify) | Compact view: top-5 incomplete via `TaskRow` + "See all (N)" button |
| `src/lib/components/AllTasks.svelte` (new) | All-tasks window view: Incomplete + Completed sections via `TaskRow`, add input, event listeners |
| `src/lib/windows.ts` (new) | `openAllTasksWindow()` — focus existing or create the `all-tasks` window |
| `src/main.ts` (modify) | Branch on window label → mount `AllTasks` or `App` |
| `src/App.svelte` (modify) | Broadcast `timer:running-changed`; listen for `tasks:changed` / `task:active-changed`; focus backstop |
| `src-tauri/capabilities/default.json` (modify) | Add `core:webview:allow-create-webview-window` + event perms to `main` |
| `src-tauri/capabilities/all-tasks.json` (new) | Capability for the `all-tasks` window |

---

## Task 1: Pure list helpers (`taskFilters.ts`)

**Files:**
- Create: `src/lib/taskFilters.ts`
- Test: `src/lib/taskFilters.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// src/lib/taskFilters.test.ts
import { describe, it, expect } from "vitest";
import type { FoTask } from "./types";
import { incompleteTasks, completedTasks, topIncomplete, shouldShowSeeAll } from "./taskFilters";

function task(id: string, done: boolean): FoTask {
  return { id, task_id: id, title: id, is_completed: done, created_at: "", completed_at: done ? "x" : null, pomodoro_count: 0 };
}

describe("taskFilters", () => {
  const list = [task("a", false), task("b", true), task("c", false), task("d", false),
                task("e", false), task("f", false), task("g", false)]; // 6 incomplete, 1 done

  it("splits incomplete and completed", () => {
    expect(incompleteTasks(list).map((t) => t.id)).toEqual(["a", "c", "d", "e", "f", "g"]);
    expect(completedTasks(list).map((t) => t.id)).toEqual(["b"]);
  });

  it("topIncomplete returns at most n incomplete tasks in order", () => {
    expect(topIncomplete(list, 5).map((t) => t.id)).toEqual(["a", "c", "d", "e", "f"]);
    expect(topIncomplete([task("a", false)], 5)).toHaveLength(1);
  });

  it("shouldShowSeeAll is true when total > 5 OR any task is completed", () => {
    expect(shouldShowSeeAll(7, 1)).toBe(true);   // both
    expect(shouldShowSeeAll(3, 1)).toBe(true);    // has completed
    expect(shouldShowSeeAll(7, 0)).toBe(true);    // > 5
    expect(shouldShowSeeAll(5, 0)).toBe(false);   // <=5, none done
    expect(shouldShowSeeAll(0, 0)).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run (Windows): `npm test -- taskFilters`
Expected: FAIL — cannot find module `./taskFilters`.

- [ ] **Step 3: Write minimal implementation**

```ts
// src/lib/taskFilters.ts
import type { FoTask } from "./types";

export function incompleteTasks(tasks: FoTask[]): FoTask[] {
  return tasks.filter((t) => !t.is_completed);
}

export function completedTasks(tasks: FoTask[]): FoTask[] {
  return tasks.filter((t) => t.is_completed);
}

export function topIncomplete(tasks: FoTask[], n: number): FoTask[] {
  return incompleteTasks(tasks).slice(0, n);
}

export function shouldShowSeeAll(total: number, completedCount: number): boolean {
  return total > 5 || completedCount > 0;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- taskFilters`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/taskFilters.ts src/lib/taskFilters.test.ts
git commit -m "feat: pure task-filter helpers for compact/all views"
```

---

## Task 2: Cross-window event wrappers (`sync.ts`)

These wrap Tauri `emit`/`listen` so they are safe to call outside a Tauri context (unit tests, SSR) and centralize the event names.

**Files:**
- Create: `src/lib/sync.ts`
- Test: `src/lib/sync.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// src/lib/sync.test.ts
import { describe, it, expect, vi } from "vitest";

// Simulate "not in Tauri": emit/listen throw.
vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn(() => { throw new Error("no tauri"); }),
  listen: vi.fn(() => { throw new Error("no tauri"); }),
}));

import { broadcast, subscribe, EVENTS } from "./sync";

describe("sync wrappers", () => {
  it("broadcast resolves (swallows errors) outside Tauri", async () => {
    await expect(broadcast(EVENTS.tasksChanged)).resolves.toBeUndefined();
  });

  it("subscribe resolves to a no-op unlisten outside Tauri", async () => {
    const un = await subscribe(EVENTS.tasksChanged, () => {});
    expect(typeof un).toBe("function");
    expect(() => un()).not.toThrow();
  });

  it("exposes the three event names", () => {
    expect(EVENTS).toEqual({
      tasksChanged: "tasks:changed",
      activeChanged: "task:active-changed",
      timerRunning: "timer:running-changed",
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- sync`
Expected: FAIL — cannot find module `./sync`.

- [ ] **Step 3: Write minimal implementation**

```ts
// src/lib/sync.ts
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";

export const EVENTS = {
  tasksChanged: "tasks:changed",
  activeChanged: "task:active-changed",
  timerRunning: "timer:running-changed",
} as const;

/** Emit a global event to all windows. No-op (best effort) outside Tauri. */
export async function broadcast(event: string, payload?: unknown): Promise<void> {
  try {
    await emit(event, payload);
  } catch {
    // not running inside Tauri (e.g. unit tests) — ignore
  }
}

/** Listen for a global event; returns an unlisten fn (no-op outside Tauri). */
export async function subscribe(
  event: string,
  handler: (payload: unknown) => void,
): Promise<UnlistenFn> {
  try {
    return await listen(event, (e) => handler(e.payload));
  } catch {
    return () => {};
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- sync`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/sync.ts src/lib/sync.test.ts
git commit -m "feat: error-swallowing Tauri cross-window event wrappers"
```

---

## Task 3: Store broadcasts + remote active setter (`tasks.ts`)

Add event emission to mutations and a force-set `applyActiveId` for remote updates. Filtering stays in `taskFilters.ts` (Task 1). Existing `tasks.test.ts` stays green because `broadcast` no-ops outside Tauri.

**Files:**
- Modify: `src/lib/stores/tasks.ts`
- Test: `src/lib/stores/tasks.sync.test.ts` (new)

- [ ] **Step 1: Write the failing test**

```ts
// src/lib/stores/tasks.sync.test.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";
import type { FoTask } from "../types";

const inserted: FoTask[] = [];
vi.mock("../api", () => ({
  api: {
    getTasks: vi.fn(async () => []),
    insertTask: vi.fn(async (title: string) => {
      const t: FoTask = { id: `id-${inserted.length + 1}`, task_id: `FO-0${inserted.length + 1}`,
        title, is_completed: false, created_at: "", completed_at: null, pomodoro_count: 0 };
      inserted.push(t); return t;
    }),
    updateTask: vi.fn(async () => {}),
    deleteTask: vi.fn(async () => {}),
    recordSession: vi.fn(async () => {}),
  },
}));
vi.mock("../sync", () => ({
  EVENTS: { tasksChanged: "tasks:changed", activeChanged: "task:active-changed", timerRunning: "timer:running-changed" },
  broadcast: vi.fn(async () => {}),
  subscribe: vi.fn(async () => () => {}),
}));

import { createTasksStore } from "./tasks";
import { broadcast } from "../sync";

beforeEach(() => { inserted.length = 0; vi.clearAllMocks(); });

describe("tasks store cross-window broadcasts", () => {
  it("broadcasts tasks:changed after add", async () => {
    const s = createTasksStore();
    await s.add("A");
    expect(broadcast).toHaveBeenCalledWith("tasks:changed");
  });

  it("broadcasts task:active-changed with the new id on setActive", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setActive(a);
    expect(broadcast).toHaveBeenCalledWith("task:active-changed", a.id);
    s.setActive(a); // toggle off -> broadcasts null
    expect(broadcast).toHaveBeenCalledWith("task:active-changed", null);
  });

  it("applyActiveId force-sets the active id without broadcasting", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    (broadcast as ReturnType<typeof vi.fn>).mockClear();
    s.applyActiveId(a.id);
    expect(get(s.activeTaskId)).toBe(a.id);
    expect(broadcast).not.toHaveBeenCalled();
    s.applyActiveId(null);
    expect(get(s.activeTaskId)).toBe(null);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tasks.sync`
Expected: FAIL — `broadcast` not called / `applyActiveId` is not a function.

- [ ] **Step 3: Write minimal implementation**

Edit `src/lib/stores/tasks.ts`:

1. Add the import near the top (after the `api` import):
```ts
import { broadcast, EVENTS } from "../sync";
```

2. In `add`, after `newTaskTitle.set("")`, add:
```ts
      await broadcast(EVENTS.tasksChanged);
```

3. In `toggle`, after the `await api.updateTask(updated)` try/catch block (still inside the function), add:
```ts
    await broadcast(EVENTS.tasksChanged);
```

4. In `remove`, after the `await api.deleteTask(task.id)` try/catch block, add:
```ts
    await broadcast(EVENTS.tasksChanged);
```

5. In `onFocusSessionCompleted`, after the pomodoro `api.updateTask` try/catch (before `recordSession`), add:
```ts
        await broadcast(EVENTS.tasksChanged);
```

6. Replace `setActive` with a version that broadcasts the resulting id:
```ts
  function setActive(task: FoTask) {
    const current = get(activeTaskId);
    if (timerRunning && current !== null && current !== task.id) {
      switchedDuringSession = true;
    }
    const next = current === task.id ? null : task.id;
    activeTaskId.set(next);
    void broadcast(EVENTS.activeChanged, next);
  }
```

7. Add a remote setter (force-set, no broadcast) and export it. Add the function above `registerTaskToggled`:
```ts
  // Apply an active-task change received from another window. Force-sets the id
  // (no toggle, no re-broadcast) and respects the mid-session switch rule.
  function applyActiveId(id: string | null) {
    const current = get(activeTaskId);
    if (id === current) return;
    if (timerRunning && current !== null && id !== null) switchedDuringSession = true;
    activeTaskId.set(id);
  }
```

8. Add `applyActiveId` to the returned object:
```ts
    onFocusSessionCompleted, registerTaskToggled, applyActiveId,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- tasks`
Expected: PASS — both `tasks.test.ts` (unchanged behavior) and `tasks.sync.test.ts`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/tasks.ts src/lib/stores/tasks.sync.test.ts
git commit -m "feat: store broadcasts task/active changes for cross-window sync"
```

---

## Task 4: Extract `TaskRow.svelte` (refactor, no behavior change)

**Files:**
- Create: `src/lib/components/TaskRow.svelte`
- Modify: `src/lib/components/TaskList.svelte`

- [ ] **Step 1: Create `TaskRow.svelte`**

```svelte
<!-- src/lib/components/TaskRow.svelte -->
<script lang="ts">
  import type { FoTask } from "../types";
  import type { TasksStore } from "../stores/tasks";

  let { task, store, activeId, timerRunning = false }:
    { task: FoTask; store: TasksStore; activeId: string | null; timerRunning?: boolean } = $props();
</script>

<div class="row">
  <button
    class="active-dot"
    class:on={activeId === task.id}
    title="Set active task"
    onclick={() => store.setActive(task)}
    aria-label={`Set active task: ${task.title}`}
  ></button>
  <span class="badge">{task.task_id}</span>
  <label class="check">
    <input type="checkbox" checked={task.is_completed} onchange={() => store.toggle(task)} />
    <span class="title" class:done={task.is_completed}>{task.title}</span>
  </label>
  {#if task.pomodoro_count > 0}
    <span class="pomo-badge">🍅×{task.pomodoro_count}</span>
  {/if}
  <button class="danger del" title="Delete" aria-label={`Delete ${task.title}`} onclick={() => store.remove(task)}>×</button>
</div>

<style>
  .row { display: flex; align-items: center; gap: 5px; margin-top: 3px; }
  .active-dot { width: 9px; height: 9px; padding: 0; border-radius: 50%; background: transparent; border: 1.5px solid var(--subtext); }
  .active-dot.on { background: var(--red); border-color: var(--red); }
  .badge { font-family: ui-monospace, "Consolas", monospace; font-size: 10px; color: var(--accent); min-width: 40px; }
  .check { display: flex; align-items: center; gap: 6px; flex: 1; cursor: pointer; }
  .check input { accent-color: var(--accent); }
  .title { font-size: 13px; color: var(--text); }
  .title.done { opacity: 0.55; text-decoration: line-through; }
  .pomo-badge { font-size: 10px; color: var(--yellow); }
  .del { width: 24px; height: 24px; padding: 0; font-size: 13px; }
</style>
```

- [ ] **Step 2: Use it in `TaskList.svelte`**

In `src/lib/components/TaskList.svelte`:

Add to the `<script>` block:
```ts
  import TaskRow from "./TaskRow.svelte";
```

Replace the `.list` block (the `{#each $tasks as task (task.id)} ... {/each}` and the full `.row` markup inside it) with:
```svelte
      <div class="list">
        {#each $tasks as task (task.id)}
          <TaskRow {task} {store} activeId={$activeTaskId} {timerRunning} />
        {/each}
      </div>
```

Delete the now-unused row-related styles from `TaskList.svelte` (`.row`, `.active-dot`, `.active-dot.on`, `.badge`, `.check`, `.check input`, `.title`, `.title.done`, `.pomo-badge`, `.del`). Keep `.tasks`, `.header`, `.header-right`, `.count`, `.notice`, `.list`, `.add-row`, `.add`.

- [ ] **Step 3: Typecheck + run app (Windows)**

Run: `npm run check`
Expected: 0 errors.
Run: `npm run tauri dev` → the task list looks and behaves identical to before (active dot, toggle, delete, 🍅 badge).

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/TaskRow.svelte src/lib/components/TaskList.svelte
git commit -m "refactor: extract TaskRow component shared by task views"
```

---

## Task 5: Compact view — top 5 incomplete + "See all (N)" button

**Files:**
- Modify: `src/lib/components/TaskList.svelte`

- [ ] **Step 1: Add filtering + visibility logic to the script**

In `src/lib/components/TaskList.svelte` `<script>`, add imports and derived values:
```ts
  import { topIncomplete, shouldShowSeeAll, completedTasks } from "../taskFilters";
  import { openAllTasksWindow } from "../windows";

  const visible = $derived(topIncomplete($tasks, 5));
  const total = $derived($tasks.length);
  const completedCount = $derived(completedTasks($tasks).length);
  const showSeeAll = $derived(shouldShowSeeAll(total, completedCount));
```
> `openAllTasksWindow` is created in Task 6. If implementing strictly in order, temporarily stub the onclick with `() => {}` and wire it in Task 6 — but Task 6 follows immediately, so prefer importing now.

- [ ] **Step 2: Render the visible subset + button**

Replace the `.list` each-loop source from `$tasks` to `visible`, and add the "See all" button after the `.list` div:
```svelte
      <div class="list">
        {#each visible as task (task.id)}
          <TaskRow {task} {store} activeId={$activeTaskId} {timerRunning} />
        {/each}
      </div>

      {#if showSeeAll}
        <button class="see-all" onclick={() => openAllTasksWindow()}>See all ({total}) ▸</button>
      {/if}
```

- [ ] **Step 3: Add button style (consistent with app tokens)**

Add to `TaskList.svelte` `<style>`:
```css
  .see-all { width: 100%; margin-top: 6px; font-size: 11px; color: var(--accent); background: none; border: none; cursor: pointer; padding: 4px; }
  .see-all:hover { text-decoration: underline; }
```

- [ ] **Step 4: Typecheck**

Run: `npm run check`
Expected: 0 errors (assumes Task 6's `windows.ts` exists; do Task 6 before re-running if needed).

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/TaskList.svelte
git commit -m "feat: compact task list shows top 5 incomplete + See all button"
```

---

## Task 6: Window opener (`windows.ts`) — focus or create

**Files:**
- Create: `src/lib/windows.ts`

- [ ] **Step 1: Implement the opener**

```ts
// src/lib/windows.ts
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

const ALL_TASKS_LABEL = "all-tasks";

/** Open the All Tasks window, or focus it if it's already open (no duplicates). */
export async function openAllTasksWindow(): Promise<void> {
  const existing = await WebviewWindow.getByLabel(ALL_TASKS_LABEL);
  if (existing) {
    await existing.setFocus();
    return;
  }
  const win = new WebviewWindow(ALL_TASKS_LABEL, {
    url: "index.html",
    title: "FoPoMoro — All Tasks",
    width: 520,
    height: 640,
    resizable: true,
    decorations: true,
    alwaysOnTop: false,
  });
  win.once("tauri://error", (e) => console.error("[windows] all-tasks create failed:", e));
}
```

- [ ] **Step 2: Typecheck**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/windows.ts
git commit -m "feat: openAllTasksWindow focuses or creates the all-tasks window"
```

---

## Task 7: `main.ts` branch + `AllTasks.svelte` view

**Files:**
- Modify: `src/main.ts`
- Create: `src/lib/components/AllTasks.svelte`

- [ ] **Step 1: Branch the mount target on window label**

Replace `src/main.ts` with:
```ts
import { mount } from "svelte";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./theme.css";
import App from "./App.svelte";
import AllTasks from "./lib/components/AllTasks.svelte";

const label = getCurrentWebviewWindow().label;
const Component = label === "all-tasks" ? AllTasks : App;

const app = mount(Component, {
  target: document.getElementById("app")!,
});

export default app;
```

- [ ] **Step 2: Create `AllTasks.svelte` (view + listeners)**

```svelte
<!-- src/lib/components/AllTasks.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { createTasksStore } from "../stores/tasks";
  import { createSettingsStore } from "../stores/settings";
  import { subscribe, EVENTS } from "../sync";
  import { incompleteTasks, completedTasks } from "../taskFilters";
  import { textColorsFor } from "../contrast";
  import TaskRow from "./TaskRow.svelte";

  const store = createTasksStore();
  const tasks = $derived(store.tasks);
  const newTaskTitle = $derived(store.newTaskTitle);
  const activeTaskId = $derived(store.activeTaskId);

  const settings = createSettingsStore();
  const bgColor = settings.bgColor;
  const textColors = $derived(textColorsFor($bgColor));

  let timerRunning = $state(false);
  const incomplete = $derived(incompleteTasks($tasks));
  const completed = $derived(completedTasks($tasks));
  const doneCount = $derived(completed.length);

  let unlisteners: Array<() => void> = [];
  let unfocus: (() => void) | undefined;

  function onInputKey(e: KeyboardEvent) {
    if (e.key === "Enter") store.add();
  }

  onMount(async () => {
    await settings.load();
    await store.load();

    unlisteners.push(await subscribe(EVENTS.tasksChanged, () => { void store.load(); }));
    unlisteners.push(await subscribe(EVENTS.activeChanged, (id) => store.applyActiveId(id as string | null)));
    unlisteners.push(await subscribe(EVENTS.timerRunning, (running) => { timerRunning = running as boolean; store.setTimerRunning(running as boolean); }));

    // Backstop: refetch when this window regains focus.
    unfocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) void store.load();
    });
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
    unfocus?.();
  });
</script>

<main class="all" style="--panel-bg:{$bgColor}; --text:{textColors.text}; --subtext:{textColors.subtext}">
  <header class="bar">
    <span class="title">All Tasks</span>
    <span class="count">{doneCount} / {$tasks.length} done</span>
  </header>

  <div class="scroll">
    <div class="section-header">INCOMPLETE</div>
    {#each incomplete as task (task.id)}
      <TaskRow {task} {store} activeId={$activeTaskId} {timerRunning} />
    {/each}
    {#if incomplete.length === 0}<div class="empty">No incomplete tasks 🎉</div>{/if}

    <div class="section-header done-header">COMPLETED</div>
    {#each completed as task (task.id)}
      <TaskRow {task} {store} activeId={$activeTaskId} {timerRunning} />
    {/each}
    {#if completed.length === 0}<div class="empty">No completed tasks yet</div>{/if}
  </div>

  <div class="add-row">
    <input type="text" placeholder="Add a task…"
      value={$newTaskTitle}
      oninput={(e) => store.newTaskTitle.set((e.target as HTMLInputElement).value)}
      onkeydown={onInputKey} />
    <button class="add" onclick={() => store.add()}>+</button>
  </div>
</main>

<style>
  .all { box-sizing: border-box; min-height: 100vh; padding: 16px; background: var(--panel-bg); display: flex; flex-direction: column; }
  .bar { display: flex; align-items: center; justify-content: space-between; margin-bottom: 10px; }
  .title { font-size: 15px; font-weight: 600; color: var(--text); }
  .count { font-size: 11px; color: var(--subtext); }
  .scroll { flex: 1; overflow-y: auto; }
  .section-header { font-size: 10px; letter-spacing: 0.08em; color: var(--subtext); margin: 10px 0 4px; }
  .done-header { margin-top: 16px; }
  .empty { font-size: 12px; color: var(--subtext); opacity: 0.7; padding: 4px 0; }
  .add-row { display: flex; gap: 6px; margin-top: 12px; }
  .add-row input { flex: 1; }
  .add { width: 36px; padding: 0; }
</style>
```
> `createSettingsStore`, `textColorsFor`, and `getCurrentWindow().onFocusChanged` already exist (used in `App.svelte`). `store.setTimerRunning`, `store.applyActiveId`, `store.load`, `store.add` exist after Task 3.

- [ ] **Step 3: Typecheck**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add src/main.ts src/lib/components/AllTasks.svelte
git commit -m "feat: AllTasks window view + label-based mount branch"
```

---

## Task 8: Wire main-window events (broadcast timer, listen for changes)

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Broadcast timer state + add listeners**

In `src/App.svelte` `<script>`, add imports:
```ts
  import { subscribe, broadcast, EVENTS } from "./lib/sync";
```

Replace the existing timer-running effect:
```ts
  $effect(() => {
    tasksStore.setTimerRunning($pomodoroState.isRunning);
  });
```
with one that also broadcasts (main is the sole timer owner):
```ts
  $effect(() => {
    const running = $pomodoroState.isRunning;
    tasksStore.setTimerRunning(running);
    void broadcast(EVENTS.timerRunning, running);
  });
```

- [ ] **Step 2: Listen for cross-window changes in `onMount`**

At the end of the existing `onMount` callback in `App.svelte` (after the ResizeObserver setup), add:
```ts
    // Cross-window sync: reflect changes made in the All Tasks window.
    appUnlisteners.push(await subscribe(EVENTS.tasksChanged, () => { void tasksStore.load(); void progressStore.load(); }));
    appUnlisteners.push(await subscribe(EVENTS.activeChanged, (id) => tasksStore.applyActiveId(id as string | null)));
    appUnlisteners.push(await subscribe(EVENTS.timerRunning, (running) => tasksStore.setTimerRunning(running as boolean)));
    appUnfocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) void tasksStore.load();
    });
```

Add the holder declarations near the other `let` declarations (e.g. after `let ro: ResizeObserver | undefined;`):
```ts
  let appUnlisteners: Array<() => void> = [];
  let appUnfocus: (() => void) | undefined;
```

Add cleanup in the existing `onDestroy`:
```ts
  onDestroy(() => {
    pomodoro.dispose();
    ro?.disconnect();
    appUnlisteners.forEach((u) => u());
    appUnfocus?.();
  });
```
> Note: the main window also receives its own `timer:running-changed` / `task:active-changed` broadcasts; `setTimerRunning`/`applyActiveId` are idempotent so self-receipt is harmless.

- [ ] **Step 3: Typecheck**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add src/App.svelte
git commit -m "feat: main window broadcasts timer state and syncs from All Tasks"
```

---

## Task 9: Capabilities for window creation + events

**Files:**
- Modify: `src-tauri/capabilities/default.json`
- Create: `src-tauri/capabilities/all-tasks.json`

- [ ] **Step 1: Grant the main window window-creation + events**

Edit `src-tauri/capabilities/default.json` — add these entries to the `permissions` array (keep existing ones):
```json
    "core:webview:allow-create-webview-window",
    "core:event:default"
```

- [ ] **Step 2: Create the all-tasks capability**

```json
// src-tauri/capabilities/all-tasks.json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "all-tasks",
  "description": "Capability for the All Tasks window",
  "windows": ["all-tasks"],
  "permissions": [
    "core:default",
    "core:window:allow-close",
    "core:window:allow-minimize",
    "core:event:default"
  ]
}
```

- [ ] **Step 3: Rebuild (capabilities are compiled in)**

Run (Windows): `npm run tauri dev`
Expected: app launches; no ACL/permission errors in the console when opening the All Tasks window.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/capabilities/default.json src-tauri/capabilities/all-tasks.json
git commit -m "feat: capabilities for all-tasks window creation + cross-window events"
```

---

## Task 10: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Unit tests + typecheck (Windows)**

Run: `npm test`
Expected: all suites pass (incl. `taskFilters`, `sync`, `tasks`, `tasks.sync`).
Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 2: Manual test matrix (Windows, `npm run tauri dev`)**

- [ ] Main panel shows at most 5 incomplete tasks; completed tasks are absent.
- [ ] "See all (N)" appears only when total > 5 OR a task is completed; hidden otherwise; N = total.
- [ ] Click "See all" → All Tasks window opens (520×640, resizable, has native title bar, not always-on-top, in taskbar).
- [ ] Click "See all" again with the window open → focuses the existing window (no duplicate).
- [ ] Add/toggle/delete in All Tasks → reflected in the main panel within a moment; and vice-versa.
- [ ] Set active task in either window → red dot matches in both.
- [ ] Start a focus session in the main window → in All Tasks, toggling/deleting a non-active task is blocked (lock applies).
- [ ] Completing a task in the main panel removes it from the compact list and moves it under "Completed" in All Tasks.
- [ ] Signed out (local only): All Tasks still opens and lists local tasks.

- [ ] **Step 3: Commit (if any tweaks were needed during verification)**

```bash
git add -A
git commit -m "test: verify See-all window end-to-end"
```

---

## Self-review notes (author)

- **Spec coverage:** window creation (T6/T7), label branch (T7), 3-event full sync (T3/T7/T8), focus backstop (T7/T8), TaskRow extraction (T4), compact top-5 + button rule (T1/T5), AllTasks sections (T7), capabilities (T9), tests (T1/T2/T3/T10). All spec sections mapped.
- **Type consistency:** `applyActiveId(id: string | null)`, `setTimerRunning(running: boolean)`, `EVENTS.{tasksChanged,activeChanged,timerRunning}`, `openAllTasksWindow()`, label `"all-tasks"` used identically across tasks.
- **Known minor limitation (documented):** windows receive their own broadcasts; handlers are idempotent so this is harmless (one extra refetch per local mutation).
