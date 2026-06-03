# FoPoMoro Mac Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Windows WPF app FoPoMoro to a native macOS overlay (Tauri v2 + Svelte 5 + TypeScript) with full feature parity — clock, Pomodoro timer, task list, progress stats, and Supabase cloud sync — sharing the same Supabase backend/schema as the Windows app.

**Architecture:** Mirror the original MVVM split onto Tauri/Svelte: all network + file I/O lives in **Rust** behind `#[tauri::command]` functions (= Services); reactive **Svelte stores** hold app state (= ViewModels); stateless **Svelte components** render (= Views). The Pomodoro countdown and clock run in the frontend (a store with `setInterval`). The Supabase anon key never reaches the webview. When `.env` is present and complete, Supabase is the source of truth for tasks; otherwise the app falls back to local JSON. Progress and settings are always local.

**Tech Stack:** Tauri v2 (Rust core, macOS WKWebView shell), Svelte 5 + Vite + TypeScript, `reqwest` (Supabase REST), `serde`/`serde_json` + `std::fs` (storage), `chrono` (dates/UUID via `uuid`), `tauri-plugin-window-state` (position memory), `tauri-plugin-notification` (toasts), Web Audio API (session-end chime). Tests: `cargo test` (Rust), Vitest (store logic).

---

## Source of Truth

The Windows app at `../fo-po-moro/FoPoMoro` is the parity reference. Key files already analyzed:
- `Models/{FoTask,FoSession,PomodoroConfig,WindowSettings}.cs`
- `Services/{DataService,SupabaseService,NotificationService}.cs`
- `ViewModels/{MainViewModel,TimerViewModel,TaskViewModel}.cs`
- `MainWindow.xaml(.cs)`, `App.xaml` (Catppuccin theme), `Views/*Panel.xaml`, `NativeMethods.cs`

### Catppuccin Mocha palette (from `App.xaml`)
| Token | Hex |
|---|---|
| base | `#1E1E2E` |
| surface | `#313244` |
| text | `#CDD6F4` |
| subtext | `#BAC2DE` |
| accent | `#89B4FA` |
| green | `#A6E3A1` |
| red | `#F38BA8` |
| yellow | `#F9E2AF` |
| panel bg (80% alpha) | `#CC1E1E2E` |
| panel bg, click-through | `#CC0D1B2E` |
| primary hover / pressed | `#A6C8FF` / `#6B9CDB` |
| secondary hover / pressed | `#3D3D52` / `#2A2A3C` |
| danger hover / pressed | `#F5A0B5` / `#D96B85` |

### Layout (single 290px-wide panel, top → bottom)
1. **Title bar** (drag region): "FoPoMoro" label · click-through toggle `⊙` · close `✕`
2. **CLOCK**: header + collapse `▾`; time `HH:mm:ss` (38px, light); date `dddd, dd MMMM yyyy` (12px)
3. **POMODORO**: header + 4 session dots + `⚙` config + collapse; state label (color by state) + countdown `mm:ss` (52px accent) + Start/Pause/Reset; config sub-panel (Focus/Short/Long with `−`/`+`/input, "min", Cancel/Apply)
4. **TASKS**: header + "X / Y done" + collapse; timer-running notice; task rows (active dot · `FO-NN` badge · checkbox+title · `🍅×N` badge · `×` delete); add row (input + Enter, `+` button)
5. **TODAY**: header + collapse; 3 columns "N sessions" · "N min" · "N tasks"
6. **Opacity** slider (0.3–1.0)
7. **Click-through active** banner (visible only when enabled)

### Exact behaviors to preserve
- **Timer is NOT auto-continue.** On every session end the timer stops and `isRunning=false`; the user presses Start to begin the next session. (`TimerViewModel.TransitionTo` sets `IsRunning=false`.)
- **Long-break cadence:** focus sessions count 1→2→3; on the 4th focus completion the counter resets to 0 and it transitions to LongBreak. (`newCount >= SessionsBeforeLongBreak(4)`.)
- **Session-complete recording (focus only):** Only **focus** completions call `record_session`; breaks only fire a notification. (`MainViewModel.OnSessionCompleted`: `if (isFocus) { ...record... } else { notify only }`.)
- **Task-switch quirk:** if the active task is switched while the timer is running, the completed focus session does **not** increment any task's pomodoro count, and it is recorded with `task_id=null, was_focused=false`. Otherwise `task_id=activeTask.id, was_focused=true` and `activeTask.pomodoro_count++`. (`TaskViewModel.OnFocusSessionCompleted` + `IsTimerRunning` setter resets the switch flag when the timer stops.)
- **Locked task ops while running:** cannot delete any task while the timer runs; cannot toggle a task other than the active one. (`ExecuteDeleteTask`/`ExecuteToggleTask` guards.)
- **`FO-NN` numbering:** offline, next number = `max(existing FO-NN) + 1`, formatted `FO-{n:02}`. Online, Supabase assigns `task_number`; the display id is derived `FO-{task_number:02}`.
- **Daily reset:** on load, if stored `session.date != today`, reset the session to zero and persist.
- **Opacity default 0.9**, range 0.3–1.0, persisted in `settings.json`.

### Supabase REST contract (from `SupabaseService.cs` — must match exactly)
- Base URL: `{SUPABASE_URL}/rest/v1`
- Headers on every request: `apikey: {key}`, `Authorization: Bearer {key}`, `Content-Type: application/json`
- `GET /tasks?select=*&order=task_number.asc` → array of `{id, task_number, title, is_completed, created_at, completed_at, pomodoro_count}`
- `POST /tasks` with header `Prefer: return=representation`, body `{title, is_completed, created_at, completed_at, pomodoro_count}` (NO `id`, NO `task_number` — DB assigns) → returns `[record]`; read back `id` and `task_number`
- `PATCH /tasks?id=eq.{id}`, body `{title, is_completed, completed_at, pomodoro_count}` (NO `created_at`, NO `task_number`)
- `DELETE /tasks?id=eq.{id}`
- `POST /pomodoro_sessions`, body `{task_id (nullable), duration_minutes, was_focused}`
- `IsAvailable` = both `SUPABASE_URL` and `SUPABASE_ANON_KEY` are present and non-empty.

### Intentional deviations from the WPF source (all approved by spec intent)
1. **Sound:** Windows plays `SystemSounds.Exclamation`. The spec asks for a session-end sound via Web Audio. To avoid bundling a sourced binary, we generate a short two-note chime with the Web Audio API (`AudioContext` + `OscillatorNode`). Swapping in a real `timer-end.wav` later is trivial (one function).
2. **Position memory:** Windows persists `Left/Top` in `settings.json`. We delegate window position/size to `tauri-plugin-window-state`; `settings.json` therefore holds only `opacity`. Default first-run position = top-right corner.
3. **Click-through:** Windows uses Win32 `WS_EX_TRANSPARENT`; we use `WebviewWindow::set_ignore_cursor_events` via a Rust command. Not persisted (resets to off on launch — matches Windows).
4. **Tray icon:** Windows uses a tray `NotifyIcon` whose context menu disables click-through. v1 Mac has no tray; click-through is toggled from the in-panel `⊙` button and can be disabled there. (Tray is a possible follow-up; note it in the banner text accordingly.)

---

## File Structure

```
fopomoro-mac/
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── Clock.svelte         # CLOCK section
│   │   │   ├── Pomodoro.svelte      # POMODORO section + config sub-panel
│   │   │   ├── TaskList.svelte      # TASKS section
│   │   │   └── Progress.svelte      # TODAY section
│   │   ├── stores/
│   │   │   ├── timer.ts             # clock + pomodoro state machine (ViewModel)
│   │   │   ├── tasks.ts             # task CRUD, active task, switch quirk (ViewModel)
│   │   │   ├── progress.ts          # daily session stats (ViewModel)
│   │   │   └── settings.ts          # opacity + click-through (ViewModel)
│   │   ├── api.ts                   # invoke() wrappers to Rust commands
│   │   └── types.ts                 # mirrored model types
│   ├── App.svelte                   # = MainWindow: composition root, overlay, wiring
│   ├── main.ts                      # mounts App
│   └── theme.css                    # Catppuccin tokens + shared element styles
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                  # thin entry → lib::run()
│   │   ├── lib.rs                   # build state, register plugins + commands
│   │   ├── models.rs                # FoTask, FoSession, PomodoroConfig, WindowSettings
│   │   ├── storage.rs               # = DataService (serde_json + fs)
│   │   ├── supabase.rs              # = SupabaseService (reqwest)
│   │   └── commands.rs              # #[tauri::command] surface
│   ├── capabilities/default.json    # Tauri v2 permissions
│   ├── tauri.conf.json              # transparent / decorations:false / alwaysOnTop
│   └── Cargo.toml
├── .env.example                     # SUPABASE_URL, SUPABASE_ANON_KEY
├── package.json
├── vitest.config.ts
└── README.md
```

### Rust command surface (frontend ↔ core contract)
```
get_tasks() -> Vec<FoTask>                                   // Supabase if available (caches local), else local
insert_task(title) -> FoTask                                 // computes FO-NN, inserts, caches
update_task(task: FoTask) -> ()                              // PATCH if available, updates cache
delete_task(id: String) -> ()                                // DELETE if available, removes from cache
record_session(task_id: Option<String>, duration_minutes, was_focused) -> ()
load_progress() -> FoSession                                 // daily-reset check inside
save_progress(session: FoSession) -> ()
load_config() -> PomodoroConfig
save_config(config: PomodoroConfig) -> ()
load_settings() -> WindowSettings
save_settings(settings: WindowSettings) -> ()
set_click_through(enabled: bool) -> ()                       // window.set_ignore_cursor_events
```
(The WPF spec sketch omitted `save_progress` and the config commands; they are required because the frontend updates progress counts and pomodoro durations and Rust owns persistence.)

---

## Task 1: Scaffold project + transparent overlay window

**Files:**
- Create (scaffold): `package.json`, `src-tauri/Cargo.toml`, `src/main.ts`, `src/App.svelte`, `index.html`, `vite.config.ts`, etc.
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Create: `.gitignore`

- [ ] **Step 1: Scaffold a Tauri v2 + Svelte-TS app into the current (empty) repo**

