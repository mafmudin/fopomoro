# Background Color + Opacity Fix — Design

Date: 2026-06-04

## Problem

Two related issues with the floating panel's appearance:

1. **Opacity slider maxed out still looks transparent.** Root cause: in `src/theme.css`,
   `--panel-bg: #CC1E1E2E` was written in Android `#AARRGGBB` form (intended: alpha `CC`/80%,
   color `#1E1E2E`). CSS parses 8-digit hex as `#RRGGBBAA`, so it resolves to color `#CC1E1E`
   (a dark red) with alpha `0x2E` ≈ 18%. The panel fill is only ~18% opaque, so even at
   `opacity: 1` on `.panel` the panel is nearly see-through.

2. **No way to change the background color.** User wants to pick the panel background.

## Goals

- Background color is fully solid at opacity = 1 (slider alone controls translucency).
- User can pick the background color via preset swatches *and* a free native color picker.
- Text stays readable on any chosen color (auto-contrast).
- Existing saved `opacity` is preserved across the upgrade.

## Design

### 1. Opacity fix (`src/theme.css`)
`--panel-bg` becomes a **solid** color (default `#1E1E2E`, no alpha). All chosen colors are
6-digit hex (`#rrggbb`), so there is no alpha-format ambiguity. The `.panel` element `opacity`
(driven by the slider) is the sole translucency control; at `1.0` the panel is fully opaque.

### 2. Persistence (Rust + TS)
- `WindowSettings` (`src-tauri/src/models.rs`): add `pub bg_color: String`, default `"#1E1E2E"`.
- Field carries `#[serde(default = "default_bg_color")]` so an existing `settings.json` that only
  contains `opacity` still deserializes — **the saved opacity is not reset**.
- `Default for WindowSettings` sets `bg_color: "#1E1E2E".into()`.
- TS `WindowSettings` (`src/lib/types.ts`): add `bg_color: string`.
- Settings store (`src/lib/stores/settings.ts`): add `bgColor` writable + `setBgColor(value)`.
  `load()` reads both; the debounced save sends `{ opacity, bg_color }` together (the Rust
  command takes the whole struct).

### 3. UI (`src/App.svelte`)
Settings area at the bottom of the panel:

```
Background   [■][■][■][■][■]  [color picker]
Opacity      ──────●──
```

- 5 preset swatches (curated dark Catppuccin):
  `#1E1E2E` base · `#181825` mantle · `#11111B` crust · `#24273A` macchiato · `#303446` frappé.
- Native `<input type="color">` for free choice.
- Clicking a swatch or changing the picker calls `settings.setBgColor(...)` → applies immediately
  + debounced save (same pattern as the opacity slider).

### 4. Auto-contrast text (`src/lib/contrast.ts`, new)
Pure util `textColorsFor(hex)` computes relative luminance and returns `{ text, subtext }`:
- dark bg → light Catppuccin text (`#CDD6F4` / `#BAC2DE`)
- light bg → dark text (`#1E1E2E` / `#45475A`)

Applied as inline CSS custom properties on `.panel` so descendants inherit:

```svelte
<main class="panel" style="--panel-bg:{$bgColor}; --text:{txt}; --subtext:{sub}; opacity:{$opacity}">
```

Accent / surface / button tokens are unchanged (they read fine on any panel background).

### 5. Testing
`src/lib/contrast.test.ts` (vitest, already configured): dark bg → light text, light bg → dark
text, and a boundary case. Pure function, no mocks needed.

## Files touched
- `src/theme.css` — solid `--panel-bg`
- `src-tauri/src/models.rs` — `bg_color` field + serde default
- `src/lib/types.ts` — `bg_color`
- `src/lib/stores/settings.ts` — `bgColor` + `setBgColor`
- `src/App.svelte` — swatches + picker, apply contrast vars
- `src/lib/contrast.ts` (new) + `src/lib/contrast.test.ts` (new)

## Out of scope (YAGNI)
- Per-component theming beyond `--text`/`--subtext`.
- Background alpha/gradient/image. Slider already covers translucency.
- Light/dark accent recoloring.
