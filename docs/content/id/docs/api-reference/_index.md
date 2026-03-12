+++
title = "Referensi API"
description = "Referensi REST API interaktif untuk NQRust-MicroVM — jelajahi semua endpoint, skema, dan coba request secara langsung"
icon = "code"
weight = 95
layout = "single"
toc = true
+++

NQRust-MicroVM mengekspos REST API lengkap yang dilayani oleh layanan **Manager**. Referensi API interaktif terintegrasi langsung ke dalam web UI — tidak memerlukan alat eksternal.

---

## Mengakses Referensi API

Buka URL berikut di browser Anda, ganti `<microvm-ip>` dengan alamat IP atau hostname server Anda:

```
http://<microvm-ip>:3000/docs
```

{{% alert icon="💡" context="info" %}}
`3000` adalah port default untuk web UI NQRust-MicroVM. Jika Anda mengonfigurasi port yang berbeda selama instalasi, gunakan port tersebut.
{{% /alert %}}

Referensi API terintegrasi ke dalam UI dan secara otomatis menggunakan base URL yang benar untuk host Anda — tidak diperlukan konfigurasi manual.

---

## Apa yang Tersedia

Referensi API mencakup semua endpoint yang tersedia, diorganisir berdasarkan resource:

| Bagian | Deskripsi |
|---|---|
| **Auth** | Login, manajemen token |
| **VMs** | Buat, jalankan, hentikan, hapus, daftar virtual machine |
| **VM Configuration** | CPU, memori, jaringan, pengaturan boot |
| **VM Devices** | Drive, antarmuka jaringan |
| **Containers** | Deploy dan kelola container Docker di dalam VM |
| **Functions** | Siklus hidup serverless function |
| **Images** | Registry image — impor, jelajahi, kelola |
| **Snapshots** | Tangkap dan pulihkan state VM |
| **Templates** | Template konfigurasi VM yang dapat digunakan kembali |
| **Hosts** | Registrasi dan manajemen host agent |
| **Users** | Akun pengguna dan RBAC |
| **Logs** | Streaming log container dan function |

---

## Autentikasi

Semua panggilan API (kecuali login) memerlukan Bearer token:

```bash
# 1. Dapatkan token
curl -X POST http://<microvm-ip>:18080/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "root", "password": "root"}'

# 2. Gunakan token
curl -H "Authorization: Bearer <your-token>" \
  http://<microvm-ip>:18080/v1/vms
```

**Base URL** untuk semua panggilan API adalah:

```
http://<microvm-ip>:18080/v1
```

{{% alert icon="⚠️" context="warning" %}}
UI referensi API di `:3000/docs` secara otomatis membangun base URL yang benar dari hostname browser Anda saat ini. API itu sendiri berjalan di port **18080** (layanan Manager), bukan 3000.
{{% /alert %}}

---

## Spesifikasi OpenAPI

Spesifikasi OpenAPI 3.0 mentah tersedia di:

```
http://<microvm-ip>:18080/api-docs/openapi.json
```

Gunakan ini untuk menghasilkan SDK klien dengan alat seperti `openapi-generator` atau impor ke Postman / Insomnia.
