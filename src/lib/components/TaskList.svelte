<script lang="ts">
  import type { TasksStore } from "../stores/tasks";

  let { store, timerRunning = false }: { store: TasksStore; timerRunning?: boolean } = $props();

  const tasks = $derived(store.tasks);
  const newTaskTitle = $derived(store.newTaskTitle);
  const activeTaskId = $derived(store.activeTaskId);
  const taskCountDisplay = $derived(store.taskCountDisplay);

  let expanded = $state(true);

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
              aria-label={`Set active task: ${task.title}`}
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
            <button class="danger del" title="Delete" aria-label={`Delete ${task.title}`} onclick={() => store.remove(task)}>×</button>
          </div>
        {/each}
      </div>

      <div class="add-row">
        <input
          type="text"
          placeholder="Add a task…"
          value={$newTaskTitle}
          oninput={(e) => store.newTaskTitle.set((e.target as HTMLInputElement).value)}
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
