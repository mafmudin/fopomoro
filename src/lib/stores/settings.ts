import { writable, get } from "svelte/store";
import { api } from "../api";

export function createSettingsStore() {
  const opacity = writable<number>(0.9);

  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  async function load() {
    try {
      const s = await api.loadSettings();
      opacity.set(s.opacity);
    } catch (e) {
      console.error("[settings] load failed:", e);
    }
  }

  function setOpacity(value: number) {
    opacity.set(value);
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      api.saveSettings({ opacity: get(opacity) }).catch((e) =>
        console.error("[settings] save failed:", e)
      );
    }, 200); // debounce slider drags
  }

  return {
    opacity: { subscribe: opacity.subscribe },
    load, setOpacity,
  };
}

export type SettingsStore = ReturnType<typeof createSettingsStore>;
