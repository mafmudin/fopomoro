<script lang="ts">
  import type { ProgressStore } from "../stores/progress";

  let { store }: { store: ProgressStore } = $props();

  const sessionsDisplay = $derived(store.sessionsDisplay);
  const minutesDisplay = $derived(store.minutesDisplay);
  const tasksDisplay = $derived(store.tasksDisplay);

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
