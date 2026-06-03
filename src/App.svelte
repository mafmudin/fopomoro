<script lang="ts">
  import { onDestroy } from "svelte";
  import Clock from "./lib/components/Clock.svelte";
  import Pomodoro from "./lib/components/Pomodoro.svelte";
  import { createPomodoro } from "./lib/stores/timer";
  import { playChime } from "./lib/sound";
  import { notify } from "./lib/notify";

  // Config persistence comes online in a later task; default for now.
  const pomodoro = createPomodoro({ focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 });

  pomodoro.onSessionComplete((_minutes, wasFocus) => {
    playChime();
    if (wasFocus) notify("Focus Complete", "Time for a break!");
    else notify("Break Over", "Back to focus!");
  });

  onDestroy(() => pomodoro.dispose());
</script>

<main class="panel">
  <header class="titlebar" data-tauri-drag-region>
    <span class="title">FoPoMoro</span>
    <div class="titlebar-actions">
      <button class="icon" title="Toggle click-through">⊙</button>
      <button class="danger close" title="Close">✕</button>
    </div>
  </header>

  <Clock />
  <Pomodoro {pomodoro} />
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
