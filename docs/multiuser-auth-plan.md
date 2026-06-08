# [P0] Rencana: Isolasi Data per-User (Local-first, Cloud opt-in via Email OTP)

> **Prioritas: P0.** Dikerjakan duluan, sebelum P1 (Windows signing).
> **Tujuan:** menghilangkan kerancuan & lubang keamanan di mana semua user
> berbagi satu tabel `tasks` global yang sama.

## Masalah saat ini (terkonfirmasi dari kode)

App yang dipublish menyimpan task ke Supabase **tanpa identitas user**:

- `supabase.rs:68` — `get_tasks` = `SELECT * FROM tasks` **tanpa filter user** →
  setiap install yang sync menarik task **semua orang**.
- `supabase.rs:58-62` — semua request pakai **anon key yang sama** (di-bake ke
  binary, `supabase.rs:16-17`). Tidak ada identitas per-user.
- `models.rs:4-12` — `FoTask` tidak punya `user_id`; tabel tak mengenal pemilik.
- `commands.rs:63` — cloud selalu aktif kalau key ter-bake (`cfg = Some`).

Akibatnya **dua masalah**:

1. **Privasi/rancu** — task User A muncul di app User B; nomor `FO-NN` tabrakan.
2. **Keamanan** — anon key (bisa diekstrak dari binary) + tabel terbuka =
   siapa pun bisa baca/ubah/hapus task semua orang via REST langsung.

## Model yang disepakati: Local-first, Cloud opt-in

- **Tidak login (default)** → data **100% lokal** (`tasks.json`), **tidak
  menyentuh Supabase**. Privasi penuh, nol friction. Otomatis menutup masalah
  rancu (tabel global tak lagi dipakai anonim) **dan** lubang keamanan (anon key
  tak lagi dipakai untuk operasi data).
- **Login (opt-in, Email OTP)** → sync ke Supabase dengan **auth asli**:
  `user_id` + RLS, terisolasi & sinkron lintas device.

### Metode auth: Email OTP (6 digit)

Flow: user masukkan email → Supabase kirim kode 6 digit → user tempel ke app →
app verifikasi → dapat session (JWT). **Tanpa password, tanpa deep-link** →
paling cocok untuk Tauri. Auth dilakukan di **Rust** (konsisten dengan pola
Supabase yang sudah ada), via endpoint GoTrue REST:

- `POST /auth/v1/otp` `{email}` (header `apikey`) → kirim kode
- `POST /auth/v1/verify` `{email, token, type:"email"}` → `access_token` +
  `refresh_token` + `user`
- `POST /auth/v1/token?grant_type=refresh_token` → perpanjang session

---

## Tahapan kerja

### Tahap 0 — DB & RLS (sisi Supabase)

1. Tambah kolom `user_id uuid not null default auth.uid()` ke `tasks` dan
   `pomodoro_sessions`.
2. **Enable RLS** di kedua tabel; tulis policy `user_id = auth.uid()` untuk
   select/insert/update/delete.
3. **Hapus data global lama** (row peninggalan era anon — sudah tak terpakai).
4. Penomoran `FO-NN` jadi **per-user** (unik per `user_id`, bukan global).

### Tahap 1 — Auth core (Rust)

5. Modul `auth.rs`: `request_otp(email)`, `verify_otp(email, code) -> Session`,
   `refresh()`, `sign_out()`.
6. **Session store**: simpan `refresh_token`/`access_token`. *Rekomendasi:* OS
   keychain (mis. `tauri-plugin-stronghold`/keyring); minimal v1 boleh file di
   app data dir (catat trade-off keamanan). Load saat startup + auto-refresh.
7. `AppState` simpan `Option<Session>`; `supabase.rs` pakai **`Bearer
   <user_JWT>`** (anon key tetap di header `apikey` saja).

### Tahap 2 — Gating cloud vs local

8. Refactor `commands.rs`: jalur cloud **hanya** aktif kalau ada session login;
   tanpa login → murni lokal (jalur `tasks.json` yang sudah ada).
