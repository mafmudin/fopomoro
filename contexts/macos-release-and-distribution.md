# Context — Rilis & Distribusi macOS

Catatan operasional FoPoMoro (Tauri + Svelte) untuk macOS. Fokus ke hal-hal
non-obvious yang sempat bikin kejebak — baca sebelum menggarap rilis/signing/theming.

---

## 1. Signing & Gatekeeper (penting)

FoPoMoro **tidak punya Apple Developer account** ($99/thn), jadi DMG release
**hanya ad-hoc signed**, tidak di-notarize.

- Ad-hoc signing diaktifkan via `src-tauri/tauri.conf.json` → `bundle.macOS.signingIdentity: "-"`.
  Tauri yang menjalankan `codesign -s -` saat bundling. **Nol secret/cert di CI.**
- Konsekuensi: **setiap orang yang download `.dmg` kena Gatekeeper** dan harus
  membersihkan quarantine sebelum bisa membuka:

  ```bash
  xattr -cr /Applications/FoPoMoro.app
  ```

  (Alternatif: klik-kanan app → Open → Open. Tapi di macOS 15+/Tahoe sering
  tidak muncul untuk pesan "damaged" — `xattr` lebih reliable.)

- Ad-hoc signing **hanya menurunkan** pesan dari `"…is damaged, move to Trash"`
  (buntu) ke `"unidentified developer"` (ada Open Anyway). **Tidak** menghilangkan
  langkah bypass — itu hanya bisa lewat **notarization**.
- Pesan "damaged" itu **menyesatkan**: app-nya tidak rusak, murni kombinasi
  unsigned/ad-hoc + atribut `com.apple.quarantine` yang otomatis nempel saat
  file di-download.

**Aturan praktis:** jangan janji "buka langsung tanpa warning". `release.yml`
(`releaseBody`) wajib selalu memuat instruksi `xattr` di atas. Kalau nanti dapat
Apple Dev account → upgrade ke Developer ID sign + notarize (env
`APPLE_CERTIFICATE` / `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID`) untuk
zero-warning.

---

## 2. Release workflow & versioning

`.github/workflows/release.yml` — manual `workflow_dispatch`, pilih bump
`patch`/`minor`/`major`. Alurnya:

1. Baca versi sekarang dari `src-tauri/tauri.conf.json`.
2. Bump versi, **commit balik ke `main`** sebagai `chore: release vX.Y.Z` lalu `git push`.
3. Build DMG (ad-hoc signed), buat tag `vX.Y.Z`, publish GitHub Release.

### ⚠️ Gotcha: local selalu jadi "behind" setelah rilis

Karena workflow push commit bump sendiri, **local `main` ketinggalan 1 commit**
tiap kali habis rilis. Kalau lanjut commit lokal tanpa pull:

- history **divergen** (`git status -sb` → `ahead N, behind 1`),
- versi lokal ketinggalan → bump berikutnya bisa menghasilkan **tag yang sudah ada**
  → step release **gagal**.

> Kejadian nyata 2026-06-04: lokal masih `0.1.0` sementara origin sudah `0.1.1`
> via commit `chore: release v0.1.1`. Patch dari `0.1.0` akan coba bikin `v0.1.1`
> yang sudah ada.

**SOP sebelum tiap rilis:**

```bash
git status -sb              # cek ahead/behind
git pull --rebase origin main   # sinkronkan versi & history
git push                   # push commit lokal
# baru trigger workflow: Actions → Release (macOS) → Run workflow → bump: patch
```

---

## 3. Theming panel & opacity

Panel mengambang transparan; ada **dua** lapis transparansi yang sering ketuker:

1. Window `transparent: true` (tauri.conf.json) → desktop kelihatan di sudut
   membulat. Jangan diubah.
2. Slider **Opacity** → `style="opacity: …"` di `.panel` (range `0.3`–`1`).
   Inilah **satu-satunya** kontrol translusensi.

### Gotcha format hex (pernah jadi bug)

`--panel-bg` **harus warna solid `#RRGGBB`** (mis. `#1E1E2E`), bukan 8-digit.
CSS membaca 8-digit hex sebagai **`#RRGGBBAA`**, bukan Android `#AARRGGBB`.
Bug lama: `--panel-bg: #CC1E1E2E` dikira alpha `CC`/80%, padahal CSS membacanya
sebagai warna `#CC1E1E` (merah gelap) dengan alpha `2E` ≈ 18% → panel nyaris
tembus pandang bahkan saat slider mentok kanan. **Selalu pakai `#RRGGBB` solid**
dan biarkan slider yang atur translusensi.

### Ganti warna background + auto-contrast

- Warna dipilih lewat 5 preset swatch + native color picker (`App.svelte`),
  disimpan sebagai `WindowSettings.bg_color` (`models.rs`, default `#1E1E2E`,
  pakai `#[serde(default)]` agar `settings.json` lama tetap kebaca).
- `src/lib/contrast.ts` → `textColorsFor(hex)` menghitung luminance lalu
  mengembalikan `{ text, subtext }`: teks terang di bg gelap, teks gelap di bg
  terang. Diterapkan sebagai inline CSS custom props (`--text`/`--subtext`) di
  `.panel` sehingga semua child ikut. Ada unit test `contrast.test.ts`.

---

## Source anchors

| Hal | File |
|-----|------|
| Signing config | `src-tauri/tauri.conf.json` → `bundle.macOS.signingIdentity` |
| Release pipeline | `.github/workflows/release.yml` |
| Settings model (persist) | `src-tauri/src/models.rs` → `WindowSettings` |
| Panel + theming UI | `src/App.svelte`, `src/theme.css` |
| Auto-contrast util | `src/lib/contrast.ts` (+ test) |
| Design spec | `docs/superpowers/specs/2026-06-04-background-color-and-opacity-fix-design.md` |