The repo already has `.git/` and `docs/`. Scaffold into a temp dir, then move files in (so the scaffolder doesn't refuse a non-empty dir).

```bash
cd /Users/muchamad_mafmudin/project
npm create tauri-app@latest fopomoro-mac-scaffold -- --template svelte-ts --manager npm --yes
# Move generated files into the existing repo (keep .git and docs)
rsync -a --exclude '.git' fopomoro-mac-scaffold/ fopomoro-mac/
rm -rf fopomoro-mac-scaffold
cd fopomoro-mac
npm install
```

Expected: `fopomoro-mac/` now contains `package.json`, `src/`, `src-tauri/`, `index.html`, `vite.config.ts`, `tsconfig.json`. `docs/` is untouched.

- [ ] **Step 2: Confirm the dev toolchain is present**

Run:
```bash
rustc --version && cargo --version && node --version && xcode-select -p
```
Expected: Rust + Cargo versions print, Node prints, and `xcode-select -p` prints a developer dir path (Command Line Tools installed). If any are missing, install before continuing.

- [ ] **Step 3: Configure the overlay window in `tauri.conf.json`**

Replace `src-tauri/tauri.conf.json` with:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "FoPoMoro",
  "version": "0.1.0",
  "identifier": "com.fopomoro.mac",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "macOSPrivateApi": true,
    "withGlobalTauri": false,
    "windows": [
      {
        "title": "FoPoMoro",
        "label": "main",
        "width": 290,
        "height": 620,
        "minWidth": 290,
        "maxWidth": 290,
        "resizable": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "shadow": false,
        "skipTaskbar": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```
Note: `macOSPrivateApi: true` is REQUIRED for a transparent window on macOS (it disables App Store submission — fine for dev-local v1).

- [ ] **Step 4: Make the webview background transparent**

Replace `index.html`'s `<body>`/global CSS so the page is transparent. In `src/App.svelte` (will be replaced later) and/or `src/styles.css`, ensure:
```css
html, body {
  margin: 0;
  padding: 0;
  background: transparent;
}
```
If the scaffold has `src/styles.css`, set its body background to `transparent` and remove any opaque background. Confirm `index.html` imports that stylesheet.

- [ ] **Step 5: Add a minimal draggable skeleton panel**

Replace `src/App.svelte` with a placeholder that proves transparency + drag:
```svelte
<script lang="ts">
</script>

<main class="panel">
  <header class="titlebar" data-tauri-drag-region>
    <span class="title">FoPoMoro</span>
  </header>
  <p class="placeholder">overlay skeleton</p>
</main>

<style>
  .panel {
    width: 290px;
    box-sizing: border-box;
    padding: 14px;
    border-radius: 16px;
    background: #CC1E1E2E;
    color: #CDD6F4;
    font-family: -apple-system, "Segoe UI", system-ui, sans-serif;
  }
  .titlebar {
    cursor: grab;
    user-select: none;
    margin-bottom: 10px;
  }
  .title { font-size: 13px; font-weight: 600; }
  .placeholder { color: #BAC2DE; font-size: 12px; }
</style>
```

- [ ] **Step 6: Set the Vite dev port to 1420 (matches `devUrl`)**

In `vite.config.ts`, ensure the server block:
```ts
server: {
  port: 1420,
  strictPort: true,
},
```

- [ ] **Step 7: Run the app and verify the overlay manually**

Run:
```bash
npm run tauri dev
```
Expected: a borderless, semi-transparent rounded panel appears, always on top of other windows, with no title bar/traffic lights. Dragging the "FoPoMoro" header moves the window. The area outside the rounded panel is transparent (you can see desktop through the corners). Close the window to stop.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri v2 + Svelte overlay skeleton"
```

---

## Task 2: Theme tokens + App shell layout

**Files:**
- Create: `src/theme.css`
- Modify: `src/App.svelte`
- Modify: `src/main.ts` (import theme.css)

- [ ] **Step 1: Create the theme stylesheet with Catppuccin tokens + shared element styles**

Create `src/theme.css`:
```css
:root {
  --base: #1E1E2E;
  --surface: #313244;
  --text: #CDD6F4;
  --subtext: #BAC2DE;
  --accent: #89B4FA;
  --green: #A6E3A1;
  --red: #F38BA8;
  --yellow: #F9E2AF;
  --panel-bg: #CC1E1E2E;
  --panel-bg-clickthrough: #CC0D1B2E;
  --accent-hover: #A6C8FF;
  --accent-press: #6B9CDB;
  --surface-hover: #3D3D52;
  --surface-press: #2A2A3C;
  --danger-hover: #F5A0B5;
  --danger-press: #D96B85;
}

html, body {
  margin: 0;
  padding: 0;
  background: transparent;
  overflow: hidden;
  font-family: -apple-system, "Segoe UI", system-ui, sans-serif;
  color: var(--text);
}

.section-header {
  color: var(--subtext);
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
}

button {
  font-family: inherit;
  border: none;
  cursor: pointer;
  border-radius: 8px;
  padding: 6px 12px;
  font-weight: 600;
  color: var(--base);
  background: var(--accent);
  transition: background 0.12s ease;
}
button:hover { background: var(--accent-hover); }
button:active { background: var(--accent-press); }
button:disabled { opacity: 0.5; cursor: default; }

button.secondary { background: var(--surface); color: var(--text); }
button.secondary:hover { background: var(--surface-hover); }
button.secondary:active { background: var(--surface-press); }

button.danger { background: var(--red); padding: 6px 4px; }
button.danger:hover { background: var(--danger-hover); }
button.danger:active { background: var(--danger-press); }

button.icon {
  background: transparent;
  color: var(--subtext);
  padding: 2px 4px;
  font-weight: 400;
}
button.icon:hover { background: transparent; color: var(--text); }
button.icon.on { color: var(--accent); }

input[type="text"], input.num {
  box-sizing: border-box;
  background: var(--surface);
  color: var(--text);
  border: 1.5px solid var(--surface);
  border-radius: 8px;
  padding: 8px 6px;
  font-family: inherit;
  font-size: 13px;
  caret-color: var(--accent);
}
input[type="text"]:focus, input.num:focus { outline: none; border-color: var(--accent); }
```

- [ ] **Step 2: Import the theme in `main.ts`**

In `src/main.ts`, add at the top (alongside any existing style import):
```ts
import "./theme.css";
```

- [ ] **Step 3: Build the App shell with the full panel skeleton**

Replace `src/App.svelte` with the shell (sections are placeholders for now; later tasks fill them):
```svelte
<script lang="ts">
</script>

<main class="panel">
  <header class="titlebar" data-tauri-drag-region>
    <span class="title">FoPoMoro</span>
    <div class="titlebar-actions">
      <button class="icon" title="Toggle click-through">⊙</button>
      <button class="danger close" title="Close">✕</button>
    </div>
  </header>

  <section class="slot"><span class="section-header">CLOCK</span></section>
  <section class="slot"><span class="section-header">POMODORO</span></section>
  <section class="slot"><span class="section-header">TASKS</span></section>
  <section class="slot"><span class="section-header">TODAY</span></section>

  <div class="opacity-row">
    <span class="section-header">Opacity</span>
    <input type="range" min="0.3" max="1" step="0.01" value="0.9" />
  </div>
</main>

<style>
  .panel {
    width: 290px;
    box-sizing: border-box;
    padding: 14px;
    border-radius: 16px;
    background: var(--panel-bg);
  }
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: grab;
    user-select: none;
    margin-bottom: 10px;
  }
  .title { font-size: 13px; font-weight: 600; color: var(--text); }
  .titlebar-actions { display: flex; align-items: center; gap: 4px; }
  .close { width: 24px; height: 24px; padding: 0; font-size: 11px; }
  .slot { margin-bottom: 8px; }
  .opacity-row { display: flex; align-items: center; gap: 8px; margin-top: 4px; }
  .opacity-row input[type="range"] { flex: 1; accent-color: var(--accent); }
</style>
```

- [ ] **Step 4: Run and verify the shell renders with the dark theme**

Run:
```bash
npm run tauri dev
```
Expected: the panel shows the title bar with `⊙` and `✕` buttons, four labeled section slots, and an opacity slider. Colors match Catppuccin (dark panel, light text, blue accent on the slider). Close to stop. (Buttons do nothing yet.)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add Catppuccin theme tokens and app shell layout"
```

---

## Task 3: Clock store + Clock component

**Files:**
- Create: `src/lib/stores/timer.ts` (clock portion now; pomodoro added in Task 4)
- Create: `src/lib/components/Clock.svelte`
- Create: `vitest.config.ts`
- Create: `src/lib/stores/timer.test.ts`
- Modify: `package.json` (add vitest + test script)
- Modify: `src/App.svelte` (mount Clock)

- [ ] **Step 1: Add Vitest to the project**

Run:
```bash
npm install -D vitest @vitest/ui jsdom
```
Then add to `package.json` `scripts`:
```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 2: Create `vitest.config.ts`**

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
  },
});
```

- [ ] **Step 3: Write the failing test for clock formatting**

Create `src/lib/stores/timer.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { formatClockTime, formatClockDate } from "./timer";

describe("clock formatting", () => {
  it("formats time as HH:mm:ss with zero padding", () => {
    const d = new Date(2026, 5, 3, 9, 7, 4); // 2026-06-03 09:07:04
    expect(formatClockTime(d)).toBe("09:07:04");
  });

  it("formats date as 'Weekday, DD Month YYYY'", () => {
    const d = new Date(2026, 5, 3, 9, 7, 4); // Wednesday, 03 June 2026
    expect(formatClockDate(d)).toBe("Wednesday, 03 June 2026");
  });
});
```

- [ ] **Step 4: Run the test to verify it fails**

Run:
```bash
npm run test
```
Expected: FAIL — `formatClockTime`/`formatClockDate` are not exported from `./timer` (module not found / no export).

- [ ] **Step 5: Create the timer store with clock formatting + a clock store**

Create `src/lib/stores/timer.ts`:
```ts
import { readable } from "svelte/store";

const MONTHS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];
const WEEKDAYS = [
  "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
];

const pad2 = (n: number) => n.toString().padStart(2, "0");

export function formatClockTime(d: Date): string {
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
}

export function formatClockDate(d: Date): string {
  return `${WEEKDAYS[d.getDay()]}, ${pad2(d.getDate())} ${MONTHS[d.getMonth()]} ${d.getFullYear()}`;
}

export interface ClockState {
  time: string;
  date: string;
}

