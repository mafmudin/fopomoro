import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";

export const EVENTS = {
  tasksChanged: "tasks:changed",
  activeChanged: "task:active-changed",
  timerRunning: "timer:running-changed",
} as const;

/** Emit a global event to all windows. No-op (best effort) outside Tauri. */
export async function broadcast(event: string, payload?: unknown): Promise<void> {
  try {
    await emit(event, payload);
  } catch {
    // not running inside Tauri (e.g. unit tests) — ignore
  }
}

/** Listen for a global event; returns an unlisten fn (no-op outside Tauri). */
export async function subscribe(
  event: string,
  handler: (payload: unknown) => void,
): Promise<UnlistenFn> {
  try {
    return await listen(event, (e) => handler(e.payload));
  } catch {
    return () => {};
  }
}
