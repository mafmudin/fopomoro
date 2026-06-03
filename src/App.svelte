<script lang="ts">
  import Clock from "./lib/components/Clock.svelte";
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
