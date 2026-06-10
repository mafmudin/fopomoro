import { describe, it, expect } from "vitest";
import type { FoTask } from "./types";
import { incompleteTasks, completedTasks, topIncomplete, shouldShowSeeAll } from "./taskFilters";

function task(id: string, done: boolean): FoTask {
  return { id, task_id: id, title: id, is_completed: done, created_at: "", completed_at: done ? "x" : null, pomodoro_count: 0 };
}

describe("taskFilters", () => {
  const list = [task("a", false), task("b", true), task("c", false), task("d", false),
                task("e", false), task("f", false), task("g", false)]; // 6 incomplete, 1 done

  it("splits incomplete and completed", () => {
    expect(incompleteTasks(list).map((t) => t.id)).toEqual(["a", "c", "d", "e", "f", "g"]);
    expect(completedTasks(list).map((t) => t.id)).toEqual(["b"]);
  });

  it("topIncomplete returns at most n incomplete tasks in order", () => {
    expect(topIncomplete(list, 5).map((t) => t.id)).toEqual(["a", "c", "d", "e", "f"]);
    expect(topIncomplete([task("a", false)], 5)).toHaveLength(1);
  });

  it("shouldShowSeeAll is true when total > 5 OR any task is completed", () => {
    expect(shouldShowSeeAll(7, 1)).toBe(true);   // both
    expect(shouldShowSeeAll(3, 1)).toBe(true);    // has completed
    expect(shouldShowSeeAll(7, 0)).toBe(true);    // > 5
    expect(shouldShowSeeAll(5, 0)).toBe(false);   // <=5, none done
    expect(shouldShowSeeAll(0, 0)).toBe(false);
  });
});
