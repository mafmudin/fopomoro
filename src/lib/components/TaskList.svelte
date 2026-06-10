<script lang="ts">
  import type { TasksStore } from "../stores/tasks";
  import TaskRow from "./TaskRow.svelte";
  import { topIncomplete, completedTasks, shouldShowSeeAll } from "../taskFilters";
  import { openAllTasksWindow } from "../windows";

  let { store, timerRunning = false }: { store: TasksStore; timerRunning?: boolean } = $props();

  const tasks = $derived(store.tasks);
  const newTaskTitle = $derived(store.newTaskTitle);
  const activeTaskId = $derived(store.activeTaskId);
  const taskCountDisplay = $derived(store.taskCountDisplay);

  const visible = $derived(topIncomplete($tasks, 5));
  const total = $derived($tasks.length);
  const completedCount = $derived(completedTasks($tasks).length);
  const showSeeAll = $derived(shouldShowSeeAll(total, completedCount));

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
        {#each visible as task (task.id)}
          <TaskRow {task} {store} activeId={$activeTaskId} {timerRunning} />
        {/each}
      </div>

      {#if showSeeAll}
        <button class="see-all" onclick={() => openAllTasksWindow()}>See all ({total}) ▸</button>
      {/if}

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
  .see-all { width: 100%; margin-top: 6px; font-size: 11px; color: var(--accent); background: none; border: none; cursor: pointer; padding: 4px; }
  .see-all:hover { text-decoration: underline; }
  .add-row { display: flex; gap: 6px; margin-top: 8px; }
  .add-row input { flex: 1; }
  .add { width: 36px; padding: 0; }
</style>
