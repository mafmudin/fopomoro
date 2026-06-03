import { writable, get } from "svelte/store";
import { api } from "../api";

export function createSettingsStore() {
  const opacity = writable<number>(0.9);
  // Runtime-only — not persisted; resets to off on each launch (by design).
  const clickThrough = writable<boolean>(false);

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

  async function toggleClickThrough() {
    const next = !get(clickThrough);
    clickThrough.set(next);
    try {
      await api.setClickThrough(next);
    } catch (e) {
      console.error("[settings] setClickThrough failed:", e);
      clickThrough.set(!next); // revert on failure
    }
  }

  return {
    opacity: { subscribe: opacity.subscribe },
    clickThrough: { subscribe: clickThrough.subscribe },
    load, setOpacity, toggleClickThrough,
  };
}

export type SettingsStore = ReturnType<typeof createSettingsStore>;