9. Pastikan anon key **tidak lagi** dipakai untuk operasi data (hanya endpoint
   auth) → menutup lubang keamanan.

### Tahap 3 — Migrasi data saat pertama login

10. **[default rekomendasi — bisa dikoreksi]** Saat user pertama kali login dan
    ada task lokal: **push task lokal ke cloud (merge)**, lalu adopsi state
    server. Alternatif: server-wins / tanya user. *Perlu konfirmasi final.*

### Tahap 4 — UI (Svelte)

11. Settings: tombol **"Sign in to sync"** → input email → input OTP → state
    "signed in as <email>" + tombol **Sign out**.
12. Indikator status sync / email yang login.
13. **Sign out** → kembali ke mode lokal (salinan lokal tetap dipertahankan).

### Tahap 5 — Verifikasi

14. Uji **dua akun**: task akun A tak terlihat oleh akun B.
15. Uji **RLS** menolak akses lintas user (coba akses langsung via REST).
16. Uji **mode lokal**: tanpa login app jalan penuh & offline; tak ada call cloud.

## Catatan & out-of-scope

- **Robust offline sync queue** (lihat `commands.rs:98-101`, limitation v1) tetap
  di luar scope kecuali diputuskan lain — fokus P0 = isolasi & auth, bukan
  konflik-resolution offline yang canggih.
- Data anonymous-era akan dibuang (Tahap 0.3); pastikan tak ada data penting di
  sana sebelum hapus.

## Status implementasi (live)

| Tahap | Status | Catatan |
|-------|--------|---------|
| 0 — DB & RLS | ✅ applied | `supabase/migrations/0001_per_user_isolation.sql` sudah di-run user |
| 1 — Auth core | ✅ kode | `src-tauri/src/auth.rs` (OTP, session store, refresh) |
| 2 — Gating cloud/local | ✅ kode | `commands.rs` lewat `auth::active_session`; data calls pakai user JWT |
| 3 — Migrasi login | ✅ kode | `src-tauri/src/sync.rs` (smart-merge: server-empty→push, else adopt) |
| 4 — UI | ✅ kode + ✅ frontend check | `Account.svelte`; `npm run check` 0 error, 24/24 test lulus |
| 5 — Verifikasi | ⬜ butuh build Rust + run | checklist di bawah |

> ⚠️ Sisi **Rust belum di-compile** (tak ada toolchain di WSL). Build via
> Rider/RustRover di Windows dulu sebelum verifikasi.

### Prasyarat verifikasi
- Build Rust sukses (`cargo build` / Rider).
- Supabase: Authentication → Providers → **Email enabled**, template OTP memuat
  `{{ .Token }}` (kode 6 digit, bukan hanya magic link).

### Checklist verifikasi (Tahap 5)
1. **Mode lokal (signed out):** jalankan app tanpa login → tambah/hapus task →
   pastikan jalan & tidak ada call ke Supabase (cek tabel tetap kosong).
2. **Login OTP:** Sign in to sync → masukkan email → tempel kode → "● synced".
3. **Smart-merge:** task lokal yang dibuat sebelum login muncul di cloud (cek
   tabel `tasks` → kolom `user_id` terisi).
4. **Isolasi 2 akun:** login akun B di instance/profil lain → task akun A TIDAK
   terlihat; sebaliknya juga.
5. **RLS langsung:** coba `GET /rest/v1/tasks` pakai anon key saja (tanpa JWT) →
   harus kosong/forbidden, bukan membocorkan data.
6. **Sign out:** kembali "local only"; salinan task lokal tetap ada.
7. **Persistensi:** restart app saat signed-in → tetap signed-in (session
   ke-load dari `auth_session.json`).

## Referensi

- Supabase Auth — Email OTP: https://supabase.com/docs/guides/auth/auth-email-passwordless
- Supabase — Row Level Security: https://supabase.com/docs/guides/database/postgres/row-level-security
- GoTrue REST (verify/otp/token): https://supabase.com/docs/reference/auth
