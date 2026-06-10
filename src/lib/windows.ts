import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

const ALL_TASKS_LABEL = "all-tasks";

/** Open the All Tasks window, or focus it if it's already open (no duplicates). */
export async function openAllTasksWindow(): Promise<void> {
  const existing = await WebviewWindow.getByLabel(ALL_TASKS_LABEL);
  if (existing) {
    await existing.setFocus();
    return;
  }
  const win = new WebviewWindow(ALL_TASKS_LABEL, {
    url: "index.html",
    title: "FoPoMoro — All Tasks",
    width: 520,
    height: 640,
    resizable: true,
    decorations: true,
    alwaysOnTop: false,
  });
  win.once("tauri://error", (e) => console.error("[windows] all-tasks create failed:", e));
}
