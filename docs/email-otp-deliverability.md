# Email OTP + Deliverability — Playbook (Supabase + Tauri + Resend)

Pelajaran dari sesi verifikasi P0 (2026-06-08). Reusable untuk project mana pun
yang pakai Supabase Email OTP pada app desktop (Tauri) + domain sendiri.

## 1. Alur auth: verify-token, BUKAN magic link

App desktop tidak bisa pakai magic link (link redirect ke `localhost:3000` →
`otp_expired`/`access_denied`). Pakai alur **kode 6–8 digit**:
`POST /auth/v1/otp {email, create_user:true}` → user tempel kode → `POST
/auth/v1/verify {type:"email", email, token}` → session.

## 2. Template email: yang menentukan tergantung user BARU vs LAMA

- **Email belum terdaftar (pertama OTP)** → Supabase pakai template **"Confirm signup"**.
- **Email sudah terdaftar** → pakai template **"Magic Link"**.

Karena `create_user:true`, percobaan pertama selalu "Confirm signup". **Kedua
template HARUS memuat `{{ .Token }}`** (merender kode), kalau tidak email cuma
berisi link. Gejala klasik: "sudah edit Magic Link tapi tetap link" = yang
terpakai sebenarnya "Confirm signup".

## 3. Panjang OTP bisa 6–10 digit (configurable di Supabase)

Project ini menghasilkan **8 digit**. Jangan hardcode "6-digit" di UI input —
buat netral ("Verification code"). Sisi Rust kirim token apa adanya, tak peduli
panjang. Setting: Authentication → "Email OTP Length".

## 4. Deliverability: Gmail wajib SPF atau DKIM (2024+), plus DMARC

Bounce `550-5.7.26 ... sender is unauthenticated` = domain belum
terotentikasi. Butuh **SPF + DKIM + DMARC** valid di DNS.

### Shared hosting (cPanel) sering GAGAL untuk ini
- IP keluar **rotasi** (mis. `103.89.154.65/.66`) lewat relay provider
  (BiznetGio/Proxmox) → SPF (daftar IP terbatas) gagal terus.
- Relay **tidak menandatangani DKIM** dgn selector domain → DKIM "did not pass".
- DKIM lewat relay shared hosting praktis **tak bisa diperbaiki** dari sisi user.

### Solusi: pakai Resend (atau ESP sejenis)
Resend tanda-tangani DKIM benar + kirim dari IP SES terotorisasi → SPF+DKIM lolos.
SMTP Supabase:
- Host `smtp.resend.com`, Port `465`
- **Username `resend`** (literal! BUKAN alamat email — jebakan paling sering,
  gejalanya 500 `unexpected_failure` di Supabase)
- Password = API key `re_...` (scope Sending/Full, jangan di-restrict ke domain salah)
- Sender = alamat di domain yang **Verified** di Resend (mis. `noreply@domain`)

Sandbox `onboarding@resend.dev` (tanpa verifikasi domain) **hanya** bisa kirim ke
email pendaftar akun Resend → cukup buat tes kode, TIDAK cukup untuk rilis (user
nyata butuh domain verified).

### Verifikasi DNS tanpa dig/nslookup (mis. di WSL yg minim tool)
Pakai DNS-over-HTTPS Google (resolver yang dipakai Gmail):
```bash
curl -s "https://dns.google/resolve?name=default._domainkey.DOMAIN&type=TXT"
curl -s "https://dns.google/resolve?name=DOMAIN&type=TXT"          # SPF
curl -s "https://dns.google/resolve?name=_dmarc.DOMAIN&type=TXT"    # DMARC
```

### DNS dikelola di mana?
cPanel mungkin bukan authoritative (cek nameserver). Di sini NS = DomaiNesia →
record SPF/DKIM/DMARC dipasang di panel DomaiNesia, bukan cPanel. Saat re-add
domain di Resend, **DKIM key di-generate baru** → update record lama agar persis.

## 5. Rate limit email built-in Supabase

Layanan email bawaan Supabase sangat dibatasi (beberapa/jam, ada batas keras).
429 `over_email_send_rate_limit` saat testing itu normal. Custom SMTP (Resend/
cPanel) melepas batas ini. Jangan klik "Send" berulang.

## 6. Isolasi diagnosa (pola yang berhasil)

Saat error berlapis, isolasi tiap lapis:
- API key Resend → tes langsung `POST https://api.resend.com/emails` (lepas dari Supabase).
- Kredensial SMTP → "Send test email" di Supabase / cek Auth Logs untuk pesan SMTP asli.
- DNS → DoH Google.
Tiap tes menyingkirkan satu kemungkinan, jadi tahu pasti lapis mana yang salah.
