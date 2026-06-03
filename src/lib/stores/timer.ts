import { readable, writable, type Writable } from "svelte/store";
import type { PomodoroConfig } from "../types";

const MONTHS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];
const WEEKDAYS = [
  "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
];

const pad2 = (n: number) => n.toString().padStart(2, "0");

export function formatClockTime(d: Date): string {
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
}

export function formatClockDate(d: Date): string {
  return `${WEEKDAYS[d.getDay()]}, ${pad2(d.getDate())} ${MONTHS[d.getMonth()]} ${d.getFullYear()}`;
}

export interface ClockState {
  time: string;
  date: string;
}

function clockState(now: Date): ClockState {
  return { time: formatClockTime(now), date: formatClockDate(now) };
}

// Ticks every second; first value is emitted immediately.
export const clock = readable<ClockState>(clockState(new Date()), (set) => {
  const tick = () => set(clockState(new Date()));
  tick();
  const id = setInterval(tick, 1000);
  return () => clearInterval(id);
});

export type PomodoroLabel = "Ready" | "Focus" | "Short Break" | "Long Break";

export interface PomodoroState {
  label: PomodoroLabel;
  remainingSeconds: number;
  completedSessions: number; // 0..3, drives the 4 dots (resets to 0 on the 4th focus → long break, matching WPF)
  isRunning: boolean;
  timeDisplay: string; // mm:ss
}

const SESSIONS_BEFORE_LONG_BREAK = 4;
type InternalState = "Idle" | "Focus" | "ShortBreak" | "LongBreak";
type SessionListener = (minutes: number, wasFocus: boolean) => void;

const mmss = (totalSeconds: number) => {
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  return `${pad2(m)}:${pad2(s)}`;
};

const labelFor = (st: InternalState): PomodoroLabel => {
  switch (st) {
    case "Focus": return "Focus";
    case "ShortBreak": return "Short Break";
    case "LongBreak": return "Long Break";
    default: return "Ready";
  }
};

export function createPomodoro(initial: PomodoroConfig) {
  let focusMinutes = Math.max(1, initial.focus_minutes);
  let shortBreakMinutes = Math.max(1, initial.short_break_minutes);
  let longBreakMinutes = Math.max(1, initial.long_break_minutes);

  let st: InternalState = "Idle";
  let remaining = focusMinutes * 60;
  let completed = 0;
  let running = false;
  let activeFocusMinutes = focusMinutes;
  let intervalId: ReturnType<typeof setInterval> | null = null;

  const listeners: SessionListener[] = [];
  const state: Writable<PomodoroState> = writable(snapshot());

  function snapshot(): PomodoroState {
    return {
      label: labelFor(st),
      remainingSeconds: remaining,
      completedSessions: completed,
      isRunning: running,
      timeDisplay: mmss(remaining),
    };
  }
  function publish() { state.set(snapshot()); }

  function durationFor(s: InternalState): number {
    switch (s) {
      case "Focus": return focusMinutes * 60;
      case "ShortBreak": return shortBreakMinutes * 60;
      case "LongBreak": return longBreakMinutes * 60;
      default: return focusMinutes * 60;
    }
  }

  function stopInterval() {
    if (intervalId !== null) { clearInterval(intervalId); intervalId = null; }
  }

  function transitionTo(next: InternalState) {
    st = next;
    remaining = durationFor(next);
    if (next === "Focus") activeFocusMinutes = focusMinutes;
    running = false; // timer stops at each transition (parity: user presses Start)
    stopInterval();
    publish();
  }

  function emit(minutes: number, wasFocus: boolean) {
    for (const l of listeners) l(minutes, wasFocus);
  }

  function handleSessionComplete() {
    // transitionTo() (called in every branch below) stops the interval and
    // clears `running`, so no need to do it here too.
    if (st === "Focus") {
      const newCount = completed + 1;
      if (newCount >= SESSIONS_BEFORE_LONG_BREAK) {
        completed = 0;
        emit(activeFocusMinutes, true);
        transitionTo("LongBreak");
      } else {
        completed = newCount;
        emit(activeFocusMinutes, true);
        transitionTo("ShortBreak");
      }
    } else {
      const breakMinutes = st === "ShortBreak" ? shortBreakMinutes : longBreakMinutes;
      emit(breakMinutes, false);
      transitionTo("Focus");
    }
  }

  function tick() {
    remaining -= 1;
    if (remaining <= 0) { handleSessionComplete(); return; }
    publish();
  }

  function start() {
    if (st === "Idle") {
      st = "Focus";
      remaining = focusMinutes * 60;
      activeFocusMinutes = focusMinutes;
    }
    if (running) return;
    running = true;
    stopInterval();
    intervalId = setInterval(tick, 1000);
    publish();
  }

  function pause() {
    stopInterval();
    running = false;
    publish();
  }

  function reset() {
    stopInterval();
    running = false;
    remaining = durationFor(st === "Idle" ? "Focus" : st);
    publish();
  }

  function applyConfig(focusText: string, shortText: string, longText: string): boolean {
    const f = Number(focusText), s = Number(shortText), l = Number(longText);
    const valid = (n: number) => Number.isInteger(n) && n > 0;
    if (!valid(f) || !valid(s) || !valid(l)) return false;
    focusMinutes = f; shortBreakMinutes = s; longBreakMinutes = l;
    if (!running) {
      remaining = durationFor(st === "Idle" ? "Focus" : st);
      if (st === "Focus" || st === "Idle") activeFocusMinutes = focusMinutes;
      publish();
    }
    return true;
  }

  function getConfig(): PomodoroConfig {
    return {
      focus_minutes: focusMinutes,
      short_break_minutes: shortBreakMinutes,
      long_break_minutes: longBreakMinutes,
    };
  }

  function onSessionComplete(cb: SessionListener) { listeners.push(cb); }

  function dispose() { stopInterval(); }

  return {
    state: { subscribe: state.subscribe },
    start, pause, reset, applyConfig, getConfig, onSessionComplete, dispose,
    // test-only hook to simulate the countdown hitting zero without waiting:
    _completeForTest: () => { handleSessionComplete(); },
  };
}

export type Pomodoro = ReturnType<typeof createPomodoro>;
