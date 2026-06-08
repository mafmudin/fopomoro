# Troubleshooting build Windows/WSL (FoPoMoro)

Catatan masalah lingkungan yang pernah menghambat `cargo build` (dan koneksi
Claude Code di WSL). Disusun 2026-06-08 setelah kejadian nyata.

## Issue #1 — DNS gagal: IPv6 router (`fe80::1`) ngadat (BERULANG)

### Gejala
- `cargo build` → `Could not resolve host: index.crates.io` (curl gagal).
- `nslookup index.crates.io` → `DNS request timed out`, `Server: UnKnown,
  Address: fe80::1`.
- **Claude Code di WSL ikut tak bisa konek** (kejadian terpisah, akar sama).

### Akar masalah
Jaringan rumah (router) meng-iklankan **DNS via IPv6** = `fe80::1` lewat Router
Advertisement, **tapi IPv6 tidak benar-benar nyambung ke internet**. Windows
**memprioritaskan DNS IPv6**, jadi semua query nyangkut ke `fe80::1` yang mati —
padahal **IPv4 sehat** (`nslookup index.crates.io 8.8.8.8` berhasil).

Kenapa Claude di WSL ikut kena: `/etc/resolv.conf` WSL = `nameserver
10.255.255.254` (NAT gateway yang mem-proxy DNS ke **host Windows**). Saat DNS
host Windows rusak, WSL ikut buta → Claude Code tak bisa reach API.

### Diagnosa cepat
```powershell
nslookup index.crates.io            # gagal, Server = fe80::1
nslookup index.crates.io 8.8.8.8    # BERHASIL -> bukti: IPv4 sehat, cuma DNS router/IPv6 yang rusak
```

### Fix permanen (per-adapter, persist lintas reboot) — PowerShell as Admin
```powershell
# 1. DNS publik untuk IPv4 + IPv6
Set-DnsClientServerAddress -InterfaceAlias "Wi-Fi" -ServerAddresses ("8.8.8.8","1.1.1.1","2001:4860:4860::8888","2606:4700:4700::1111")

# 2. Matikan IPv6 di adapter (router-nya iklankan IPv6 yang tak fungsional)
Disable-NetAdapterBinding -Name "Wi-Fi" -ComponentID ms_tcpip6

# 3. Flush + verifikasi (Server harus jadi dns.google, bukan fe80::1)
ipconfig /flushdns
nslookup index.crates.io
```
> Ganti `"Wi-Fi"` dgn nama adapter aktif (`Get-NetAdapter | ? Status -eq 'Up'`).

Membalikkan: `Enable-NetAdapterBinding -Name "Wi-Fi" -ComponentID ms_tcpip6`
dan/atau `Set-DnsClientServerAddress -InterfaceAlias "Wi-Fi" -ResetServerAddresses`.

### (Opsional) Hardening sisi WSL — lepas ketergantungan dari DNS host
Kalau mau Claude/WSL tahan walau DNS host Windows rusak lagi, pin DNS WSL:
`/etc/wsl.conf` → tambah `[network]\ngenerateResolvConf = false`, lalu buat
`/etc/resolv.conf` berisi `nameserver 8.8.8.8`, lalu `wsl --shutdown` dari
PowerShell. (Belum diterapkan — fix host di atas sudah cukup untuk saat ini.)

## Issue #2 — `link.exe` not found (MSVC linker)

### Gejala
`cargo build` → `error: linker 'link.exe' not found` saat compile build-script
crate pertama (proc-macro2, quote, dst).

### Akar & fix
Toolchain Rust Windows default ke target **MSVC**, butuh linker C++ dari Visual
Studio Build Tools (belum terpasang).
```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```
Atau GUI: https://visualstudio.microsoft.com/visual-cpp-build-tools/ → centang
**"Desktop development with C++"**. Setelah selesai, **buka terminal baru** lalu
`cargo build` lagi. (WebView2 untuk Tauri umumnya sudah bawaan Win10/11.)

## Status
Setelah kedua fix di atas: `cargo build` → **Finished dev profile, 0 error**
(2026-06-08). Lanjut ke Verifikasi Tahap 5 di `multiuser-auth-plan.md`.
