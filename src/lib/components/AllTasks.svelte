<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { createTasksStore } from "../stores/tasks";
  import { createSettingsStore } from "../stores/settings";
  import { subscribe, EVENTS } from "../sync";
  import { incompleteTasks, completedTasks } from "../taskFilters";
  import { textColorsFor } from "../contrast";
  import TaskRow from "./TaskRow.svelte";

  const store = createTasksStore();
  const tasks = $derived(store.tasks);
  const newTaskTitle = $derived(store.newTaskTitle);
  const activeTaskId = $derived(store.activeTaskId);

  const settings = createSettingsStore();
  const bgColor = settings.bgColor;
  const textColors = $derived(textColorsFor($bgColor));

  let timerRunning = $state(false);
  const incomplete = $derived(incompleteTasks($tasks));
  const completed = $derived(completedTasks($tasks));
  const doneCount = $derived(completed.length);

  let unlisteners: Array<() => void> = [];
  let unfocus: (() => void) | undefined;

  function onInputKey(e: KeyboardEvent) {
    if (e.key === "Enter") store.add();
  }

  onMount(async () => {
    await settings.load();
    await store.load();

    unlisteners.push(await subscribe(EVENTS.tasksChanged, () => { void store.load(); }));
    unlisteners.push(await subscribe(EVENTS.activeChanged, (id) => store.applyActiveId(id as string | null)));
    unlisteners.push(await subscribe(EVENTS.timerRunning, (running) => { timerRunning = running as boolean; store.setTimerRunning(running as boolean); }));

    // Backstop: refetch when this window regains focus.
    unfocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) void store.load();
    });
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
    unfocus?.();
  });
</script>

<main class="all" style="--panel-bg:{$bgColor}; --text:{textColors.text}; --subtext:{textColors.subtext}">
  <header class="bar">
    <span class="title">All Tasks</span>
    <span class="count">{doneCount} / {$tasks.length} done</span>
  </header>

  <div class="scroll">
    <div class="section-header">INCOMPLETE</div>
    {#each incomplete as task (task.id)}
      <TaskRow {task} {store} activeId={$activeTaskId} {timerRunning} />
    {/each}
    {#if incomplete.length === 0}<div class="empty">No incomplete tasks 🎉</div>{/if}

    <div class="section-header done-header">COMPLETED</div>
    {#each completed as task (task.id)}
      <TaskRow {task} {store} activeId={$activeTaskId} {timerRunning} />
    {/each}
    {#if completed.length === 0}<div class="empty">No completed tasks yet</div>{/if}
  </div>

  <div class="add-row">
    <input type="text" placeholder="Add a task…"
      value={$newTaskTitle}
      oninput={(e) => store.newTaskTitle.set((e.target as HTMLInputElement).value)}
      onkeydown={onInputKey} />
    <button class="add" onclick={() => store.add()}>+</button>
  </div>
</main>

<style>
  .all { box-sizing: border-box; min-height: 100vh; padding: 16px; background: var(--panel-bg); display: flex; flex-direction: column; }
  .bar { display: flex; align-items: center; justify-content: space-between; margin-bottom: 10px; }
  .title { font-size: 15px; font-weight: 600; color: var(--text); }
  .count { font-size: 11px; color: var(--subtext); }
  .scroll { flex: 1; overflow-y: auto; }
  .section-header { font-size: 10px; letter-spacing: 0.08em; color: var(--subtext); margin: 10px 0 4px; }
  .done-header { margin-top: 16px; }
  .empty { font-size: 12px; color: var(--subtext); opacity: 0.7; padding: 4px 0; }
  .add-row { display: flex; gap: 6px; margin-top: 12px; }
  .add-row input { flex: 1; }
  .add { width: 36px; padding: 0; }
</style>
