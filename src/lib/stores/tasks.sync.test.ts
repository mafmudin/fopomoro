import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";
import type { FoTask } from "../types";

const inserted: FoTask[] = [];
vi.mock("../api", () => ({
  api: {
    getTasks: vi.fn(async () => []),
    insertTask: vi.fn(async (title: string) => {
      const t: FoTask = { id: `id-${inserted.length + 1}`, task_id: `FO-0${inserted.length + 1}`,
        title, is_completed: false, created_at: "", completed_at: null, pomodoro_count: 0 };
      inserted.push(t); return t;
    }),
    updateTask: vi.fn(async () => {}),
    deleteTask: vi.fn(async () => {}),
    recordSession: vi.fn(async () => {}),
  },
}));
vi.mock("../sync", () => ({
  EVENTS: { tasksChanged: "tasks:changed", activeChanged: "task:active-changed", timerRunning: "timer:running-changed" },
  broadcast: vi.fn(async () => {}),
  subscribe: vi.fn(async () => () => {}),
}));

import { createTasksStore } from "./tasks";
import { broadcast } from "../sync";

beforeEach(() => { inserted.length = 0; vi.clearAllMocks(); });

describe("tasks store cross-window broadcasts", () => {
  it("broadcasts tasks:changed after add", async () => {
    const s = createTasksStore();
    await s.add("A");
    expect(broadcast).toHaveBeenCalledWith("tasks:changed");
  });

  it("broadcasts task:active-changed with the new id on setActive", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    s.setActive(a);
    expect(broadcast).toHaveBeenCalledWith("task:active-changed", a.id);
    s.setActive(a); // toggle off -> broadcasts null
    expect(broadcast).toHaveBeenCalledWith("task:active-changed", null);
  });

  it("applyActiveId force-sets the active id without broadcasting", async () => {
    const s = createTasksStore();
    await s.add("A");
    const [a] = get(s.tasks);
    (broadcast as ReturnType<typeof vi.fn>).mockClear();
    s.applyActiveId(a.id);
    expect(get(s.activeTaskId)).toBe(a.id);
    expect(broadcast).not.toHaveBeenCalled();
    s.applyActiveId(null);
    expect(get(s.activeTaskId)).toBe(null);
  });
});
