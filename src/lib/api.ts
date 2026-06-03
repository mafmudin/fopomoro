import { invoke } from "@tauri-apps/api/core";
import type { FoTask, FoSession, PomodoroConfig, WindowSettings } from "./types";

export const api = {
  loadConfig: () => invoke<PomodoroConfig>("load_config"),
  saveConfig: (config: PomodoroConfig) => invoke<void>("save_config", { config }),

  loadSettings: () => invoke<WindowSettings>("load_settings"),
  saveSettings: (settings: WindowSettings) => invoke<void>("save_settings", { settings }),

  setClickThrough: (enabled: boolean) => invoke<void>("set_click_through", { enabled }),

  // Implemented in later tasks (commands not registered yet — do not call until then):
  getTasks: () => invoke<FoTask[]>("get_tasks"),
  insertTask: (title: string) => invoke<FoTask>("insert_task", { title }),
  updateTask: (task: FoTask) => invoke<void>("update_task", { task }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
  recordSession: (taskId: string | null, durationMinutes: number, wasFocused: boolean) =>
    invoke<void>("record_session", { taskId, durationMinutes, wasFocused }),
  loadProgress: () => invoke<FoSession>("load_progress"),
  saveProgress: (session: FoSession) => invoke<void>("save_progress", { session }),
};
