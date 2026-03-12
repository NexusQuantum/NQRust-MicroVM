+++
title = "Monitor Statistik"
description = "Penggunaan resource dan metrik performa secara real-time"
weight = 34
date = 2025-12-18
+++

Monitor penggunaan resource container Anda secara real-time dari tab **Stats**.

---

## Membuka Statistik

1. Buka **Containers** dan klik container
2. Klik tab **Stats**

![Container real-time metrics dashboard](/images/containers/container-stats.png)

Statistik mengalir secara langsung selama container berjalan. Status bar di bagian bawah menampilkan **"Monitoring for X seconds • Connected"**.

Gunakan tombol **Refresh** di header kanan atas untuk memuat ulang secara manual, atau **Stop Monitoring** untuk menjeda aliran langsung.

---

## Kartu Ringkasan Metrik

Empat counter langsung yang terus diperbarui di bagian atas halaman:

| Kartu | Yang Ditampilkan |
|-------|-----------------|
| **CPU Usage** | Utilisasi CPU saat ini dalam persentase |
| **Memory Usage** | Utilisasi RAM saat ini dalam persentase |
| **Network I/O** | Throughput gabungan masuk + keluar dalam KB/s |
| **Disk I/O** | Throughput baca + tulis gabungan dalam KB/s |

---

## Chart

### Penggunaan CPU & Memory

Chart garis time-series memplot **CPU %** (oranye) dan **Memory %** (biru) dalam jendela bergulir. Gunakan ini untuk melihat penggunaan tinggi yang berkelanjutan, lonjakan, atau garis memory yang naik perlahan (kemungkinan kebocoran memory).

### I/O Jaringan & Disk

Chart kedua memplot **Disk KB/s** (ungu) dan **Network KB/s** (hijau). Berguna untuk mengidentifikasi lonjakan aktivitas disk atau jaringan — misalnya saat startup container, operasi file besar, atau lonjakan traffic masuk.

---

## Aksi Header

Dari header detail container Anda dapat:

- **Refresh** — muat ulang halaman secara paksa
- **Edit** — ubah pengaturan container
- **View Container VM** — lompat ke VM yang mendasari container ini
- **Delete** — hapus container

---

## Analisis Performa

### CPU terlalu tinggi (>90%)

- Tingkatkan alokasi CPU: stop → Edit → tingkatkan vCPU → start
- Profile aplikasi dan optimalkan jalur yang sering dijalankan
- Periksa apakah cron job atau siklus GC menyebabkan lonjakan — lihat tab Logs pada timestamp yang sama

### Memory terus naik

Kemiringan ke atas yang bertahap pada garis memory adalah tanda kebocoran memory. Solusi jangka pendek: restart container. Solusi jangka panjang: profile aplikasi untuk menemukan kebocoran.

### Lonjakan Disk I/O besar saat startup

Normal — runtime container menarik layer dan menginisialisasi penyimpanan. Disk I/O seharusnya stabil setelah container berjalan penuh.

### Network I/O tiba-tiba tinggi

Cek tab Logs untuk error atau retry storm. I/O keluar yang tinggi dengan I/O masuk yang rendah bisa mengindikasikan container mengirim data yang tidak seharusnya.

---

## Pemecahan Masalah

### Statistik tidak muncul / chart kosong

1. Pastikan status container adalah **Running** (statistik hanya tersedia untuk container yang berjalan)
2. Klik **Refresh** di header
3. Klik **View Container VM** untuk memverifikasi VM yang mendasarinya juga berjalan

### Metrik membeku / tidak diperbarui

1. Pindah ke tab jika sedang di latar belakang (browser membatasi tab yang tidak aktif)
2. Refresh halaman
3. Periksa koneksi jaringan Anda

### Memory selalu 100% tetapi container stabil

Beberapa aplikasi (Redis, Memcached) secara sengaja menggunakan semua memory yang dialokasikan sebagai cache — ini normal. Cek Log untuk error OOM guna memastikan ini bukan masalah.

---

## Langkah Selanjutnya

- **[Lihat Log](../logs/)** — Debug masalah yang teridentifikasi dari statistik
- **[Kelola Containers](../manage-containers/)** — Sesuaikan resource berdasarkan statistik
- **[Deploy Container](../deploy-container/)** — Terapkan pelajaran alokasi yang dipelajari
