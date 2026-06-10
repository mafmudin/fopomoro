import { describe, it, expect, vi } from "vitest";

// Simulate "not in Tauri": emit/listen throw.
vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn(() => { throw new Error("no tauri"); }),
  listen: vi.fn(() => { throw new Error("no tauri"); }),
}));

import { broadcast, subscribe, EVENTS } from "./sync";

describe("sync wrappers", () => {
  it("broadcast resolves (swallows errors) outside Tauri", async () => {
    await expect(broadcast(EVENTS.tasksChanged)).resolves.toBeUndefined();
  });

  it("subscribe resolves to a no-op unlisten outside Tauri", async () => {
    const un = await subscribe(EVENTS.tasksChanged, () => {});
    expect(typeof un).toBe("function");
    expect(() => un()).not.toThrow();
  });

  it("exposes the three event names", () => {
    expect(EVENTS).toEqual({
      tasksChanged: "tasks:changed",
      activeChanged: "task:active-changed",
      timerRunning: "timer:running-changed",
    });
  });
});
