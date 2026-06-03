# FoPoMoro (macOS)

A semi-transparent, always-on-top floating overlay for productivity and study
tracking — a Tauri + Svelte port of the Windows FoPoMoro app. Clock, Pomodoro
timer, task list, daily progress stats, and Supabase cloud sync (shared backend
with the Windows version).

## Requirements

- Rust toolchain, Node.js, Xcode Command Line Tools.

## Setup

```bash
npm install
cp .env.example .env   # optional: fill in for Supabase sync; omit to run offline
```

`.env`:

```
SUPABASE_URL=https://your-project-id.supabase.co
SUPABASE_ANON_KEY=your-anon-key-here
```

When `.env` is present and complete, Supabase is the source of truth for tasks.
Without it, the app runs offline using local JSON. Progress and settings are
always local (`~/Library/Application Support/com.fopomoro.mac/`).

## Develop

```bash
npm run tauri dev
```

## Build

```bash
npm run tauri build   # produces a .app / .dmg under src-tauri/target/release/bundle
```

## Test

```bash
npm run test                      # Svelte store logic (Vitest)
cd src-tauri && cargo test        # Rust storage / mapping
```

## Notes / Known limitations (v1)

- No code signing / notarization (dev-local run).
- Transparency requires `macOSPrivateApi: true` (disables Mac App Store submission).
- The window is draggable by its title bar ("FoPoMoro"); position is remembered across launches.
