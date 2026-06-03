import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";
import type { FoTask } from "../types";

// Mock the api module before importing the store.
const inserted: FoTask[] = [];
vi.mock("../api", () => ({
  api: {
    getTasks: vi.fn(async () => []),
    insertTask: vi.fn(async (title: string) => {
      const t: FoTask = {
        id: `id-${inserted.length + 1}`,
        task_id: `FO-0${inserted.length + 1}`,
        title,
        is_completed: false,
        created_at: "2026-06-03T00:00:00+00:00",
        completed_at: null,
        pomodoro_count: 0,
      };
      inserted.push(t);
      return t;
    }),
    updateTask: vi.fn(async () => {}),
    deleteTask: vi.fn(async () => {}),
    recordSession: vi.fn(async () => {}),
  },
}));

import { createTasksStore } from "./tasks";
import { api } from "../api";

beforeEach(() => {
  inserted.length = 0;
  vi.clearAllMocks();
});

describe("tasks store", () => {
  it("adds a task and clears the input", async () => {
    const s = createTasksStore();
    await s.add("Read chapter 3");
    const tasks = get(s.tasks);
    expect(tasks).toHaveLength(1);
    expect(tasks[0].title).toBe("Read chapter 3");
    expect(get(s.newTaskTitle)).toBe("");
  });

  it("blocks toggling a non-active task while the timer runs", async () => {
    const s = createTasksStore();
    await s.add("A");
    await s.add("B");
    const [a, b] = get(s.tasks);
    s.setActive(a);
    s.setTimerRunning(true);
    await s.toggle(b); // should be ignored
    expect(get(s.tasks).find((t) => t.id === b.id)!.is_completed).toBe(false);
    await s.toggle(a); // active task can toggle
    expect(get(s.tasks).find((t) => t.id === a.id)!.is_completed).toBe(true);
  });

  it("blocks deleting any task while the timer runs", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setTimerRunning(true);
    await s.remove(a);
    expect(get(s.tasks)).toHaveLength(1);
  });

  it("setActive toggles off when the same task is clicked again", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setActive(a);
    expect(get(s.activeTaskId)).toBe(a.id);
    s.setActive(a);
    expect(get(s.activeTaskId)).toBe(null);
  });

  it("focus completion increments the active task pomodoro and records with task_id", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setActive(a);
    await s.onFocusSessionCompleted(25);
    expect(get(s.tasks)[0].pomodoro_count).toBe(1);
    expect(api.recordSession).toHaveBeenCalledWith(a.id, 25, true);
  });

  it("switching tasks mid-session records null/false and does NOT increment", async () => {
    const s = createTasksStore();
    await s.add("A");
    await s.add("B");
    const [a, b] = get(s.tasks);
    s.setActive(a);
    s.setTimerRunning(true);
    s.setActive(b); // switch during run -> sets the quirk flag
    await s.onFocusSessionCompleted(25);
    expect(get(s.tasks)[0].pomodoro_count).toBe(0);
    expect(get(s.tasks)[1].pomodoro_count).toBe(0);
    expect(api.recordSession).toHaveBeenCalledWith(null, 25, false);
  });

  it("toggling a task off clears completed_at", async () => {
    const s = createTasksStore();
    await s.add("A");
    await s.toggle(get(s.tasks)[0]); // complete
    expect(get(s.tasks)[0].is_completed).toBe(true);
    expect(get(s.tasks)[0].completed_at).not.toBeNull();
    await s.toggle(get(s.tasks)[0]); // un-complete
    expect(get(s.tasks)[0].is_completed).toBe(false);
    expect(get(s.tasks)[0].completed_at).toBeNull();
  });

  it("removing the active task clears activeTaskId", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setActive(a);
    expect(get(s.activeTaskId)).toBe(a.id);
    await s.remove(a);
    expect(get(s.activeTaskId)).toBe(null);
    expect(get(s.tasks)).toHaveLength(0);
  });
});
