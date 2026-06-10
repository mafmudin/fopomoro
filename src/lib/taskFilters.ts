import type { FoTask } from "./types";

export function incompleteTasks(tasks: FoTask[]): FoTask[] {
  return tasks.filter((t) => !t.is_completed);
}

export function completedTasks(tasks: FoTask[]): FoTask[] {
  return tasks.filter((t) => t.is_completed);
}

export function topIncomplete(tasks: FoTask[], n: number): FoTask[] {
  return incompleteTasks(tasks).slice(0, n);
}

export function shouldShowSeeAll(total: number, completedCount: number): boolean {
  return total > 5 || completedCount > 0;
}
