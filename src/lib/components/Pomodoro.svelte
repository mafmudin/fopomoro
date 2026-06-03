<script lang="ts">
  import type { Pomodoro } from "../stores/timer";
  import type { PomodoroConfig } from "../types";

  let {
    pomodoro,
    onConfigSaved,
  }: {
    pomodoro: Pomodoro;
    onConfigSaved?: (config: PomodoroConfig) => void;
  } = $props();

  // $derived so the subscription tracks `pomodoro` if the prop is ever reassigned.
  const pomoState = $derived(pomodoro.state);

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

  function clampMinutes(value: string, delta: number): string {
    const n = Number(value);
    const base = Number.isInteger(n) ? n : 1;
    return String(Math.min(180, Math.max(1, base + delta)));
  }

  function apply() {
    const ok = pomodoro.applyConfig(focusText, shortText, longText);
    if (!ok) {
      configError = "All fields must be a positive number.";
      return;
    }
    configError = "";
    configOpen = false;
    onConfigSaved?.(pomodoro.getConfig());
  }
</script>

<section class="pomo">
  <div class="header">
    <div class="header-left">
      <span class="section-header">POMODORO</span>
      <span class="dots">
        {#each [1, 2, 3, 4] as n}
          <span class="dot" class:on={$pomoState.completedSessions >= n}></span>
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
      <div class="state-label" style="color: {labelColor($pomoState.label)}">{$pomoState.label}</div>
      <div class="countdown">{$pomoState.timeDisplay}</div>

      <div class="controls">
        {#if !$pomoState.isRunning}
          <button onclick={() => pomodoro.start()}>Start</button>
        {:else}
          <button class="secondary" onclick={() => pomodoro.pause()}>Pause</button>
        {/if}
        <button class="secondary" onclick={() => pomodoro.reset()}>Reset</button>
      </div>

      {#if configOpen}
        <div class="config">
          <div class="config-row">
            <span class="config-label">Focus</span>
            <button class="secondary tiny" onclick={() => (focusText = clampMinutes(focusText, -1))}>−</button>
            <input class="num" bind:value={focusText} />
            <button class="secondary tiny" onclick={() => (focusText = clampMinutes(focusText, 1))}>+</button>
            <span class="unit">min</span>
          </div>
          <div class="config-row">
            <span class="config-label">Short Break</span>
            <button class="secondary tiny" onclick={() => (shortText = clampMinutes(shortText, -1))}>−</button>
            <input class="num" bind:value={shortText} />
            <button class="secondary tiny" onclick={() => (shortText = clampMinutes(shortText, 1))}>+</button>
            <span class="unit">min</span>
          </div>
          <div class="config-row">
            <span class="config-label">Long Break</span>
            <button class="secondary tiny" onclick={() => (longText = clampMinutes(longText, -1))}>−</button>
            <input class="num" bind:value={longText} />
            <button class="secondary tiny" onclick={() => (longText = clampMinutes(longText, 1))}>+</button>
            <span class="unit">min</span>
          </div>
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
