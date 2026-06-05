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

### Release (CI)

A GitHub Actions workflow (`.github/workflows/release.yml`) publishes releases.
Trigger it manually: **Actions → Release → Run workflow**, then pick a version
bump (patch/minor/major). It bumps the version, commits it to `main`, then builds
**macOS (.dmg)** and **Windows (.msi/.exe)** in parallel and attaches both to one
tagged GitHub Release. Supabase creds come from the `SUPABASE` GitHub Environment
secrets and are baked into the binary at compile time.

> After a CI release, run `git pull` — the workflow commits a version bump to `main`.

## Test

```bash
npm run test                      # Svelte store logic (Vitest)
cd src-tauri && cargo test        # Rust storage / mapping
```

## Notes / Known limitations (v1)

- No code signing / notarization. macOS: ad-hoc signed → on first launch run `xattr -cr /Applications/FoPoMoro.app`. Windows: unsigned → SmartScreen "More info → Run anyway".
- Transparency requires `macOSPrivateApi: true` (disables Mac App Store submission). Windows uses WebView2 — the transparent/rounded overlay may render less cleanly than macOS, so verify the Windows build before relying on it.
- The window is draggable by its title bar ("FoPoMoro"); position is remembered across launches.

## License

Released under the [MIT License](LICENSE).
