<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import Clock from "./lib/components/Clock.svelte";
  import Pomodoro from "./lib/components/Pomodoro.svelte";
  import TaskList from "./lib/components/TaskList.svelte";
  import Progress from "./lib/components/Progress.svelte";
  import { createPomodoro } from "./lib/stores/timer";
  import { createTasksStore } from "./lib/stores/tasks";
  import { createProgressStore } from "./lib/stores/progress";
  import { createSettingsStore } from "./lib/stores/settings";
  import { get } from "svelte/store";
  import { playChime } from "./lib/sound";
  import { notify } from "./lib/notify";
  import { api } from "./lib/api";
  import type { PomodoroConfig } from "./lib/types";

  const pomodoro = createPomodoro({ focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 });
  const tasksStore = createTasksStore();
  const progressStore = createProgressStore();
  const pomodoroState = pomodoro.state;

  const settings = createSettingsStore();
  const opacity = settings.opacity;
  const clickThrough = settings.clickThrough;

  let panelEl: HTMLElement | undefined = $state();
  let ro: ResizeObserver | undefined;

  // The timer's running state has two consumers: this effect drives the tasks
  // store's lock guards, and `timerRunning` is also passed to <TaskList> as a
  // prop to show the "locked" notice.
  $effect(() => {
    tasksStore.setTimerRunning($pomodoroState.isRunning);
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
  });

  function persistConfig(config: PomodoroConfig) {
    api.saveConfig(config).catch((e) => console.warn("[fopomoro] saveConfig failed:", e));
  }

  onDestroy(() => {
    pomodoro.dispose();
    ro?.disconnect();
  });
</script>

<main class="panel" class:clickthrough={$clickThrough} bind:this={panelEl} style="opacity: {$opacity}">
  <header class="titlebar" data-tauri-drag-region>
    <span class="title">FoPoMoro</span>
    <div class="titlebar-actions">
      <button class="icon" class:on={$clickThrough} title="Toggle click-through" onclick={() => settings.toggleClickThrough()}>⊙</button>
      <button class="danger close" title="Close" onclick={() => getCurrentWindow().close()}>✕</button>
    </div>
  </header>

  <Clock />
  <Pomodoro {pomodoro} onConfigSaved={persistConfig} />
  <TaskList store={tasksStore} timerRunning={$pomodoroState.isRunning} />
  <Progress store={progressStore} />

  <div class="opacity-row">
    <span class="section-header">Opacity</span>
    <input
      type="range" min="0.3" max="1" step="0.01"
      value={$opacity}
      oninput={(e) => settings.setOpacity(Number((e.target as HTMLInputElement).value))}
    />
  </div>

  {#if $clickThrough}
    <div class="ct-banner">Click-through active • relaunch to disable</div>
  {/if}
</main>

<style>
  .panel {
    width: 290px;
    box-sizing: border-box;
    padding: 14px;
    border-radius: 16px;
    background: var(--panel-bg);
  }
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
  .opacity-row { display: flex; align-items: center; gap: 8px; margin-top: 4px; }
  .opacity-row input[type="range"] { flex: 1; accent-color: var(--accent); }
</style>
