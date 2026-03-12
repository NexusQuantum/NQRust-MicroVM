+++
title = "Pemantauan"
description = "Pantau performa dan penggunaan sumber daya VM"
weight = 45
date = 2025-12-16
+++

Pantau performa VM Anda secara real-time dari tab **Metrics** di halaman detail VM.

---

## Membuka Tab Metrics

1. Pergi ke **Virtual Machines** dan klik VM yang sedang berjalan
2. Klik tab **Metrics** di navigasi atas

![VM real-time metrics dashboard](/images/vm/vm-metrics.png)

Dashboard langsung mulai mengalirkan data langsung. Status bar di bagian bawah menampilkan **"Monitoring for X seconds • Connected"** yang mengonfirmasi koneksi WebSocket aktif.

---

## Kartu Ringkasan Metrik

Di bagian atas Anda akan menemukan empat penghitung langsung yang diperbarui setiap detik:

| Kartu | Yang Ditampilkan |
|---|---|
| **CPU Usage** | Utilisasi CPU saat ini sebagai persentase |
| **Memory Usage** | Utilisasi RAM saat ini sebagai persentase |
| **Network I/O** | Throughput gabungan masuk + keluar dalam KB/s |
| **Disk I/O** | Throughput gabungan baca + tulis dalam KB/s |

---

## Grafik

### Penggunaan CPU & Memori

Grafik time-series yang memplot **CPU %** (oranye) dan **Memory %** (biru) dalam jendela waktu bergulir. Gunakan ini untuk mendeteksi lonjakan, penggunaan tinggi yang berkelanjutan, atau kebocoran memori dari waktu ke waktu.

### Network & Disk I/O

Grafik kedua memplot **Disk KB/s** (ungu) dan **Network KB/s** (hijau). Berguna untuk mengidentifikasi lonjakan aktivitas disk atau jaringan — misalnya transfer file besar atau penulisan database.

---

## Memulai dan Menghentikan Pemantauan

- Pemantauan dimulai secara otomatis saat Anda membuka tab Metrics.
- Klik **Stop Monitoring** (pojok kanan atas) untuk menjeda aliran langsung.
- Meninggalkan tab secara otomatis memutus aliran.

---

## Langkah Berikutnya

- **[Backup & Snapshot](backup-snapshot/)** — Lindungi data VM Anda
- **[Kelola VM](manage-vm/)** — Operasi start, stop, pause
