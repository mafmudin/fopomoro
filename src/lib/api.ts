import { invoke } from "@tauri-apps/api/core";
import type { AuthStatus, FoTask, FoSession, PomodoroConfig, WindowSettings } from "./types";

export const api = {
  loadConfig: () => invoke<PomodoroConfig>("load_config"),
  saveConfig: (config: PomodoroConfig) => invoke<void>("save_config", { config }),

  loadSettings: () => invoke<WindowSettings>("load_settings"),
  saveSettings: (settings: WindowSettings) => invoke<void>("save_settings", { settings }),

  // Tasks, sessions, and progress:
  getTasks: () => invoke<FoTask[]>("get_tasks"),
  insertTask: (title: string) => invoke<FoTask>("insert_task", { title }),
  updateTask: (task: FoTask) => invoke<void>("update_task", { task }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
  recordSession: (taskId: string | null, durationMinutes: number, wasFocused: boolean) =>
    invoke<void>("record_session", { taskId, durationMinutes, wasFocused }),
  loadProgress: () => invoke<FoSession>("load_progress"),
  saveProgress: (session: FoSession) => invoke<void>("save_progress", { session }),

  // Auth (Email OTP). Cloud sync is opt-in: signed out ⇒ everything stays local.
  authStatus: () => invoke<AuthStatus>("auth_status"),
  authRequestOtp: (email: string) => invoke<void>("auth_request_otp", { email }),
  authVerifyOtp: (email: string, code: string) =>
    invoke<AuthStatus>("auth_verify_otp", { email, code }),
  authSignOut: () => invoke<void>("auth_sign_out"),
};
