import { describe, it, expect } from "vitest";
import { get } from "svelte/store";
import { formatClockTime, formatClockDate, createPomodoro } from "./timer";

describe("clock formatting", () => {
  it("formats time as HH:mm:ss with zero padding", () => {
    const d = new Date(2026, 5, 3, 9, 7, 4); // 2026-06-03 09:07:04
    expect(formatClockTime(d)).toBe("09:07:04");
  });

  it("formats date as 'Weekday, DD Month YYYY'", () => {
    const d = new Date(2026, 5, 3, 9, 7, 4); // Wednesday, 03 June 2026
    expect(formatClockDate(d)).toBe("Wednesday, 03 June 2026");
  });

  it("zero-pads midnight to 00:00:00", () => {
    const d = new Date(2026, 5, 3, 0, 0, 0);
    expect(formatClockTime(d)).toBe("00:00:00");
  });
});

describe("pomodoro state machine", () => {
  const cfg = { focus_minutes: 25, short_break_minutes: 5, long_break_minutes: 15 };

  it("starts Idle showing focus duration", () => {
    const p = createPomodoro(cfg);
    const s = get(p.state);
    expect(s.label).toBe("Ready");
    expect(s.remainingSeconds).toBe(25 * 60);
    expect(s.isRunning).toBe(false);
  });

  it("Start moves Idle -> Focus and runs", () => {
    const p = createPomodoro(cfg);
    p.start();
    const s = get(p.state);
    expect(s.label).toBe("Focus");
    expect(s.isRunning).toBe(true);
    p.dispose();
  });

  it("focus completion 1-3 goes to Short Break, increments dots, stops, fires focus event", () => {
    const p = createPomodoro(cfg);
    const events: Array<{ minutes: number; wasFocus: boolean }> = [];
    p.onSessionComplete((minutes, wasFocus) => events.push({ minutes, wasFocus }));
    p.start();
    p._completeForTest();
    const s = get(p.state);
    expect(s.label).toBe("Short Break");
    expect(s.completedSessions).toBe(1);
    expect(s.isRunning).toBe(false);
    expect(events).toEqual([{ minutes: 25, wasFocus: true }]);
  });

  it("4th focus completion goes to Long Break and resets dots to 0", () => {
    const p = createPomodoro(cfg);
    for (let i = 0; i < 3; i++) {
      p.start(); p._completeForTest(); // focus -> short break
      p.start(); p._completeForTest(); // short break -> focus
    }
    p.start(); p._completeForTest(); // 4th focus
    const s = get(p.state);
    expect(s.label).toBe("Long Break");
    expect(s.completedSessions).toBe(0);
  });

  it("break completion fires a non-focus event and returns to Focus", () => {
    const p = createPomodoro(cfg);
    const events: Array<{ minutes: number; wasFocus: boolean }> = [];
    p.onSessionComplete((minutes, wasFocus) => events.push({ minutes, wasFocus }));
    p.start(); p._completeForTest();   // focus -> short break (event focus)
    p.start(); p._completeForTest();   // short break -> focus (event break)
    expect(events[1]).toEqual({ minutes: 5, wasFocus: false });
    expect(get(p.state).label).toBe("Focus");
  });

  it("applyConfig validates positive integers and updates remaining when idle", () => {
    const p = createPomodoro(cfg);
    expect(p.applyConfig("30", "5", "15")).toBe(true);
    expect(get(p.state).remainingSeconds).toBe(30 * 60);
    expect(p.applyConfig("0", "5", "15")).toBe(false);
    expect(p.applyConfig("abc", "5", "15")).toBe(false);
  });

  it("pause preserves the remaining time and stops running", () => {
    const p = createPomodoro(cfg);
    p.start();
    p.pause();
    const s = get(p.state);
    expect(s.remainingSeconds).toBe(25 * 60);
    expect(s.isRunning).toBe(false);
  });

  it("reset restores the current state's full duration and stops running", () => {
    const p = createPomodoro(cfg);
    p.start();
    p.reset();
    const s = get(p.state);
    expect(s.remainingSeconds).toBe(25 * 60);
    expect(s.isRunning).toBe(false);
  });
});
