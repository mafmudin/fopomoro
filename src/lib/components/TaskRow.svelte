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
