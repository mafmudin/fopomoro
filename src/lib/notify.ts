import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

export async function ensureNotificationPermission(): Promise<boolean> {
  let granted = await isPermissionGranted();
  if (!granted) {
    const res = await requestPermission();
    granted = res === "granted";
  }
  return granted;
}

export async function notify(title: string, body: string) {
  if (await ensureNotificationPermission()) {
    sendNotification({ title, body });
  }
}
