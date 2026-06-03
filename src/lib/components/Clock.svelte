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
