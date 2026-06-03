import { writable, derived, get } from "svelte/store";
import type { FoTask } from "../types";
import { api } from "../api";

function isToday(iso: string | null): boolean {
  if (!iso) return false;
  const d = new Date(iso);
  const now = new Date();
  return d.getFullYear() === now.getFullYear()
    && d.getMonth() === now.getMonth()
    && d.getDate() === now.getDate();
}

export function createTasksStore() {
  const tasks = writable<FoTask[]>([]);
  const newTaskTitle = writable<string>("");
  const activeTaskId = writable<string | null>(null);

  let timerRunning = false;
  let switchedDuringSession = false;

  const taskCountDisplay = derived(tasks, ($t) => {
    const done = $t.filter((x) => x.is_completed).length;
    return `${done} / ${$t.length} done`;
  });

  const todayCompletedCount = derived(tasks, ($t) =>
    $t.filter((x) => x.is_completed && isToday(x.completed_at)).length
  );

  // App.svelte registers this to update progress when a task is toggled.
  let onTaskToggled: (() => void) | null = null;

  async function load() {
    const list = await api.getTasks();
    tasks.set(list);
  }

  async function add(titleArg?: string) {
    const title = (titleArg ?? get(newTaskTitle)).trim();
    if (!title) return;
    try {
      const created = await api.insertTask(title);
      tasks.update((arr) => [...arr, created]);
      newTaskTitle.set("");
    } catch (e) {
      console.error("[tasks] insert failed:", e);
    }
  }

  async function toggle(task: FoTask) {
    const active = get(activeTaskId);
    if (timerRunning && active !== null && active !== task.id) return;
    const updated: FoTask = {
      ...task,
      is_completed: !task.is_completed,
      completed_at: !task.is_completed ? new Date().toISOString() : null,
    };
    tasks.update((arr) => arr.map((t) => (t.id === task.id ? updated : t)));
    onTaskToggled?.();
    try {
      await api.updateTask(updated);
    } catch (e) {
      console.error("[tasks] update failed:", e);
    }
  }

  async function remove(task: FoTask) {
    if (timerRunning) return;
    if (get(activeTaskId) === task.id) activeTaskId.set(null);
    tasks.update((arr) => arr.filter((t) => t.id !== task.id));
    try {
      await api.deleteTask(task.id);
    } catch (e) {
      console.error("[tasks] delete failed:", e);
    }
  }

  function setActive(task: FoTask) {
    const current = get(activeTaskId);
    if (timerRunning && current !== null && current !== task.id) {
      switchedDuringSession = true;
    }
    if (current === task.id) {
      activeTaskId.set(null);
      return;
    }
    activeTaskId.set(task.id);
  }

  function setTimerRunning(running: boolean) {
    if (!running) switchedDuringSession = false;
    timerRunning = running;
  }

  async function onFocusSessionCompleted(durationMinutes: number) {
    const switched = switchedDuringSession;
    switchedDuringSession = false;
    const activeId = get(activeTaskId);

    if (!switched && activeId !== null) {
      const current = get(tasks).find((t) => t.id === activeId);
      if (current) {
        const updated: FoTask = { ...current, pomodoro_count: current.pomodoro_count + 1 };
        tasks.update((arr) => arr.map((t) => (t.id === activeId ? updated : t)));
        try {
          await api.updateTask(updated);
        } catch (e) {
          console.error("[tasks] update (pomodoro) failed:", e);
        }
      }
    }

    try {
      await api.recordSession(switched ? null : activeId, durationMinutes, !switched);
    } catch (e) {
      console.error("[tasks] recordSession failed:", e);
    }
  }

  function registerTaskToggled(cb: () => void) { onTaskToggled = cb; }

  return {
    tasks: { subscribe: tasks.subscribe },
    newTaskTitle,
    activeTaskId: { subscribe: activeTaskId.subscribe },
    taskCountDisplay,
    todayCompletedCount,
    load, add, toggle, remove, setActive, setTimerRunning,
    onFocusSessionCompleted, registerTaskToggled,
  };
}

export type TasksStore = ReturnType<typeof createTasksStore>;
