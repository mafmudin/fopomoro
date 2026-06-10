<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { enable as enableAutostart, disable as disableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart";
  import Clock from "./lib/components/Clock.svelte";
  import Pomodoro from "./lib/components/Pomodoro.svelte";
  import TaskList from "./lib/components/TaskList.svelte";
  import Progress from "./lib/components/Progress.svelte";
  import Account from "./lib/components/Account.svelte";
  import { createPomodoro } from "./lib/stores/timer";
  import { createTasksStore } from "./lib/stores/tasks";
  import { createProgressStore } from "./lib/stores/progress";
  import { createSettingsStore } from "./lib/stores/settings";
  import { get } from "svelte/store";
  import { playChime } from "./lib/sound";
  import { notify } from "./lib/notify";
  import { api } from "./lib/api";
  import { subscribe, broadcast, EVENTS } from "./lib/sync";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { textColorsFor } from "./lib/contrast";
  import type { PomodoroConfig } from "./lib/types";

  const pomodoro = createPomodoro({ focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 });
  const tasksStore = createTasksStore();
  const progressStore = createProgressStore();
  const pomodoroState = pomodoro.state;

  const settings = createSettingsStore();
  const opacity = settings.opacity;
  const bgColor = settings.bgColor;

  // Curated dark Catppuccin swatches; the color picker covers anything else.
  const PRESET_BGS = ["#1E1E2E", "#181825", "#11111B", "#24273A", "#303446"];
  // Text flips to stay readable on whatever background is picked.
  const textColors = $derived(textColorsFor($bgColor));

  let panelEl: HTMLElement | undefined = $state();
  let ro: ResizeObserver | undefined;
  let appUnlisteners: Array<() => void> = [];
  let appUnfocus: (() => void) | undefined;

  // "Run at startup" — source of truth is the OS (registry/LaunchAgent), read via
  // the autostart plugin; we mirror it into this state for the toggle.
  let autostartOn = $state(false);

  async function toggleAutostart(on: boolean) {
    autostartOn = on; // optimistic
    try {
      if (on) await enableAutostart();
      else await disableAutostart();
      autostartOn = await isAutostartEnabled(); // reconcile with OS truth
    } catch (e) {
      console.warn("[fopomoro] autostart toggle failed:", e);
      autostartOn = await isAutostartEnabled().catch(() => false);
    }
  }

  // The timer's running state has two consumers: this effect drives the tasks
  // store's lock guards, and `timerRunning` is also passed to <TaskList> as a
  // prop to show the "locked" notice.
  $effect(() => {
    const running = $pomodoroState.isRunning;
    tasksStore.setTimerRunning(running);
    void broadcast(EVENTS.timerRunning, running);
  });

  pomodoro.onSessionComplete(async (minutes, wasFocus) => {
    playChime();
    if (wasFocus) {
      await tasksStore.onFocusSessionCompleted(minutes);
      await progressStore.addFocusSession(minutes);
      notify("Focus Complete", "Time for a break!");
    } else {
      notify("Break Over", "Back to focus!");
    }
  });

  tasksStore.registerTaskToggled(() => {
    progressStore.setTasksCompleted(get(tasksStore.todayCompletedCount));
  });

  onMount(async () => {
    try {
      const cfg = await api.loadConfig();
      pomodoro.applyConfig(
        String(cfg.focus_minutes),
        String(cfg.short_break_minutes),
        String(cfg.long_break_minutes),
      );
    } catch (e) {
      console.warn("[fopomoro] loadConfig failed, using defaults:", e);
    }
    await tasksStore.load();
    await progressStore.load();

    await settings.load();
    try {
      autostartOn = await isAutostartEnabled();
    } catch (e) {
      console.warn("[fopomoro] isAutostartEnabled failed:", e);
    }
    // Show on all Spaces / above fullscreen apps.
    try {
      await getCurrentWindow().setVisibleOnAllWorkspaces(true);
    } catch (e) {
      console.warn("[fopomoro] setVisibleOnAllWorkspaces failed:", e);
    }
    // Resize the window to fit the panel; keep it in sync as sections expand/collapse.
    // Observes the panel (content), not the window — setSize does not trigger the
    // observer, so there is no resize feedback loop.
    if (panelEl) {
      const el = panelEl;
      ro = new ResizeObserver(async () => {
        const h = Math.ceil(el.getBoundingClientRect().height);
        try {
          await getCurrentWindow().setSize(new LogicalSize(290, h));
        } catch (e) {
          console.warn("[fopomoro] setSize failed:", e);
        }
      });
      ro.observe(el);
    }

    // Cross-window sync: reflect changes made in the All Tasks window.
    appUnlisteners.push(await subscribe(EVENTS.tasksChanged, () => { void tasksStore.load(); void progressStore.load(); }));
    appUnlisteners.push(await subscribe(EVENTS.activeChanged, (id) => tasksStore.applyActiveId(id as string | null)));
    appUnlisteners.push(await subscribe(EVENTS.timerRunning, (running) => tasksStore.setTimerRunning(running as boolean)));
    appUnfocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) void tasksStore.load();
    });
  });

  function persistConfig(config: PomodoroConfig) {
    api.saveConfig(config).catch((e) => console.warn("[fopomoro] saveConfig failed:", e));
  }

  // After sign-in/out the backend reconciles the local mirror with the cloud,
  // so re-pull tasks (and progress) to reflect the new source of truth.
  async function reloadAfterAuth() {
    await tasksStore.load();
    await progressStore.load();
  }

  async function exportTasks() {
    try {
      const path = await save({
        defaultPath: `fopomoro-tasks-${new Date().toISOString().slice(0, 10)}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      await api.exportTasksTo(path);
    } catch (e) {
      console.error("[fopomoro] export failed:", e);
    }
  }

  async function importTasks() {
    try {
      const selected = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
      if (typeof selected !== "string") return;
      await api.importTasksFrom(selected);
      await tasksStore.load();
      await broadcast(EVENTS.tasksChanged);
    } catch (e) {
      console.error("[fopomoro] import failed:", e);
    }
  }

  onDestroy(() => {
    pomodoro.dispose();
    ro?.disconnect();
    appUnlisteners.forEach((u) => u());
    appUnfocus?.();
  });
</script>

<main
  class="panel"
  bind:this={panelEl}
  style="--panel-bg:{$bgColor}; --text:{textColors.text}; --subtext:{textColors.subtext}; opacity:{$opacity}"
>
  <header class="titlebar" data-tauri-drag-region>
    <span class="title" data-tauri-drag-region>FoPoMoro</span>
    <div class="titlebar-actions">
      <button class="min" title="Minimize" onclick={() => getCurrentWindow().minimize()}>–</button>
      <button class="danger close" title="Close" onclick={() => getCurrentWindow().close()}>✕</button>
    </div>
  </header>

  <Clock />
  <Pomodoro {pomodoro} onConfigSaved={persistConfig} />
  <TaskList store={tasksStore} timerRunning={$pomodoroState.isRunning} />
  <Progress store={progressStore} />

  <div class="bg-row">
    <span class="section-header">Background</span>
    <div class="swatches">
      {#each PRESET_BGS as preset}
        <button
          class="swatch"
          class:active={$bgColor.toLowerCase() === preset.toLowerCase()}
          style="background: {preset}"
          title={preset}
          aria-label={`Set background ${preset}`}
          onclick={() => settings.setBgColor(preset)}
        ></button>
      {/each}
      <input
        class="color-picker"
        type="color"
        value={$bgColor}
        oninput={(e) => settings.setBgColor((e.target as HTMLInputElement).value)}
        title="Custom color"
        aria-label="Custom background color"
      />
    </div>
  </div>

  <div class="opacity-row">
    <span class="section-header">Opacity</span>
    <input
      type="range" min="0.3" max="1" step="0.01"
      value={$opacity}
      oninput={(e) => settings.setOpacity(Number((e.target as HTMLInputElement).value))}
    />
  </div>

  <div class="startup-row">
    <span class="section-header">Run at startup</span>
    <input
      type="checkbox"
      checked={autostartOn}
      onchange={(e) => toggleAutostart((e.target as HTMLInputElement).checked)}
      aria-label="Run FoPoMoro at login"
    />
  </div>

  <div class="data-row">
    <span class="section-header">Tasks data</span>
    <div class="data-actions">
      <button class="data-btn" onclick={importTasks}>Import</button>
      <button class="data-btn" onclick={exportTasks}>Export</button>
    </div>
  </div>

  <Account onAuthChanged={reloadAfterAuth} />
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
  .min { width: 24px; height: 24px; padding: 0; font-size: 14px; line-height: 1; }
  .close { width: 24px; height: 24px; padding: 0; font-size: 11px; }
  .opacity-row { display: flex; align-items: center; gap: 8px; margin-top: 4px; }
  .opacity-row input[type="range"] { flex: 1; accent-color: var(--accent); }

  .startup-row { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 8px; }
  .startup-row input[type="checkbox"] { accent-color: var(--accent); cursor: pointer; }

  .data-row { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 8px; }
  .data-actions { display: flex; gap: 6px; }
  .data-btn { font-size: 11px; padding: 3px 8px; }

  .bg-row { display: flex; align-items: center; gap: 8px; margin-top: 8px; }
  .swatches { display: flex; align-items: center; gap: 6px; flex: 1; }
  .swatch {
    width: 18px;
    height: 18px;
    padding: 0;
    border-radius: 4px;
    border: 2px solid transparent;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.12) inset;
  }
  .swatch.active { border-color: var(--accent); }
  .color-picker {
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: none;
    cursor: pointer;
  }
  .color-picker::-webkit-color-swatch-wrapper { padding: 0; }
  .color-picker::-webkit-color-swatch { border: none; border-radius: 4px; }
</style>
