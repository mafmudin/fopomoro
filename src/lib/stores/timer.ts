import { readable } from "svelte/store";

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
