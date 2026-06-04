import { writable, get } from "svelte/store";
import { api } from "../api";

const DEFAULT_BG = "#1E1E2E";

export function createSettingsStore() {
  const opacity = writable<number>(0.9);
  const bgColor = writable<string>(DEFAULT_BG);

  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  async function load() {
    try {
      const s = await api.loadSettings();
      opacity.set(s.opacity);
      bgColor.set(s.bg_color ?? DEFAULT_BG);
    } catch (e) {
      console.error("[settings] load failed:", e);
    }
  }

  // Both controls persist the whole settings object together, debounced so
  // slider drags and color-picker scrubbing don't spam the backend.
  function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      api.saveSettings({ opacity: get(opacity), bg_color: get(bgColor) }).catch((e) =>
        console.error("[settings] save failed:", e)
      );
    }, 200);
  }

  function setOpacity(value: number) {
    opacity.set(value);
    scheduleSave();
  }

  function setBgColor(value: string) {
    bgColor.set(value);
    scheduleSave();
  }

  return {
    opacity: { subscribe: opacity.subscribe },
    bgColor: { subscribe: bgColor.subscribe },
    load, setOpacity, setBgColor,
  };
}

export type SettingsStore = ReturnType<typeof createSettingsStore>;
