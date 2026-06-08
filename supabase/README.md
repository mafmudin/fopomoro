# Supabase — setup & migrasi

Skema & konfigurasi Supabase untuk FoPoMoro. Lihat rencana lengkap di
[`../docs/multiuser-auth-plan.md`](../docs/multiuser-auth-plan.md).

## Menjalankan migrasi

`migrations/0001_per_user_isolation.sql` menambah isolasi data per-user
(kolom `user_id`, RLS, penomoran `FO-NN` per-user).

**Cara termudah — SQL Editor:**

1. Buka Supabase Dashboard → project FoPoMoro → **SQL Editor**.
2. Tempel isi `migrations/0001_per_user_isolation.sql` → **Run**.

> ⚠️ Migrasi ini **DROP & recreate** tabel `tasks` + `pomodoro_sessions`. Data
> lama (era anon, tanpa `user_id`) akan hilang — ini disengaja.

**Alternatif — Supabase CLI:** `supabase db push` (perlu project ter-link).

## Konfigurasi Auth (untuk Email OTP) — Tahap berikutnya

Sebelum login OTP berfungsi, pastikan di Dashboard → **Authentication → Providers
→ Email**:

- **Email provider: enabled.**
- Aktifkan pengiriman **OTP** (kode 6 digit). Di template email, pastikan
  memuat token `{{ .Token }}` (kode), bukan hanya `{{ .ConfirmationURL }}` —
  app memakai kode, bukan magic link.
- (Dev) batas rate email default Supabase rendah; untuk produksi set custom SMTP.

## Setelah migrasi

Operasi data (`tasks`, `pomodoro_sessions`) hanya bisa diakses dengan **JWT user**
(`Authorization: Bearer <access_token>`). Anon key cuma dipakai untuk endpoint
`/auth/v1/*`. Verifikasi isolasi ada di task P0 #12.
