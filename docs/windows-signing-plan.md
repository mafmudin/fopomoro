# [P1] Rencana: Windows Code Signing via SignPath OSS

> **Prioritas: P1.** Dikerjakan setelah P0 (rencana lain, akan didiskusikan).
> **Tujuan:** menghilangkan dialog "Unknown Publisher" pada installer Windows
> (`.msi` / `-setup.exe`) dengan menandatangani build secara otomatis lewat
> program gratis **SignPath Foundation untuk Open Source**.

## Latar belakang singkat

Build Windows saat ini **unsigned**, sehingga SmartScreen + UAC selalu warning
(lihat `releaseBody` di `.github/workflows/release.yml`). Kita pilih jalur
**gratis** (SignPath OSS, sertifikat **OV**), bukan berbayar (Azure Trusted
Signing / EV).

Konsekuensi yang sudah disepakati:

- **Bukan instan.** Karena sertifikat OV, "Unknown Publisher" langsung hilang,
  tapi reputasi SmartScreen terbangun seiring jumlah download.
- **1 klik approval per rilis.** Setiap signing request wajib di-approve manual
  di dashboard SignPath (syarat Foundation, tidak bisa di-bypass). Selebihnya
  otomatis — **tidak ada upload file manual**.

## Arsitektur teknis

Model SignPath = *submit artifact → sign di cloud → download balik*, jadi
`signCommand` Tauri **tidak dipakai**. Job Windows di workflow dipecah:

```
build Tauri (unsigned .msi/.exe)
  └─ upload-artifact (biar artifact ada di server GitHub)
       └─ SignPath/github-action-submit-signing-request@v1 (wait-for-completion: true)
            ├─ kirim email "perlu approval" ke approver
            ├─ [MANUAL] approver klik Approve di dashboard SignPath  ← satu-satunya kerja manual
            └─ action otomatis download artifact tersigning
                 └─ upload artifact tersigning ke GitHub Release
```

macOS dibiarkan apa adanya (tetap ad-hoc signed, di luar scope ini).

## Prasyarat kelayakan (status saat ini)

| Syarat | Status |
|--------|--------|
| Lisensi OSI-approved tanpa dual-licensing komersial | ✅ MIT |
| Repo publik di GitHub | ✅ `github.com/mafmudin/fopomoro` |
| App sudah dirilis dalam bentuk yang akan di-sign | ✅ v1.0.0 |
| MFA aktif di GitHub untuk semua maintainer | ⬜ perlu dicek/diaktifkan |
| Code signing policy dipublish di halaman project | ⬜ akan dibuat (Tahap 1) |
| Build artifact dari source secara verifiable | ✅ via GitHub Actions |

---

## Tahapan kerja

### Tahap 1 — Persiapan teknis (bisa dikerjakan kapan saja, tanpa nunggu approval)

1. **Buat code signing policy** dan publish di `README.md` (section "Code Signing")
   — definisikan role author / reviewer / approver. *Syarat wajib aplikasi.*
2. **Rombak `.github/workflows/release.yml`** untuk job Windows:
   - build Tauri tanpa langsung membuat release untuk Windows;
   - `actions/upload-artifact@v4` untuk artifact `.msi` + `-setup.exe`;
   - `SignPath/github-action-submit-signing-request@v1` dengan
     `wait-for-completion: true` + `output-artifact-directory`;
   - upload artifact tersigning ke Release yang sama (mis. `gh release upload`
     atau `softprops/action-gh-release`);
   - set `timeout-minutes` job yang wajar (mis. 60) untuk menunggu approval.
3. **Update `releaseBody`** — hapus kata "unsigned" untuk Windows, ganti dengan
   catatan "signed (SmartScreen reputation building)".
4. **(Opsional) Tambah `bundle.windows`** di `tauri.conf.json` bila perlu
   penyesuaian metadata installer.

### Tahap 2 — Aplikasi ke SignPath Foundation (kerja USER, ~hari–minggu)

5. Pastikan **MFA aktif** untuk semua maintainer di GitHub.
6. Submit aplikasi di **https://signpath.org/terms** (sertakan link repo,
   lisensi, dan code signing policy dari Tahap 1).
7. Tunggu review manual SignPath.

### Tahap 3 — Aktivasi setelah approval (kerja USER, lalu verifikasi bareng)

8. Setelah disetujui, ambil dari dashboard SignPath: `organization-id`,
   `project-slug`, `signing-policy-slug`, dan **API token**.
9. Tambahkan ke **GitHub Environment `SUPABASE`** (atau environment baru) sebagai
   secrets: `SIGNPATH_API_TOKEN` (sisanya boleh hardcode non-rahasia di workflow).
10. **Jalankan rilis uji** (`patch`) → approve di SignPath saat email masuk →
    pastikan artifact tersigning muncul di Release.
11. **Verifikasi**: download `.exe`, cek properties → tab Digital Signatures
    harus menampilkan signer; pastikan dialog "Unknown Publisher" hilang.

## Deliverable Tahap 1 (yang akan dibuat saat eksekusi)

- `README.md` — section Code Signing Policy
- `.github/workflows/release.yml` — job Windows yang sudah disisipi signing
- `docs/signpath-application-checklist.md` — checklist langkah pendaftaran user

## Catatan & risiko

- Job Windows akan **blocking menunggu approval** → makan menit GitHub Actions
  selama menunggu. Mitigasi: `timeout-minutes` wajar + approve segera.
- Reputasi SmartScreen **tidak instan** — kelola ekspektasi di release notes.
- Approval manual **tidak bisa dihilangkan** di tier gratis.
- Webhook SignPath tersedia tapi **tidak diperlukan** (ditangani oleh
  `wait-for-completion`).

## Referensi

- SignPath Foundation — syarat OSS: https://signpath.org/terms.html
- SignPath GitHub Action: https://github.com/SignPath/github-action-submit-signing-request
- SignPath docs — Signing Code: https://docs.signpath.io/signing-code
- Tauri v2 — Windows signing: https://v2.tauri.app/distribute/sign/windows/
