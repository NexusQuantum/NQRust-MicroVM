+++
title = "Aktivasi Lisensi"
description = "Cara mengaktifkan NQRust-MicroVM menggunakan kunci lisensi online atau file lisensi offline"
date = 2025-12-01
weight = 13
toc = true
+++

Setelah installer selesai dan Anda membuka web UI untuk pertama kalinya, NQRust-MicroVM memerlukan aktivasi lisensi sebelum platform dapat digunakan. Anda akan diarahkan secara otomatis ke layar aktivasi.

---

## Metode Aktivasi

Ada dua cara untuk mengaktifkan lisensi Anda:

| Metode | Kapan Digunakan |
|---|---|
| **Kunci Lisensi** | Lingkungan online dengan akses internet |
| **File Offline** | Jaringan air-gapped atau terbatas |

---

## Online — Kunci Lisensi

{{< img src="/images/license/license-key.png" alt="Aktifkan Lisensi — tab Kunci Lisensi" >}}

1. Buka `http://<microvm-ip>:3000/setup/license` (atau tunggu pengalihan otomatis saat login pertama kali).
2. Pastikan tab **License Key** dipilih.
3. Masukkan kunci Anda dalam format `XXXX-XXXX-XXXX-XXXX`.
4. Klik **Activate License**.

Jika berhasil, Anda akan diarahkan ke dashboard.

{{% alert icon="🔑" context="info" %}}
Kunci lisensi diterbitkan oleh Nexus Quantum Tech. Hubungi perwakilan akun Anda atau periksa email konfirmasi pembelian.
{{% /alert %}}

---

## Offline — File Lisensi

{{< img src="/images/license/license-offline.png" alt="Aktifkan Lisensi — tab File Offline" >}}

Gunakan metode ini saat server Anda tidak memiliki akses internet keluar (instalasi air-gap).

1. Dapatkan file lisensi `.lic` dari Nexus Quantum Tech.
2. Buka `http://<microvm-ip>:3000/setup/license`.
3. Pilih tab **Offline File**.
4. Klik area unggah atau seret-dan-lepas file `.lic` Anda.
5. Klik **Upload & Activate**.

{{% alert icon="⚠️" context="warning" %}}
File lisensi offline terikat pada mesin yang dituju. Jangan menyalin file `.lic` ke host yang berbeda.
{{% /alert %}}

---

## Setelah Aktivasi

Setelah diaktifkan, status lisensi disimpan dalam database platform. Anda tidak perlu mengaktifkan ulang setelah restart normal. Aktivasi ulang hanya diperlukan jika:

- Anda migrasi ke server yang berbeda.
- Kunci lisensi Anda kedaluwarsa atau dicabut.
- Anda melakukan reset database penuh.

---

## Pemecahan Masalah

| Masalah | Solusi |
|---|---|
| "Invalid license key" | Periksa kembali kesalahan ketik; kunci tidak peka huruf besar/kecil |
| "License already in use" | Hubungi Nexus Quantum Tech untuk transfer atau penerbitan ulang |
| File offline ditolak | Pastikan file `.lic` dibuat untuk host ini |
| Redirect loop setelah aktivasi | Bersihkan cache browser dan muat ulang |
