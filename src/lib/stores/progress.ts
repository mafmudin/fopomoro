import { writable, derived, get } from "svelte/store";
import type { FoSession } from "../types";
import { api } from "../api";

function emptyToday(): FoSession {
  const now = new Date();
  const date = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return { date, focus_sessions_count: 0, total_minutes_studied: 0, tasks_completed_count: 0 };
}

export function createProgressStore() {
  const session = writable<FoSession>(emptyToday());

  const sessionsDisplay = derived(session, ($s) => `${$s.focus_sessions_count} sessions`);
  const minutesDisplay = derived(session, ($s) => `${$s.total_minutes_studied} min`);
  const tasksDisplay = derived(session, ($s) => `${$s.tasks_completed_count} tasks`);

  async function load() {
    try {
      session.set(await api.loadProgress());
    } catch (e) {
      console.error("[progress] load failed:", e);
    }
  }

  // Focus session completed: bump count + minutes, persist.
  async function addFocusSession(minutes: number) {
    const cur = get(session);
    const next: FoSession = {
      ...cur,
      focus_sessions_count: cur.focus_sessions_count + 1,
      total_minutes_studied: cur.total_minutes_studied + minutes,
    };
    session.set(next);
    try {
      await api.saveProgress(next);
    } catch (e) {
      console.error("[progress] save failed:", e);
    }
  }

  // Task toggled: set the completed-today count (recomputed by tasks store), persist.
  async function setTasksCompleted(count: number) {
    const next: FoSession = { ...get(session), tasks_completed_count: count };
    session.set(next);
    try {
      await api.saveProgress(next);
    } catch (e) {
      console.error("[progress] save failed:", e);
    }
  }

  return {
    session: { subscribe: session.subscribe },
    sessionsDisplay, minutesDisplay, tasksDisplay,
    load, addFocusSession, setTasksCompleted,
  };
}

export type ProgressStore = ReturnType<typeof createProgressStore>;
