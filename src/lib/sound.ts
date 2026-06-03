// Generates a short two-note chime via Web Audio — no binary asset required.
let ctx: AudioContext | null = null;

function context(): AudioContext {
  if (!ctx) ctx = new AudioContext();
  return ctx;
}

function beep(ac: AudioContext, freq: number, startAt: number, duration: number) {
  const osc = ac.createOscillator();
  const gain = ac.createGain();
  osc.type = "sine";
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0.0001, startAt);
  gain.gain.exponentialRampToValueAtTime(0.25, startAt + 0.02);
  gain.gain.exponentialRampToValueAtTime(0.0001, startAt + duration);
  osc.connect(gain).connect(ac.destination);
  osc.start(startAt);
  osc.stop(startAt + duration);
}

export function playChime() {
  const ac = context();
  if (ac.state === "suspended") ac.resume();
  const t = ac.currentTime;
  beep(ac, 880, t, 0.18);
  beep(ac, 1175, t + 0.18, 0.22);
}