// Ticks every second; first value is emitted immediately.
export const clock = readable<ClockState>(
  { time: formatClockTime(new Date()), date: formatClockDate(new Date()) },
  (set) => {
    const tick = () => {
      const now = new Date();
      set({ time: formatClockTime(now), date: formatClockDate(now) });
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }
);
```

- [ ] **Step 6: Run the test to verify it passes**

Run:
```bash
npm run test
```
Expected: PASS (2 passing).

- [ ] **Step 7: Create the Clock component**

Create `src/lib/components/Clock.svelte`:
```svelte
<script lang="ts">
  import { clock } from "../stores/timer";
  let expanded = $state(true);
</script>

<section class="clock">
  <div class="header">
    <span class="section-header">CLOCK</span>
    <button class="icon" onclick={() => (expanded = !expanded)}>{expanded ? "▾" : "▸"}</button>
  </div>
  {#if expanded}
    <div class="content">
      <div class="time">{$clock.time}</div>
      <div class="date">{$clock.date}</div>
    </div>
  {/if}
</section>

<style>
  .clock { margin-bottom: 8px; }
  .header { display: flex; align-items: center; justify-content: space-between; }
  .time { font-size: 38px; font-weight: 300; color: var(--text); line-height: 1.05; }
  .date { font-size: 12px; color: var(--subtext); margin-top: 2px; }
</style>
```

- [ ] **Step 8: Mount Clock in App.svelte**

In `src/App.svelte`, add the import and replace the CLOCK slot:
```svelte
<script lang="ts">
  import Clock from "./lib/components/Clock.svelte";
</script>
```
Replace `<section class="slot"><span class="section-header">CLOCK</span></section>` with:
```svelte
  <Clock />
```

- [ ] **Step 9: Run and verify the live clock**

Run:
```bash
npm run tauri dev
```
Expected: the CLOCK section shows the current time updating every second (`HH:mm:ss`) and today's date. The `▾` toggle collapses/expands the clock body. Close to stop.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat: live clock store, component, and tests"
```

---

## Task 4: Pomodoro timer (state machine) + component + chime + notification

**Files:**
- Modify: `src/lib/stores/timer.ts` (add pomodoro store)
- Modify: `src/lib/stores/timer.test.ts` (add state-machine tests)
- Create: `src/lib/sound.ts` (Web Audio chime)
- Create: `src/lib/notify.ts` (notification wrapper)
- Create: `src/lib/components/Pomodoro.svelte`
- Modify: `src/App.svelte` (mount Pomodoro, wire session-complete to chime + notification)
- Modify: `package.json` (add `@tauri-apps/plugin-notification`)
- Modify: `src-tauri/Cargo.toml` + `src-tauri/src/lib.rs` (register notification plugin)
- Modify: `src-tauri/capabilities/default.json` (notification permission)

- [ ] **Step 1: Write failing tests for the Pomodoro state machine**

Append to `src/lib/stores/timer.test.ts`:
```ts
import { get } from "svelte/store";
import { createPomodoro } from "./timer";

describe("pomodoro state machine", () => {
  const cfg = { focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 };

  it("starts Idle showing focus duration", () => {
    const p = createPomodoro(cfg);
    const s = get(p.state);
    expect(s.label).toBe("Ready");
    expect(s.remainingSeconds).toBe(25 * 60);
    expect(s.isRunning).toBe(false);
  });

  it("Start moves Idle -> Focus and runs", () => {
    const p = createPomodoro(cfg);
    p.start();
    const s = get(p.state);
    expect(s.label).toBe("Focus");
    expect(s.isRunning).toBe(true);
  });

  it("focus completion 1-3 goes to Short Break, increments dots, stops, fires focus event", () => {
    const p = createPomodoro(cfg);
    const events: Array<{ minutes: number; wasFocus: boolean }> = [];
    p.onSessionComplete((minutes, wasFocus) => events.push({ minutes, wasFocus }));
    p.start();
    p._completeForTest(); // simulate countdown reaching 0
    const s = get(p.state);
    expect(s.label).toBe("Short Break");
    expect(s.completedSessions).toBe(1);
    expect(s.isRunning).toBe(false);
    expect(events).toEqual([{ minutes: 25, wasFocus: true }]);
  });

  it("4th focus completion goes to Long Break and resets dots to 0", () => {
    const p = createPomodoro(cfg);
    // 3 full focus+break cycles, then a 4th focus
    for (let i = 0; i < 3; i++) {
      p.start(); p._completeForTest(); // focus -> short break
      p.start(); p._completeForTest(); // short break -> focus
    }
    p.start(); p._completeForTest(); // 4th focus
    const s = get(p.state);
    expect(s.label).toBe("Long Break");
    expect(s.completedSessions).toBe(0);
  });

  it("break completion fires a non-focus event and returns to Focus", () => {
    const p = createPomodoro(cfg);
    const events: Array<{ minutes: number; wasFocus: boolean }> = [];
    p.onSessionComplete((minutes, wasFocus) => events.push({ minutes, wasFocus }));
    p.start(); p._completeForTest();   // focus -> short break (event focus)
    p.start(); p._completeForTest();   // short break -> focus (event break)
    expect(events[1]).toEqual({ minutes: 5, wasFocus: false });
    expect(get(p.state).label).toBe("Focus");
  });

  it("applyConfig validates positive integers and updates remaining when idle", () => {
    const p = createPomodoro(cfg);
    expect(p.applyConfig("30", "5", "15")).toBe(true);
    expect(get(p.state).remainingSeconds).toBe(30 * 60);
    expect(p.applyConfig("0", "5", "15")).toBe(false);
    expect(p.applyConfig("abc", "5", "15")).toBe(false);
  });
});
```

- [ ] **Step 2: Run tests to verify the new ones fail**

Run:
```bash
npm run test
```
Expected: FAIL — `createPomodoro` not exported.

- [ ] **Step 3: Implement the Pomodoro store**

Append to `src/lib/stores/timer.ts`:
```ts
import { writable, type Writable } from "svelte/store";
import type { PomodoroConfig } from "../types";

export type PomodoroLabel = "Ready" | "Focus" | "Short Break" | "Long Break";

export interface PomodoroState {
  label: PomodoroLabel;
  remainingSeconds: number;
  completedSessions: number; // 0..4, drives the 4 dots
  isRunning: boolean;
  timeDisplay: string; // mm:ss
}

const SESSIONS_BEFORE_LONG_BREAK = 4;
type InternalState = "Idle" | "Focus" | "ShortBreak" | "LongBreak";
type SessionListener = (minutes: number, wasFocus: boolean) => void;

const mmss = (totalSeconds: number) => {
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  return `${pad2(m)}:${pad2(s)}`;
};

const labelFor = (st: InternalState): PomodoroLabel => {
  switch (st) {
    case "Focus": return "Focus";
    case "ShortBreak": return "Short Break";
    case "LongBreak": return "Long Break";
    default: return "Ready";
  }
};

export function createPomodoro(initial: PomodoroConfig) {
  let focusMinutes = Math.max(1, initial.focus_minutes);
  let shortBreakMinutes = Math.max(1, initial.short_break_minutes);
  let longBreakMinutes = Math.max(1, initial.long_break_minutes);

  let st: InternalState = "Idle";
  let remaining = focusMinutes * 60;
  let completed = 0;
  let running = false;
  let activeFocusMinutes = focusMinutes;
  let intervalId: ReturnType<typeof setInterval> | null = null;

  const listeners: SessionListener[] = [];
  const state: Writable<PomodoroState> = writable(snapshot());

  function snapshot(): PomodoroState {
    return {
      label: labelFor(st),
      remainingSeconds: remaining,
      completedSessions: completed,
      isRunning: running,
      timeDisplay: mmss(remaining),
    };
  }
  function publish() { state.set(snapshot()); }

  function durationFor(s: InternalState): number {
    switch (s) {
      case "Focus": return focusMinutes * 60;
      case "ShortBreak": return shortBreakMinutes * 60;
      case "LongBreak": return longBreakMinutes * 60;
      default: return focusMinutes * 60;
    }
  }

  function stopInterval() {
    if (intervalId !== null) { clearInterval(intervalId); intervalId = null; }
  }

  function transitionTo(next: InternalState) {
    st = next;
    remaining = durationFor(next);
    if (next === "Focus") activeFocusMinutes = focusMinutes;
    running = false; // timer stops at each transition (parity: user presses Start)
    stopInterval();
    publish();
  }

  function handleSessionComplete() {
    stopInterval();
    running = false;
    if (st === "Focus") {
      const newCount = completed + 1;
      if (newCount >= SESSIONS_BEFORE_LONG_BREAK) {
        completed = 0;
        emit(activeFocusMinutes, true);
        transitionTo("LongBreak");
      } else {
        completed = newCount;
        emit(activeFocusMinutes, true);
        transitionTo("ShortBreak");
      }
    } else {
      const breakMinutes = st === "ShortBreak" ? shortBreakMinutes : longBreakMinutes;
      emit(breakMinutes, false);
      transitionTo("Focus");
    }
  }

  function emit(minutes: number, wasFocus: boolean) {
    for (const l of listeners) l(minutes, wasFocus);
  }

  function tick() {
    remaining -= 1;
    if (remaining <= 0) { handleSessionComplete(); return; }
    publish();
  }

  function start() {
    if (st === "Idle") {
      st = "Focus";
      remaining = focusMinutes * 60;
      activeFocusMinutes = focusMinutes;
    }
    if (running) return;
    running = true;
    stopInterval();
    intervalId = setInterval(tick, 1000);
    publish();
  }

  function pause() {
    stopInterval();
    running = false;
    publish();
  }

  function reset() {
    stopInterval();
    running = false;
    remaining = durationFor(st === "Idle" ? "Focus" : st);
    publish();
  }

  function applyConfig(focusText: string, shortText: string, longText: string): boolean {
    const f = Number(focusText), s = Number(shortText), l = Number(longText);
    const valid = (n: number) => Number.isInteger(n) && n > 0;
    if (!valid(f) || !valid(s) || !valid(l)) return false;
    focusMinutes = f; shortBreakMinutes = s; longBreakMinutes = l;
    if (!running) {
      remaining = durationFor(st === "Idle" ? "Focus" : st);
      if (st === "Focus" || st === "Idle") activeFocusMinutes = focusMinutes;
      publish();
    }
    return true;
  }

  function getConfig(): PomodoroConfig {
    return {
      focus_minutes: focusMinutes,
      short_break_minutes: shortBreakMinutes,
      long_break_minutes: longBreakMinutes,
    };
  }

  function onSessionComplete(cb: SessionListener) { listeners.push(cb); }

  function dispose() { stopInterval(); }

  return {
    state: { subscribe: state.subscribe },
    start, pause, reset, applyConfig, getConfig, onSessionComplete, dispose,
    // test-only hook to simulate the countdown hitting zero without waiting:
    _completeForTest: () => { handleSessionComplete(); },
  };
}

export type Pomodoro = ReturnType<typeof createPomodoro>;
```

(Note: `src/lib/types.ts` is created in Task 5. Until then, add a temporary local interface OR create `types.ts` now with just `PomodoroConfig`. Create it now — see Step 4.)

- [ ] **Step 4: Create the types module with `PomodoroConfig` (full types added in Task 5)**

Create `src/lib/types.ts`:
```ts
export interface PomodoroConfig {
  focus_minutes: number;
  short_break_minutes: number;
  long_break_minutes: number;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run:
```bash
npm run test
```
Expected: PASS (all clock + pomodoro tests green).

- [ ] **Step 6: Create the Web Audio chime**

Create `src/lib/sound.ts`:
```ts
// Generates a short two-note chime via Web Audio — no binary asset required.
// To swap in a real timer-end.wav later, replace the body with an <audio> play.
let ctx: AudioContext | null = null;

function context(): AudioContext {
  if (!ctx) ctx = new AudioContext();
  return ctx;
}

function beep(ac: AudioContext, freq: number, startAt: number, duration: number) {
  const osc = ac.createOscillator();
  const gain = ac.createGain();
  osc.type = "sine";
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0.0001, startAt);
  gain.gain.exponentialRampToValueAtTime(0.25, startAt + 0.02);
  gain.gain.exponentialRampToValueAtTime(0.0001, startAt + duration);
  osc.connect(gain).connect(ac.destination);
  osc.start(startAt);
  osc.stop(startAt + duration);
}

export function playChime() {
  const ac = context();
  if (ac.state === "suspended") ac.resume();
  const t = ac.currentTime;
  beep(ac, 880, t, 0.18);
  beep(ac, 1175, t + 0.18, 0.22);
}
```

- [ ] **Step 7: Install + register the notification plugin**

Run:
```bash
npm install @tauri-apps/plugin-notification
cd src-tauri && cargo add tauri-plugin-notification && cd ..
```
In `src-tauri/src/lib.rs`, register the plugin in the builder chain (inside the existing `tauri::Builder::default()...`):
```rust
.plugin(tauri_plugin_notification::init())
```
In `src-tauri/capabilities/default.json`, add to `"permissions"`:
```json
"notification:default"
```

- [ ] **Step 8: Create the notification wrapper**

Create `src/lib/notify.ts`:
```ts
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

export async function ensureNotificationPermission(): Promise<boolean> {
  let granted = await isPermissionGranted();
  if (!granted) {
    const res = await requestPermission();
    granted = res === "granted";
  }
  return granted;
}

export async function notify(title: string, body: string) {
  if (await ensureNotificationPermission()) {
    sendNotification({ title, body });
  }
}
```

- [ ] **Step 9: Create the Pomodoro component**

Create `src/lib/components/Pomodoro.svelte`:
```svelte
<script lang="ts">
  import type { Pomodoro } from "../stores/timer";

  let { pomodoro }: { pomodoro: Pomodoro } = $props();
  const state = pomodoro.state;

  let expanded = $state(true);
  let configOpen = $state(false);
  let focusText = $state("25");
  let shortText = $state("5");
  let longText = $state("15");
  let configError = $state("");

  const labelColor = (label: string) =>
    label === "Focus" ? "var(--accent)"
    : label === "Short Break" ? "var(--green)"
    : label === "Long Break" ? "var(--yellow)"
    : "var(--subtext)";

  function openConfig() {
    const c = pomodoro.getConfig();
    focusText = String(c.focus_minutes);
    shortText = String(c.short_break_minutes);
    longText = String(c.long_break_minutes);
    configError = "";
    configOpen = true;
  }

  function adjust(which: "focus" | "short" | "long", delta: number) {
    const clamp = (v: string) => {
      const n = Number(v);
      const base = Number.isInteger(n) ? n : 1;
      return String(Math.min(180, Math.max(1, base + delta)));
    };
    if (which === "focus") focusText = clamp(focusText);
    else if (which === "short") shortText = clamp(shortText);
    else longText = clamp(longText);
  }

  function apply() {
    const ok = pomodoro.applyConfig(focusText, shortText, longText);
    if (!ok) { configError = "All fields must be a positive number."; return; }
    configError = "";
    configOpen = false;
    onConfigSaved?.(pomodoro.getConfig());
  }

  // App.svelte injects this to persist config to Rust.
  let { onConfigSaved }: { onConfigSaved?: (c: any) => void } = $props();
</script>

<section class="pomo">
  <div class="header">
    <div class="header-left">
      <span class="section-header">POMODORO</span>
      <span class="dots">
        {#each [1, 2, 3, 4] as n}
          <span class="dot" class:on={$state.completedSessions >= n}></span>
        {/each}
      </span>
    </div>
    <div class="header-actions">
      <button class="icon" title="Timer settings" onclick={openConfig}>⚙</button>
      <button class="icon" onclick={() => (expanded = !expanded)}>{expanded ? "▾" : "▸"}</button>
    </div>
  </div>

  {#if expanded}
    <div class="content">
      <div class="state-label" style="color: {labelColor($state.label)}">{$state.label}</div>
      <div class="countdown">{$state.timeDisplay}</div>

      <div class="controls">
        {#if !$state.isRunning}
          <button onclick={() => pomodoro.start()}>Start</button>
        {:else}
          <button class="secondary" onclick={() => pomodoro.pause()}>Pause</button>
        {/if}
        <button class="secondary" onclick={() => pomodoro.reset()}>Reset</button>
      </div>

      {#if configOpen}
        <div class="config">
          {#each [["Focus", "focus", focusText], ["Short Break", "short", shortText], ["Long Break", "long", longText]] as row}
            <div class="config-row">
              <span class="config-label">{row[0]}</span>
              <button class="secondary tiny" onclick={() => adjust(row[1] as any, -1)}>−</button>
              <input
                class="num"
                value={row[1] === "focus" ? focusText : row[1] === "short" ? shortText : longText}
                oninput={(e) => {
                  const v = (e.target as HTMLInputElement).value;
                  if (row[1] === "focus") focusText = v;
                  else if (row[1] === "short") shortText = v;
                  else longText = v;
                }}
              />
              <button class="secondary tiny" onclick={() => adjust(row[1] as any, 1)}>+</button>
              <span class="unit">min</span>
            </div>
          {/each}
          {#if configError}<div class="config-error">{configError}</div>{/if}
          <div class="config-actions">
            <button class="secondary" onclick={() => (configOpen = false)}>Cancel</button>
            <button onclick={apply}>Apply</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .pomo { margin-bottom: 8px; }
  .header { display: flex; align-items: center; justify-content: space-between; }
  .header-left { display: flex; align-items: center; gap: 8px; }
  .header-actions { display: flex; align-items: center; gap: 4px; }
  .dots { display: flex; gap: 4px; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--surface); }
  .dot.on { background: var(--accent); }
  .state-label { text-align: center; font-size: 12px; margin-top: 4px; }
  .countdown { text-align: center; font-size: 52px; font-weight: 300; color: var(--accent); margin: 4px 0 8px; }
  .controls { display: flex; justify-content: center; gap: 8px; }
  .config { margin-top: 10px; background: var(--surface); border-radius: 8px; padding: 10px; }
  .config-row { display: grid; grid-template-columns: 1fr auto 44px auto auto; align-items: center; gap: 4px; margin-bottom: 6px; }
  .config-label { font-size: 12px; color: var(--text); }
  .num { text-align: center; padding: 4px; font-size: 12px; }
  .tiny { padding: 3px 7px; font-size: 13px; }
  .unit { font-size: 11px; color: var(--subtext); margin-left: 6px; }
  .config-error { color: var(--red); font-size: 11px; margin-bottom: 6px; }
  .config-actions { display: flex; justify-content: flex-end; gap: 6px; }
</style>
```

- [ ] **Step 10: Wire Pomodoro into App.svelte (chime + notification on session end)**

In `src/App.svelte` `<script>`:
```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Clock from "./lib/components/Clock.svelte";
  import Pomodoro from "./lib/components/Pomodoro.svelte";
  import { createPomodoro } from "./lib/stores/timer";
  import { playChime } from "./lib/sound";
  import { notify } from "./lib/notify";

  // Config persistence comes online in Task 5; default for now.
  const pomodoro = createPomodoro({ focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 });

  pomodoro.onSessionComplete((_minutes, wasFocus) => {
    playChime();
    if (wasFocus) notify("Focus Complete", "Time for a break!");
    else notify("Break Over", "Back to focus!");
  });

  onDestroy(() => pomodoro.dispose());
</script>
```
Replace the POMODORO slot with:
```svelte
  <Pomodoro {pomodoro} />
```
(Leave `onConfigSaved` unset for now; it's wired in Task 5.)

- [ ] **Step 11: Run and verify the Pomodoro UI + behavior**

Run:
```bash
npm run tauri dev
```
Verify manually:
- Countdown shows `25:00`, label "Ready". Press Start → label "Focus", counts down, dots empty.
- Press Pause → stops; Start resumes. Reset → back to `25:00`.
- Open `⚙`, change Focus to `1`, Apply → display shows `01:00`. (For a fast end-to-end check, set Focus to 1 min, Start, wait → on completion you hear the chime, a macOS notification "Focus Complete" appears (grant permission on first prompt), the timer stops at "Short Break" `05:00`, and dot 1 fills.)
- Close to stop.

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "feat: pomodoro state machine store, component, chime, notification"
```

---

## Task 5: Rust models + storage module + config commands

**Files:**
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/storage.rs`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Create: `src/lib/api.ts`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/stores/settings.ts` is NOT here (Task 6); config wiring touches `App.svelte` + `Pomodoro.svelte`

- [ ] **Step 1: Add Rust dependencies**

Run:
```bash
cd src-tauri
cargo add serde --features derive
cargo add serde_json
cargo add uuid --features v4
cargo add chrono --features clock
cargo add reqwest --features json,rustls-tls --no-default-features
cargo add dotenvy
cd ..
```

- [ ] **Step 2: Write a failing Rust test for storage round-trip + defaults**

Create `src-tauri/src/storage.rs` with the test module first (implementation in next step):
```rust
use std::fs;
use std::path::{Path, PathBuf};
use serde::{de::DeserializeOwned, Serialize};

/// Reads JSON `<dir>/<file>`. Returns `T::default()` if the file is missing or unparsable.
pub fn read_json<T: DeserializeOwned + Default>(dir: &Path, file: &str) -> T {
    let path = dir.join(file);
    let Ok(text) = fs::read_to_string(&path) else { return T::default() };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Writes `value` as pretty JSON to `<dir>/<file>`, creating `dir` if needed.
pub fn write_json<T: Serialize>(dir: &Path, file: &str, value: &T) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(dir.join(file), text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FoTask, PomodoroConfig};

    fn temp_dir(tag: &str) -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("fopomoro_test_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn missing_file_returns_default_config() {
        let dir = temp_dir("cfg");
        let cfg: PomodoroConfig = read_json(&dir, "pomodoro_config.json");
        assert_eq!(cfg.focus_minutes, 25);
        assert_eq!(cfg.short_break_minutes, 5);
        assert_eq!(cfg.long_break_minutes, 15);
    }

    #[test]
    fn tasks_round_trip() {
        let dir = temp_dir("tasks");
        let tasks = vec![FoTask {
            id: "abc".into(),
            task_id: "FO-01".into(),
            title: "Read".into(),
            is_completed: false,
            created_at: "2026-06-03T09:00:00+00:00".into(),
            completed_at: None,
            pomodoro_count: 2,
        }];
        write_json(&dir, "tasks.json", &tasks).unwrap();
        let loaded: Vec<FoTask> = read_json(&dir, "tasks.json");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].task_id, "FO-01");
        assert_eq!(loaded[0].pomodoro_count, 2);
        let _ = fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 3: Create the models module**

Create `src-tauri/src/models.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FoTask {
    pub id: String,                  // UUID string
    pub task_id: String,             // display id, e.g. "FO-01"
    pub title: String,
    pub is_completed: bool,
    pub created_at: String,          // RFC3339
    pub completed_at: Option<String>,
    pub pomodoro_count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FoSession {
    pub date: String,                // "YYYY-MM-DD"
    pub focus_sessions_count: i32,
    pub total_minutes_studied: i32,
    pub tasks_completed_count: i32,
}

impl Default for FoSession {
    fn default() -> Self {
        Self {
            date: crate::commands::today_string(),
            focus_sessions_count: 0,
            total_minutes_studied: 0,
            tasks_completed_count: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PomodoroConfig {
    pub focus_minutes: i32,
    pub short_break_minutes: i32,
    pub long_break_minutes: i32,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self { focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowSettings {
    pub opacity: f64,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self { opacity: 0.9 }
    }
}
```

- [ ] **Step 4: Create the commands module (config commands + helpers now; tasks/sessions extended in Tasks 6–8)**

Create `src-tauri/src/commands.rs`:
```rust
use crate::models::{PomodoroConfig, WindowSettings};
use crate::storage;
use crate::AppState;
use chrono::Local;
use tauri::State;

pub fn today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn now_rfc3339() -> String {
    Local::now().to_rfc3339()
}

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> PomodoroConfig {
    storage::read_json(&state.data_dir, "pomodoro_config.json")
}

#[tauri::command]
pub fn save_config(config: PomodoroConfig, state: State<'_, AppState>) -> Result<(), String> {
    storage::write_json(&state.data_dir, "pomodoro_config.json", &config)
}

#[tauri::command]
pub fn load_settings(state: State<'_, AppState>) -> WindowSettings {
    storage::read_json(&state.data_dir, "settings.json")
}

#[tauri::command]
pub fn save_settings(settings: WindowSettings, state: State<'_, AppState>) -> Result<(), String> {
    storage::write_json(&state.data_dir, "settings.json", &settings)
}

#[tauri::command]
pub fn set_click_through(enabled: bool, window: tauri::WebviewWindow) -> Result<(), String> {
    window.set_ignore_cursor_events(enabled).map_err(|e| e.to_string())
}
```

- [ ] **Step 5: Replace `lib.rs` to define `AppState`, resolve the data dir, and register modules + commands**

Replace `src-tauri/src/lib.rs` with:
```rust
mod models;
mod storage;
mod supabase;
mod commands;

use std::path::PathBuf;
use tauri::Manager;

pub struct AppState {
    pub data_dir: PathBuf,
    pub supabase: Option<supabase::SupabaseConfig>,
    pub http: reqwest::Client,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the project root / cwd (dev-local). No-op if absent.
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            let supabase = supabase::SupabaseConfig::from_env();
            if supabase.is_none() {
                eprintln!("[supabase] .env not found or incomplete — running offline.");
            }

            app.manage(AppState {
                data_dir,
                supabase,
                http: reqwest::Client::new(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::load_settings,
            commands::save_settings,
            commands::set_click_through,
            commands::get_tasks,
            commands::insert_task,
            commands::update_task,
            commands::delete_task,
            commands::record_session,
            commands::load_progress,
            commands::save_progress,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```
(The task/session/progress commands referenced here are added in Tasks 6–8. To compile Task 5 in isolation, temporarily remove the not-yet-created handler names AND create a stub `supabase.rs` — see Step 6. They are restored as those tasks land.)

- [ ] **Step 6: Create a minimal `supabase.rs` stub (full implementation in Task 7)**

Create `src-tauri/src/supabase.rs`:
```rust
use std::env;

#[derive(Clone, Debug)]
pub struct SupabaseConfig {
    pub base_url: String,
    pub key: String,
}

impl SupabaseConfig {
    pub fn from_env() -> Option<Self> {
        let url = env::var("SUPABASE_URL").ok().filter(|s| !s.is_empty())?;
        let key = env::var("SUPABASE_ANON_KEY").ok().filter(|s| !s.is_empty())?;
        Some(Self {
            base_url: format!("{}/rest/v1", url.trim_end_matches('/')),
            key,
        })
    }
}
```
And temporarily trim the `invoke_handler!` list in `lib.rs` to only the commands that exist now:
```rust
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::load_settings,
            commands::save_settings,
            commands::set_click_through,
        ])
```

- [ ] **Step 7: Run the Rust tests**

Run:
```bash
cd src-tauri && cargo test && cd ..
```
Expected: PASS — `missing_file_returns_default_config`, `tasks_round_trip` (and the crate compiles).

- [ ] **Step 8: Define the full TS types mirroring the Rust models**

Replace `src/lib/types.ts`:
```ts
export interface FoTask {
  id: string;
  task_id: string;
  title: string;
  is_completed: boolean;
  created_at: string;
  completed_at: string | null;
  pomodoro_count: number;
}

export interface FoSession {
  date: string;
  focus_sessions_count: number;
  total_minutes_studied: number;
  tasks_completed_count: number;
}

export interface PomodoroConfig {
  focus_minutes: number;
  short_break_minutes: number;
  long_break_minutes: number;
}

export interface WindowSettings {
  opacity: number;
}
```

- [ ] **Step 9: Create the api wrapper (config + settings + click-through now; rest filled in later tasks)**

Create `src/lib/api.ts`:
```ts
import { invoke } from "@tauri-apps/api/core";
import type { FoTask, FoSession, PomodoroConfig, WindowSettings } from "./types";

export const api = {
  loadConfig: () => invoke<PomodoroConfig>("load_config"),
  saveConfig: (config: PomodoroConfig) => invoke<void>("save_config", { config }),

  loadSettings: () => invoke<WindowSettings>("load_settings"),
  saveSettings: (settings: WindowSettings) => invoke<void>("save_settings", { settings }),

  setClickThrough: (enabled: boolean) => invoke<void>("set_click_through", { enabled }),

  // Implemented in Tasks 6–8:
  getTasks: () => invoke<FoTask[]>("get_tasks"),
  insertTask: (title: string) => invoke<FoTask>("insert_task", { title }),
  updateTask: (task: FoTask) => invoke<void>("update_task", { task }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
  recordSession: (taskId: string | null, durationMinutes: number, wasFocused: boolean) =>
    invoke<void>("record_session", { taskId, durationMinutes, wasFocused }),
  loadProgress: () => invoke<FoSession>("load_progress"),
  saveProgress: (session: FoSession) => invoke<void>("save_progress", { session }),
};
```

- [ ] **Step 10: Persist Pomodoro config through Rust**

In `src/App.svelte` `<script>`, load config on mount and re-create the store; wire `onConfigSaved`:
```svelte
  import { onMount } from "svelte";
  import { api } from "./lib/api";

  let pomodoro = $state(createPomodoro({ focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 }));

  function wireSessionComplete() {
    pomodoro.onSessionComplete((_minutes, wasFocus) => {
      playChime();
      if (wasFocus) notify("Focus Complete", "Time for a break!");
      else notify("Break Over", "Back to focus!");
    });
  }
  wireSessionComplete();

  onMount(async () => {
    const cfg = await api.loadConfig();
    pomodoro.dispose();
    pomodoro = createPomodoro(cfg);
    wireSessionComplete();
  });

  function persistConfig(c: PomodoroConfig) { api.saveConfig(c); }
```
And pass to the component:
```svelte
  <Pomodoro {pomodoro} onConfigSaved={persistConfig} />
```
Add `import type { PomodoroConfig } from "./lib/types";` to the imports.

- [ ] **Step 11: Run and verify config persistence**

Run:
```bash
npm run tauri dev
```
Verify: open `⚙`, set Focus to `30`, Apply. Close the app, reopen (`npm run tauri dev` again) — the timer now shows `30:00`, confirming config persisted to `~/Library/Application Support/com.fopomoro.mac/pomodoro_config.json`.

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "feat: Rust models, storage, config/settings commands, TS types + api"
```

---

## Task 6: Task list — local storage commands + tasks store + TaskList component

**Files:**
- Modify: `src-tauri/src/commands.rs` (get_tasks, insert_task, update_task, delete_task — LOCAL only this task)
- Modify: `src-tauri/src/lib.rs` (restore task command handlers)
- Create: `src-tauri/src/commands_test_helpers` — none; add a unit test for `next_task_number`
- Create: `src/lib/stores/tasks.ts`
- Create: `src/lib/stores/tasks.test.ts`
- Create: `src/lib/components/TaskList.svelte`
- Modify: `src/App.svelte` (mount TaskList, wire timer.isRunning → tasks)

- [ ] **Step 1: Add `next_task_number` with a Rust unit test (failing first)**

Append to `src-tauri/src/commands.rs`:
```rust
use crate::models::FoTask;

/// Next FO-NN number = max existing FO-NN + 1 (1 if none).
pub fn next_task_number(tasks: &[FoTask]) -> i32 {
    let mut max = 0;
    for t in tasks {
        if let Some(rest) = t.task_id.strip_prefix("FO-") {
            if let Ok(n) = rest.parse::<i32>() {
                if n > max { max = n; }
            }
        }
    }
    max + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(task_id: &str) -> FoTask {
        FoTask {
            id: "x".into(), task_id: task_id.into(), title: "t".into(),
            is_completed: false, created_at: "".into(), completed_at: None, pomodoro_count: 0,
        }
    }

    #[test]
    fn next_number_empty_is_one() {
        assert_eq!(next_task_number(&[]), 1);
    }

    #[test]
    fn next_number_is_max_plus_one() {
        let tasks = vec![task("FO-01"), task("FO-07"), task("FO-03"), task("weird")];
        assert_eq!(next_task_number(&tasks), 8);
    }
}
```

- [ ] **Step 2: Run the Rust test to verify it fails, then passes after adding the fn**

Run:
```bash
cd src-tauri && cargo test next_number && cd ..
```
Expected: after adding the function above, PASS (`next_number_empty_is_one`, `next_number_is_max_plus_one`).

- [ ] **Step 3: Add the local task commands (Supabase integration deferred to Task 7)**

Append to `src-tauri/src/commands.rs`:
```rust
use uuid::Uuid;

#[tauri::command]
pub fn get_tasks(state: State<'_, AppState>) -> Vec<FoTask> {
    // Task 7 will prefer Supabase when available; for now, local cache.
    storage::read_json(&state.data_dir, "tasks.json")
}

#[tauri::command]
pub fn insert_task(title: String, state: State<'_, AppState>) -> Result<FoTask, String> {
    let mut tasks: Vec<FoTask> = storage::read_json(&state.data_dir, "tasks.json");
    let n = next_task_number(&tasks);
    let task = FoTask {
        id: Uuid::new_v4().to_string(),
        task_id: format!("FO-{:02}", n),
        title: title.trim().to_string(),
        is_completed: false,
        created_at: now_rfc3339(),
        completed_at: None,
        pomodoro_count: 0,
    };
    tasks.push(task.clone());
    storage::write_json(&state.data_dir, "tasks.json", &tasks)?;
    Ok(task)
}

#[tauri::command]
pub fn update_task(task: FoTask, state: State<'_, AppState>) -> Result<(), String> {
    let mut tasks: Vec<FoTask> = storage::read_json(&state.data_dir, "tasks.json");
    if let Some(slot) = tasks.iter_mut().find(|t| t.id == task.id) {
        *slot = task;
    }
    storage::write_json(&state.data_dir, "tasks.json", &tasks)
}

#[tauri::command]
pub fn delete_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut tasks: Vec<FoTask> = storage::read_json(&state.data_dir, "tasks.json");
    tasks.retain(|t| t.id != id);
    storage::write_json(&state.data_dir, "tasks.json", &tasks)
}
```

- [ ] **Step 4: Restore the task command handlers in `lib.rs`**

In `src-tauri/src/lib.rs`, set the `invoke_handler!` list to:
```rust
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::load_settings,
            commands::save_settings,
            commands::set_click_through,
            commands::get_tasks,
            commands::insert_task,
            commands::update_task,
            commands::delete_task,
        ])
```

- [ ] **Step 5: Write failing tests for the tasks store logic (number gen, active toggle, switch quirk)**

The store talks to Rust via `api`, so tests mock `api`. Create `src/lib/stores/tasks.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";
import type { FoTask } from "../types";

// Mock the api module before importing the store.
const inserted: FoTask[] = [];
vi.mock("../api", () => ({
  api: {
    getTasks: vi.fn(async () => []),
    insertTask: vi.fn(async (title: string) => {
      const t: FoTask = {
        id: `id-${inserted.length + 1}`,
        task_id: `FO-0${inserted.length + 1}`,
        title,
        is_completed: false,
        created_at: "2026-06-03T00:00:00+00:00",
        completed_at: null,
        pomodoro_count: 0,
      };
      inserted.push(t);
      return t;
    }),
    updateTask: vi.fn(async () => {}),
    deleteTask: vi.fn(async () => {}),
    recordSession: vi.fn(async () => {}),
  },
}));

import { createTasksStore } from "./tasks";
import { api } from "../api";

beforeEach(() => {
  inserted.length = 0;
  vi.clearAllMocks();
});

describe("tasks store", () => {
  it("adds a task and clears the input", async () => {
    const s = createTasksStore();
    await s.add("Read chapter 3");
    const tasks = get(s.tasks);
    expect(tasks).toHaveLength(1);
    expect(tasks[0].title).toBe("Read chapter 3");
    expect(get(s.newTaskTitle)).toBe("");
  });

  it("blocks toggling a non-active task while the timer runs", async () => {
    const s = createTasksStore();
    await s.add("A");
    await s.add("B");
    const [a, b] = get(s.tasks);
    s.setActive(a);
    s.setTimerRunning(true);
    await s.toggle(b); // should be ignored
    expect(get(s.tasks).find((t) => t.id === b.id)!.is_completed).toBe(false);
    await s.toggle(a); // active task can toggle
    expect(get(s.tasks).find((t) => t.id === a.id)!.is_completed).toBe(true);
  });

  it("blocks deleting any task while the timer runs", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setTimerRunning(true);
    await s.remove(a);
    expect(get(s.tasks)).toHaveLength(1);
  });

  it("setActive toggles off when the same task is clicked again", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setActive(a);
    expect(get(s.activeTaskId)).toBe(a.id);
    s.setActive(a);
    expect(get(s.activeTaskId)).toBe(null);
  });

  it("focus completion increments the active task pomodoro and records with task_id", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setActive(a);
    await s.onFocusSessionCompleted(25);
    expect(get(s.tasks)[0].pomodoro_count).toBe(1);
    expect(api.recordSession).toHaveBeenCalledWith(a.id, 25, true);
  });

  it("switching tasks mid-session records null/false and does NOT increment", async () => {
    const s = createTasksStore();
    await s.add("A");
    await s.add("B");
    const [a, b] = get(s.tasks);
    s.setActive(a);
    s.setTimerRunning(true);
    s.setActive(b); // switch during run -> sets the quirk flag
    await s.onFocusSessionCompleted(25);
    expect(get(s.tasks)[0].pomodoro_count).toBe(0);
    expect(get(s.tasks)[1].pomodoro_count).toBe(0);
    expect(api.recordSession).toHaveBeenCalledWith(null, 25, false);
  });
});
```

- [ ] **Step 6: Run the store tests to verify they fail**

Run:
```bash
npm run test
```
Expected: FAIL — `createTasksStore` not exported.

- [ ] **Step 7: Implement the tasks store**

Create `src/lib/stores/tasks.ts`:
```ts
import { writable, derived, get } from "svelte/store";
import type { FoTask } from "../types";
import { api } from "../api";

function isToday(iso: string | null): boolean {
  if (!iso) return false;
  const d = new Date(iso);
  const now = new Date();
  return d.getFullYear() === now.getFullYear()
    && d.getMonth() === now.getMonth()
    && d.getDate() === now.getDate();
}

export function createTasksStore() {
  const tasks = writable<FoTask[]>([]);
  const newTaskTitle = writable<string>("");
  const activeTaskId = writable<string | null>(null);

  let timerRunning = false;
  let switchedDuringSession = false;

  const taskCountDisplay = derived(tasks, ($t) => {
    const done = $t.filter((x) => x.is_completed).length;
    return `${done} / ${$t.length} done`;
  });

  const todayCompletedCount = derived(tasks, ($t) =>
    $t.filter((x) => x.is_completed && isToday(x.completed_at)).length
  );

  // App.svelte registers this to update progress when a task is toggled.
  let onTaskToggled: (() => void) | null = null;

  async function load() {
    const list = await api.getTasks();
    tasks.set(list);
  }

  async function add(titleArg?: string) {
    const title = (titleArg ?? get(newTaskTitle)).trim();
    if (!title) return;
    const created = await api.insertTask(title);
    tasks.update((arr) => [...arr, created]);
    newTaskTitle.set("");
  }

  async function toggle(task: FoTask) {
    const active = get(activeTaskId);
    if (timerRunning && active !== null && active !== task.id) return;
    const updated: FoTask = {
      ...task,
      is_completed: !task.is_completed,
      completed_at: !task.is_completed ? new Date().toISOString() : null,
    };
    tasks.update((arr) => arr.map((t) => (t.id === task.id ? updated : t)));
    onTaskToggled?.();
    await api.updateTask(updated);
  }

  async function remove(task: FoTask) {
    if (timerRunning) return;
    if (get(activeTaskId) === task.id) activeTaskId.set(null);
    tasks.update((arr) => arr.filter((t) => t.id !== task.id));
    await api.deleteTask(task.id);
  }

  function setActive(task: FoTask) {
    const current = get(activeTaskId);
    if (timerRunning && current !== null && current !== task.id) {
      switchedDuringSession = true;
    }
    if (current === task.id) {
      activeTaskId.set(null);
      return;
    }
    activeTaskId.set(task.id);
  }

  function setTimerRunning(running: boolean) {
    if (!running) switchedDuringSession = false;
    timerRunning = running;
  }

  async function onFocusSessionCompleted(durationMinutes: number) {
    const switched = switchedDuringSession;
    switchedDuringSession = false;
    const activeId = get(activeTaskId);

    if (!switched && activeId !== null) {
      let updated: FoTask | null = null;
      tasks.update((arr) =>
        arr.map((t) => {
          if (t.id === activeId) {
            updated = { ...t, pomodoro_count: t.pomodoro_count + 1 };
            return updated;
          }
          return t;
        })
      );
      if (updated) await api.updateTask(updated);
    }

    const taskId = switched ? null : activeId;
    await api.recordSession(taskId, durationMinutes, !switched);
  }

  function registerTaskToggled(cb: () => void) { onTaskToggled = cb; }

  return {
    tasks: { subscribe: tasks.subscribe },
    newTaskTitle,
    activeTaskId: { subscribe: activeTaskId.subscribe },
    taskCountDisplay,
    todayCompletedCount,
    load, add, toggle, remove, setActive, setTimerRunning,
    onFocusSessionCompleted, registerTaskToggled,
  };
}

export type TasksStore = ReturnType<typeof createTasksStore>;
```

- [ ] **Step 8: Run the store tests to verify they pass**

Run:
```bash
npm run test
```
Expected: PASS (all tasks-store tests green, plus existing clock/pomodoro).

- [ ] **Step 9: Create the TaskList component**

Create `src/lib/components/TaskList.svelte`:
```svelte
<script lang="ts">
  import type { TasksStore } from "../stores/tasks";

  let { store }: { store: TasksStore } = $props();
  const { tasks, newTaskTitle, activeTaskId, taskCountDisplay } = store;

  let expanded = $state(true);
  let timerRunning = $state(false);
  // App.svelte calls setRunning() to reflect the timer for the locked-ops notice.
  export function setRunning(v: boolean) { timerRunning = v; }

  function onInputKey(e: KeyboardEvent) {
    if (e.key === "Enter") store.add();
  }
</script>

<section class="tasks">
  <div class="header">
    <span class="section-header">TASKS</span>
    <div class="header-right">
      <span class="count">{$taskCountDisplay}</span>
      <button class="icon" onclick={() => (expanded = !expanded)}>{expanded ? "▾" : "▸"}</button>
    </div>
  </div>

  {#if expanded}
    <div class="content">
      {#if timerRunning}
        <div class="notice">⏱ Timer running — delete &amp; toggling other tasks is locked</div>
      {/if}

      <div class="list">
        {#each $tasks as task (task.id)}
          <div class="row">
            <button
              class="active-dot"
              class:on={$activeTaskId === task.id}
              title="Set active task"
              onclick={() => store.setActive(task)}
              aria-label="Set active"
            ></button>
            <span class="badge">{task.task_id}</span>
            <label class="check">
              <input
                type="checkbox"
                checked={task.is_completed}
                onchange={() => store.toggle(task)}
              />
              <span class="title" class:done={task.is_completed}>{task.title}</span>
            </label>
            {#if task.pomodoro_count > 0}
              <span class="pomo-badge">🍅×{task.pomodoro_count}</span>
            {/if}
            <button class="danger del" title="Delete" onclick={() => store.remove(task)}>×</button>
          </div>
        {/each}
      </div>

      <div class="add-row">
        <input
          type="text"
          placeholder="Add a task…"
          value={$newTaskTitle}
          oninput={(e) => newTaskTitle.set((e.target as HTMLInputElement).value)}
          onkeydown={onInputKey}
        />
        <button class="add" onclick={() => store.add()}>+</button>
      </div>
    </div>
  {/if}
</section>

<style>
  .tasks { margin-bottom: 8px; }
  .header { display: flex; align-items: center; justify-content: space-between; }
  .header-right { display: flex; align-items: center; gap: 6px; }
  .count { font-size: 10px; color: var(--subtext); }
  .notice { background: var(--surface); color: var(--yellow); font-size: 10px; text-align: center; border-radius: 6px; padding: 5px 8px; margin: 6px 0 4px; }
  .list { max-height: 200px; overflow-y: auto; }
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
  .add-row { display: flex; gap: 6px; margin-top: 8px; }
  .add-row input { flex: 1; }
  .add { width: 36px; padding: 0; }
</style>
```

- [ ] **Step 10: Mount TaskList and wire timer→tasks running flag in App.svelte**

In `src/App.svelte` `<script>`:
```svelte
  import TaskList from "./lib/components/TaskList.svelte";
  import { createTasksStore } from "./lib/stores/tasks";

  const tasksStore = createTasksStore();
  let taskListRef: { setRunning: (v: boolean) => void } | undefined = $state();

  // Reflect timer running state into tasks store (locks ops) + UI notice.
  $effect(() => {
    const running = $pomodoroState.isRunning;
    tasksStore.setTimerRunning(running);
    taskListRef?.setRunning(running);
  });
```
Add near the top, after `pomodoro` is created:
```svelte
  const pomodoroState = pomodoro.state;
```
(If `pomodoro` is reassigned in `onMount`, also re-bind `pomodoroState` there: `pomodoroState = pomodoro.state;` — declare it with `let pomodoroState = $state(pomodoro.state);` and reassign after re-create.)

Load tasks on mount (extend the existing `onMount`):
```svelte
    await tasksStore.load();
```
Replace the TASKS slot:
```svelte
  <TaskList store={tasksStore} bind:this={taskListRef} />
```

- [ ] **Step 11: Run and verify the task list**

Run:
```bash
npm run tauri dev
```
Verify:
- Type a task, press Enter (or `+`) → row appears with `FO-01` badge, "0 / 1 done" updates.
- Add a second task → `FO-02`.
- Click the active dot on a task → it turns red.
- Check a task → strike-through, count updates.
- Delete with `×` → row removed.
- Start the timer → the "Timer running" notice appears; the `×` buttons and toggling non-active tasks are blocked.
- Restart the app → tasks persist (loaded from `tasks.json`).

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "feat: task list with local persistence, store, and component"
```

---

## Task 7: Supabase sync (Rust REST module) + offline fallback

**Files:**
- Modify: `src-tauri/src/supabase.rs` (full REST client)
- Modify: `src-tauri/src/commands.rs` (use Supabase in get/insert/update/delete + record_session)
- Modify: `src-tauri/src/lib.rs` (add `record_session` to handlers)
- Create: `.env.example`
- Modify: `.gitignore` (ignore `.env`)

- [ ] **Step 1: Create `.env.example` and ignore `.env`**

Create `.env.example`:
```
SUPABASE_URL=https://your-project-id.supabase.co
SUPABASE_ANON_KEY=your-anon-key-here
```
Append to `.gitignore`:
```
.env
```

- [ ] **Step 2: Implement the full Supabase REST client**

Replace `src-tauri/src/supabase.rs`:
```rust
use std::env;
use serde::Deserialize;
use crate::models::FoTask;

#[derive(Clone, Debug)]
pub struct SupabaseConfig {
    pub base_url: String,
    pub key: String,
}

impl SupabaseConfig {
    pub fn from_env() -> Option<Self> {
        let url = env::var("SUPABASE_URL").ok().filter(|s| !s.is_empty())?;
        let key = env::var("SUPABASE_ANON_KEY").ok().filter(|s| !s.is_empty())?;
        Some(Self {
            base_url: format!("{}/rest/v1", url.trim_end_matches('/')),
            key,
        })
    }
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

fn auth(req: reqwest::RequestBuilder, cfg: &SupabaseConfig) -> reqwest::RequestBuilder {
    req.header("apikey", &cfg.key)
        .header("Authorization", format!("Bearer {}", cfg.key))
        .header("Content-Type", "application/json")
}

pub async fn get_tasks(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
) -> Result<Vec<FoTask>, String> {
    let url = format!("{}/tasks?select=*&order=task_number.asc", cfg.base_url);
    let resp = auth(http.get(&url), cfg).send().await.map_err(|e| e.to_string())?;
    let resp = resp.error_for_status().map_err(|e| e.to_string())?;
    let records: Vec<TaskRecord> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(records.into_iter().map(FoTask::from).collect())
}

pub async fn insert_task(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    task: &FoTask,
) -> Result<FoTask, String> {
    let url = format!("{}/tasks", cfg.base_url);
    let body = serde_json::json!({
        "title": task.title,
        "is_completed": task.is_completed,
        "created_at": task.created_at,
        "completed_at": task.completed_at,
        "pomodoro_count": task.pomodoro_count,
    });
    let resp = auth(http.post(&url), cfg)
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
    task: &FoTask,
) -> Result<(), String> {
    let url = format!("{}/tasks?id=eq.{}", cfg.base_url, task.id);
    let body = serde_json::json!({
        "title": task.title,
        "is_completed": task.is_completed,
        "completed_at": task.completed_at,
        "pomodoro_count": task.pomodoro_count,
    });
    let resp = auth(http.patch(&url), cfg).json(&body).send().await.map_err(|e| e.to_string())?;
    resp.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_task(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
    id: &str,
) -> Result<(), String> {
    let url = format!("{}/tasks?id=eq.{}", cfg.base_url, id);
    let resp = auth(http.delete(&url), cfg).send().await.map_err(|e| e.to_string())?;
    resp.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn insert_session(
    http: &reqwest::Client,
    cfg: &SupabaseConfig,
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
    let resp = auth(http.post(&url), cfg).json(&body).send().await.map_err(|e| e.to_string())?;
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
```

- [ ] **Step 3: Make the task commands async and Supabase-aware (with offline fallback)**

In `src-tauri/src/commands.rs`, replace `get_tasks`, `insert_task`, `update_task`, `delete_task` with async versions, and add `record_session`:
```rust
use crate::supabase;

#[tauri::command]
pub async fn get_tasks(state: State<'_, AppState>) -> Result<Vec<FoTask>, String> {
    let dir = state.data_dir.clone();
    let cfg = state.supabase.clone();
    let http = state.http.clone();
    if let Some(cfg) = cfg {
        match supabase::get_tasks(&http, &cfg).await {
            Ok(tasks) => {
                let _ = storage::write_json(&dir, "tasks.json", &tasks);
                return Ok(tasks);
            }
            Err(e) => eprintln!("[supabase] get_tasks failed, falling back to local: {e}"),
        }
    }
    Ok(storage::read_json(&dir, "tasks.json"))
}

#[tauri::command]
pub async fn insert_task(title: String, state: State<'_, AppState>) -> Result<FoTask, String> {
    let dir = state.data_dir.clone();
    let cfg = state.supabase.clone();
    let http = state.http.clone();

    let mut tasks: Vec<FoTask> = storage::read_json(&dir, "tasks.json");
    let n = next_task_number(&tasks);
    let mut task = FoTask {
        id: Uuid::new_v4().to_string(),
        task_id: format!("FO-{:02}", n),
        title: title.trim().to_string(),
        is_completed: false,
        created_at: now_rfc3339(),
        completed_at: None,
        pomodoro_count: 0,
    };
    if let Some(cfg) = cfg {
        match supabase::insert_task(&http, &cfg, &task).await {
            Ok(saved) => task = saved, // adopt DB id + task_number
            Err(e) => eprintln!("[supabase] insert_task failed: {e}"),
        }
    }
    tasks.push(task.clone());
    storage::write_json(&dir, "tasks.json", &tasks)?;
    Ok(task)
}

#[tauri::command]
pub async fn update_task(task: FoTask, state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.data_dir.clone();
    let cfg = state.supabase.clone();
    let http = state.http.clone();

    if let Some(cfg) = cfg {
        if let Err(e) = supabase::update_task(&http, &cfg, &task).await {
            eprintln!("[supabase] update_task failed: {e}");
        }
    }
    let mut tasks: Vec<FoTask> = storage::read_json(&dir, "tasks.json");
    if let Some(slot) = tasks.iter_mut().find(|t| t.id == task.id) {
        *slot = task;
    }
    storage::write_json(&dir, "tasks.json", &tasks)
}

#[tauri::command]
pub async fn delete_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.data_dir.clone();
    let cfg = state.supabase.clone();
    let http = state.http.clone();

    if let Some(cfg) = cfg {
        if let Err(e) = supabase::delete_task(&http, &cfg, &id).await {
            eprintln!("[supabase] delete_task failed: {e}");
        }
    }
    let mut tasks: Vec<FoTask> = storage::read_json(&dir, "tasks.json");
    tasks.retain(|t| t.id != id);
    storage::write_json(&dir, "tasks.json", &tasks)
}

#[tauri::command]
pub async fn record_session(
    task_id: Option<String>,
    duration_minutes: i32,
    was_focused: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cfg = state.supabase.clone();
    let http = state.http.clone();
    if let Some(cfg) = cfg {
        if let Err(e) = supabase::insert_session(&http, &cfg, task_id.as_deref(), duration_minutes, was_focused).await {
            eprintln!("[supabase] record_session failed: {e}");
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Add `record_session` to the handler list in `lib.rs`**

In `src-tauri/src/lib.rs`, add to `invoke_handler!`:
```rust
            commands::record_session,
```

- [ ] **Step 5: Run the Rust tests**

Run:
```bash
cd src-tauri && cargo test && cd ..
```
Expected: PASS — storage tests, `next_task_number` tests, and `task_record_maps_to_fo_task_with_padded_id`.

- [ ] **Step 6: Verify offline mode still works (no `.env`)**

Run (with NO `.env` present):
```bash
npm run tauri dev
```
Expected: console logs `[supabase] .env not found or incomplete — running offline.` Tasks behave exactly as Task 6 (local JSON). Close to stop.

- [ ] **Step 7: Verify online sync (with real `.env`)**

Create a local `.env` (NOT committed) at the project root with valid `SUPABASE_URL` and `SUPABASE_ANON_KEY` for the shared project. Then:
```bash
npm run tauri dev
```
Expected: on launch, tasks load from Supabase (ordered by `task_number`). Adding a task inserts to Supabase and the row shows the DB-assigned `FO-NN`. Toggling/deleting/recording propagate. Confirm in the Supabase dashboard that the `tasks` row and a `pomodoro_sessions` row appear after completing a focus session. Cross-check: a task added on Windows shows up here after relaunch. Close to stop.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: Supabase REST sync with offline JSON fallback"
```

---

## Task 8: Progress stats + daily reset + full session wiring

**Files:**
- Modify: `src-tauri/src/commands.rs` (load_progress with daily reset, save_progress)
- Modify: `src-tauri/src/lib.rs` (add load_progress, save_progress handlers)
- Create: `src/lib/stores/progress.ts`
- Create: `src/lib/components/Progress.svelte`
- Modify: `src/App.svelte` (wire session-complete → progress + tasks.onFocusSessionCompleted + record; wire task-toggled → progress)

- [ ] **Step 1: Add `load_progress` (daily reset) + `save_progress`**

Append to `src-tauri/src/commands.rs`:
```rust
use crate::models::FoSession;

#[tauri::command]
pub fn load_progress(state: State<'_, AppState>) -> Result<FoSession, String> {
    let mut session: FoSession = storage::read_json(&state.data_dir, "session.json");
    let today = today_string();
    if session.date != today {
        session = FoSession { date: today, ..Default::default() };
        storage::write_json(&state.data_dir, "session.json", &session)?;
    }
    Ok(session)
}

#[tauri::command]
pub fn save_progress(session: FoSession, state: State<'_, AppState>) -> Result<(), String> {
    storage::write_json(&state.data_dir, "session.json", &session)
}
```
Add to `lib.rs` `invoke_handler!`:
```rust
            commands::load_progress,
            commands::save_progress,
```

- [ ] **Step 2: Run Rust build/tests to confirm compilation**

Run:
```bash
cd src-tauri && cargo test && cd ..
```
Expected: PASS / compiles cleanly.

- [ ] **Step 3: Create the progress store**

Create `src/lib/stores/progress.ts`:
```ts
import { writable, derived, get } from "svelte/store";
import type { FoSession } from "../types";
import { api } from "../api";

function emptyToday(): FoSession {
  const now = new Date();
  const date = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return { date, focus_sessions_count: 0, total_minutes_studied: 0, tasks_completed_count: 0 };
}

export function createProgressStore() {
  const session = writable<FoSession>(emptyToday());

  const sessionsDisplay = derived(session, ($s) => `${$s.focus_sessions_count} sessions`);
  const minutesDisplay = derived(session, ($s) => `${$s.total_minutes_studied} min`);
  const tasksDisplay = derived(session, ($s) => `${$s.tasks_completed_count} tasks`);

  async function load() {
    session.set(await api.loadProgress());
  }

  // Focus session completed: bump count + minutes, persist.
  async function addFocusSession(minutes: number) {
    const next: FoSession = {
      ...get(session),
      focus_sessions_count: get(session).focus_sessions_count + 1,
      total_minutes_studied: get(session).total_minutes_studied + minutes,
    };
    session.set(next);
    await api.saveProgress(next);
  }

  // Task toggled: set the completed-today count (recomputed by tasks store), persist.
  async function setTasksCompleted(count: number) {
    const next: FoSession = { ...get(session), tasks_completed_count: count };
    session.set(next);
    await api.saveProgress(next);
  }

  return {
    session: { subscribe: session.subscribe },
    sessionsDisplay, minutesDisplay, tasksDisplay,
    load, addFocusSession, setTasksCompleted,
  };
}

export type ProgressStore = ReturnType<typeof createProgressStore>;
```

- [ ] **Step 4: Create the Progress component**

Create `src/lib/components/Progress.svelte`:
```svelte
<script lang="ts">
  import type { ProgressStore } from "../stores/progress";

  let { store }: { store: ProgressStore } = $props();
  const { sessionsDisplay, minutesDisplay, tasksDisplay } = store;
  let expanded = $state(true);
</script>

<section class="progress">
  <div class="header">
    <span class="section-header">TODAY</span>
    <button class="icon" onclick={() => (expanded = !expanded)}>{expanded ? "▾" : "▸"}</button>
  </div>
  {#if expanded}
    <div class="grid">
      <div class="stat">{$sessionsDisplay}</div>
      <div class="stat">{$minutesDisplay}</div>
      <div class="stat">{$tasksDisplay}</div>
    </div>
  {/if}
</section>

<style>
  .progress { margin-bottom: 8px; }
  .header { display: flex; align-items: center; justify-content: space-between; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); margin-top: 8px; }
  .stat { text-align: center; font-size: 16px; font-weight: 600; color: var(--accent); }
</style>
```

- [ ] **Step 5: Wire the full session-complete + task-toggled flow in App.svelte**

In `src/App.svelte` `<script>`, add the progress store and replace the `onSessionComplete` handler so it mirrors `MainViewModel.OnSessionCompleted`:
```svelte
  import Progress from "./lib/components/Progress.svelte";
  import { createProgressStore } from "./lib/stores/progress";

  const progressStore = createProgressStore();

  function wireSessionComplete() {
    pomodoro.onSessionComplete(async (minutes, wasFocus) => {
      playChime();
      if (wasFocus) {
        await tasksStore.onFocusSessionCompleted(minutes); // pomodoro++ + record_session
        await progressStore.addFocusSession(minutes);
        notify("Focus Complete", "Time for a break!");
      } else {
        notify("Break Over", "Back to focus!");
      }
    });
  }
```
Wire task-toggled → progress (after `tasksStore` is created):
```svelte
  tasksStore.registerTaskToggled(() => {
    progressStore.setTasksCompleted(get(tasksStore.todayCompletedCount));
  });
```
Add `import { get } from "svelte/store";`. Extend `onMount` to load progress:
```svelte
    await progressStore.load();
```
Replace the TODAY slot:
```svelte
  <Progress store={progressStore} />
```

- [ ] **Step 6: Run and verify progress + daily reset**

Run:
```bash
npm run tauri dev
```
Verify:
- Set Focus to 1 min (config), Start, let it finish → TODAY shows "1 sessions" and "1 min" (the configured focus minutes), and the active task's `🍅×1` badge appears.
- Complete a task → "N tasks" increments; un-complete → decrements.
- Restart the app on the same day → progress persists. (To verify daily reset, you can temporarily edit `session.json`'s `date` to a past date and relaunch — counts reset to 0.)

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: progress stats, daily reset, and full session wiring"
```

---

## Task 9: Polish — opacity, click-through, position memory, show-on-all-spaces, resize-to-content

**Files:**
- Create: `src/lib/stores/settings.ts`
- Modify: `src/App.svelte` (opacity binding + persistence, click-through toggle + banner, window behaviors, resize-to-content)
- Modify: `src-tauri/Cargo.toml` + `src-tauri/src/lib.rs` (window-state plugin)
- Modify: `src-tauri/capabilities/default.json` (window + window-state permissions)
- Modify: `README.md`

- [ ] **Step 1: Install + register the window-state plugin**

Run:
```bash
cd src-tauri && cargo add tauri-plugin-window-state && cd ..
npm install @tauri-apps/plugin-window-state
```
In `src-tauri/src/lib.rs`, add to the builder chain (before `.setup`):
```rust
        .plugin(tauri_plugin_window_state::Builder::default().build())
```

- [ ] **Step 2: Add window permissions to capabilities**

In `src-tauri/capabilities/default.json`, ensure `"permissions"` includes:
```json
"core:window:allow-set-ignore-cursor-events",
"core:window:allow-set-visible-on-all-workspaces",
"core:window:allow-set-size",
"core:window:allow-inner-size",
"notification:default"
```
(Keep any defaults the scaffold added, e.g. `core:default`.)

- [ ] **Step 3: Create the settings store (opacity + click-through)**

Create `src/lib/stores/settings.ts`:
```ts
import { writable, get } from "svelte/store";
import { api } from "../api";

export function createSettingsStore() {
  const opacity = writable<number>(0.9);
  const clickThrough = writable<boolean>(false);

  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  async function load() {
    const s = await api.loadSettings();
    opacity.set(s.opacity);
  }

  function setOpacity(value: number) {
    opacity.set(value);
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      api.saveSettings({ opacity: get(opacity) });
    }, 200); // debounce slider drags
  }

  async function toggleClickThrough() {
    const next = !get(clickThrough);
    clickThrough.set(next);
    await api.setClickThrough(next);
  }

  return {
    opacity: { subscribe: opacity.subscribe },
    clickThrough: { subscribe: clickThrough.subscribe },
    load, setOpacity, toggleClickThrough,
  };
}

export type SettingsStore = ReturnType<typeof createSettingsStore>;
```

- [ ] **Step 4: Wire opacity, click-through, banner, and window behaviors in App.svelte**

In `src/App.svelte` `<script>`:
```svelte
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { createSettingsStore } from "./lib/stores/settings";

  const settings = createSettingsStore();
  const opacity = settings.opacity;
  const clickThrough = settings.clickThrough;

  let panelEl: HTMLElement | undefined = $state();

  onMount(async () => {
    await settings.load();
    // Show on all Spaces / above fullscreen apps.
    try { await getCurrentWindow().setVisibleOnAllWorkspaces(true); } catch (e) { console.warn(e); }
    // Resize the window to fit the panel content; keep it in sync as sections expand/collapse.
    if (panelEl) {
      const ro = new ResizeObserver(async () => {
        const h = Math.ceil(panelEl!.getBoundingClientRect().height);
        try { await getCurrentWindow().setSize(new LogicalSize(290, h)); } catch (e) { console.warn(e); }
      });
      ro.observe(panelEl);
    }
  });
```
Apply opacity to the panel and replace the opacity slider + title-bar buttons. Update the `<main>` element and title bar:
```svelte
<main class="panel" class:clickthrough={$clickThrough} bind:this={panelEl} style="opacity: {$opacity}">
  <header class="titlebar" data-tauri-drag-region>
    <span class="title">FoPoMoro</span>
    <div class="titlebar-actions">
      <button class="icon" class:on={$clickThrough} title="Toggle click-through" onclick={() => settings.toggleClickThrough()}>⊙</button>
      <button class="danger close" title="Close" onclick={() => getCurrentWindow().close()}>✕</button>
    </div>
  </header>
```
Replace the opacity row:
```svelte
  <div class="opacity-row">
    <span class="section-header">Opacity</span>
    <input
      type="range" min="0.3" max="1" step="0.01"
      value={$opacity}
      oninput={(e) => settings.setOpacity(Number((e.target as HTMLInputElement).value))}
    />
  </div>

  {#if $clickThrough}
    <div class="ct-banner">Click-through active • toggle ⊙ to disable</div>
  {/if}
```
Add to `<style>`:
```svelte
  .panel.clickthrough { background: var(--panel-bg-clickthrough); }
  .ct-banner {
    margin-top: 10px;
    background: var(--accent);
    color: var(--base);
    border-radius: 8px;
    padding: 8px 6px;
    font-size: 10px;
    font-weight: 600;
    text-align: center;
  }
```

- [ ] **Step 5: Run and verify all polish behaviors**

Run:
```bash
npm run tauri dev
```
Verify:
- **Opacity:** drag the slider → the whole panel fades between 0.3 and 1.0. Restart → opacity persists.
- **Position memory:** move the window, close, reopen → it reappears in the same spot (window-state plugin).
- **Resize-to-content:** collapse/expand sections → the window height tracks the panel (no large transparent dead-zone, no clipping).
- **Click-through:** click `⊙` → panel tint shifts to the darker blue, banner appears, and clicks now pass through to the window beneath. Click `⊙` again (the button still receives the click since you re-enable via the toggle — verify the toggle remains hittable; if it does not on your macOS version, note it) to disable.
- **All Spaces:** switch to another Space / a fullscreen app → the overlay remains visible.
- **Close:** `✕` closes the window.

> If the `⊙` toggle becomes unclickable while click-through is active (because the whole window ignores cursor events), document this as a known v1 limitation and add a follow-up to re-enable via a global shortcut or tray item. This mirrors the Windows app, which disables click-through from the tray menu, not the panel.

- [ ] **Step 6: Write the README**

Create/replace `README.md`:
```markdown
# FoPoMoro (macOS)

A semi-transparent, always-on-top floating overlay for productivity and study
tracking — a Tauri + Svelte port of the Windows FoPoMoro app. Clock, Pomodoro
timer, task list, daily progress stats, and Supabase cloud sync (shared backend
with the Windows version).

## Requirements

- Rust toolchain, Node.js, Xcode Command Line Tools.

## Setup

```bash
npm install
cp .env.example .env   # optional: fill in for Supabase sync; omit to run offline
```

`.env`:

```
SUPABASE_URL=https://your-project-id.supabase.co
SUPABASE_ANON_KEY=your-anon-key-here
```

When `.env` is present and complete, Supabase is the source of truth for tasks.
Without it, the app runs offline using local JSON. Progress and settings are
always local (`~/Library/Application Support/com.fopomoro.mac/`).

## Develop

```bash
npm run tauri dev
```

## Build

```bash
npm run tauri build   # produces a .app / .dmg under src-tauri/target/release/bundle
```

## Test

```bash
npm run test                      # Svelte store logic (Vitest)
cd src-tauri && cargo test        # Rust storage / mapping
```

## Notes / Known limitations (v1)

- No code signing / notarization (dev-local run).
- Click-through is toggled from the in-panel ⊙ button; there is no tray icon yet.
- Transparency requires `macOSPrivateApi: true` (disables Mac App Store submission).
```

- [ ] **Step 7: Final full verification pass**

Run the complete suite, then a manual run:
```bash
npm run test
cd src-tauri && cargo test && cd ..
npm run tauri dev
```
Walk the full feature checklist (clock ticking; pomodoro start/pause/reset; 4-dot cadence → long break; config persists; tasks add/toggle/delete with FO-NN; active dot + 🍅 count; locked ops while running; progress sessions/minutes/tasks; daily reset; Supabase sync if `.env` present; opacity; click-through; position memory; all-Spaces; resize-to-content). Confirm each works.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: opacity, click-through, position memory, all-spaces, resize-to-content + README"
```

---

## Self-Review (completed during authoring)

**Spec coverage** — every spec section maps to a task:
- Overlay window (transparent/topmost/draggable/decorations:false) → Task 1
- WPF/MVVM → Tauri/Svelte mapping (Services=Rust, ViewModels=stores, Views=components) → Tasks 3–9
- Repository structure → file structure section + tasks create exactly those files
- Data models (FoTask/FoSession/PomodoroConfig/WindowSettings) → Task 5 (Rust) + Tasks 4–5 (TS)
- Rust command surface (get/insert/update/delete tasks, record_session, load_progress, load/save settings, set_click_through) → Tasks 5–8 (plus added save_progress, load/save_config, noted)
- Sync strategy (Supabase source of truth when `.env` complete; offline fallback; progress/settings local) → Task 7
- Overlay behavior table (always-on-top, transparent, draggable, click-through, opacity, position memory, expand/collapse) → Tasks 1, 2, 9
- Pomodoro & Clock (frontend setInterval, Idle→Focus→ShortBreak→LongBreak, 4→long break, on-end sound+notification+record) → Tasks 3, 4, 8
- Build & run (create-tauri-app, tauri dev, tauri build) → Task 1, README
- Testing (Rust storage/env/mapping; Vitest store logic; manual overlay) → Tasks 3–8 tests + manual steps
- macOS nuances (NSWindow level/all-Spaces, notification permission, signing out of scope) → Tasks 4, 9 + README

**Placeholder scan** — no "TBD"/"add error handling"/"similar to Task N" left; every code step has complete code. The two cross-task forward references (`types.ts` minimal in Task 4 then full in Task 5; `supabase.rs` stub in Task 5 then full in Task 7) are explicit and intentional, with the handler-list trimming/restoring called out so each task compiles in isolation.

**Type consistency** — Rust field names are snake_case and match the Supabase columns and the TS interfaces (`task_id`, `is_completed`, `created_at`, `completed_at`, `pomodoro_count`, `focus_minutes`, etc.). Command names match between `lib.rs` `generate_handler!`, `commands.rs` functions, and `api.ts` `invoke` strings (`get_tasks`, `insert_task`, `update_task`, `delete_task`, `record_session`, `load_progress`, `save_progress`, `load_config`, `save_config`, `load_settings`, `save_settings`, `set_click_through`). `recordSession(taskId, durationMinutes, wasFocused)` in `api.ts` maps to `record_session(task_id, duration_minutes, was_focused)` — Tauri auto-converts camelCase JS args to snake_case Rust params.

---

## Risks / Follow-ups (not v1 blockers)

- **Above-fullscreen floating:** `alwaysOnTop` + `setVisibleOnAllWorkspaces` covers most cases; if it still hides behind a fullscreen Space, an objc NSWindow-level tweak (`NSStatusWindowLevel`) is the follow-up.
- **Click-through re-enable:** while ignoring cursor events the panel toggle may be unreachable; add a global shortcut or tray item later.
- **Bundled `.env`:** `dotenvy` reads from cwd (works in `tauri dev`). A signed/distributed build needs config injected differently — out of scope for v1.
- **Tray icon:** Windows had one; add `tauri-plugin-tray`/`TrayIconBuilder` later for parity.
