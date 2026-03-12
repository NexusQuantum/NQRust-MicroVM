+++
title = "Virtual Machines"
description = "Buat dan kelola Firecracker microVM yang ringan"
weight = 30
date = 2025-12-16
+++

Virtual Machines di NQRust-MicroVM ditenagai oleh Firecracker, yang menyediakan virtualisasi ringan, aman, dan cepat untuk beban kerja Anda.

---

## Gambaran Umum

NQRust-MicroVM menggunakan Firecracker untuk membuat microVM — mesin virtual minimal yang dirancang untuk beban kerja serverless dan container. Setiap VM memberikan isolasi penuh dengan sumber daya yang didedikasikan.

**[IMAGE: vm-overview.png - Screenshot of VMs dashboard showing list of running VMs]**

#### Fitur Utama

- **Boot Super Cepat** - VM menyala dalam waktu kurang dari 125ms
- **Overhead Minimal** - Hanya 5 MB memori per VM
- **Isolasi Kuat** - Keamanan berbasis virtualisasi hardware
- **Dukungan Linux Penuh** - Jalankan Ubuntu, Alpine, Debian, dan lainnya
- **Web Console** - Akses terminal berbasis browser
- **Metrik Langsung** - Pemantauan CPU, memori, dan jaringan secara real-time

---

## Mulai Cepat

**Buat VM pertama Anda dalam 3 menit**:

1. Navigasi ke **Virtual Machines** di sidebar
2. Klik tombol **Create VM**
3. Ikuti wizard 5 langkah
4. Akses VM Anda melalui web console

---

## Siklus Hidup VM

**[IMAGE: vm-lifecycle.png - Diagram showing VM states: Stopped → Running → Paused]**

VM dapat berada dalam status berikut:

- **Stopped** - VM telah dibuat tetapi tidak berjalan
- **Running** - VM aktif dan menggunakan sumber daya
- **Paused** - VM dibekukan, dapat dilanjutkan dengan cepat
- **Failed** - VM mengalami kesalahan

---

## Kasus Penggunaan Umum

#### Lingkungan Pengembangan
Buat lingkungan dev yang terisolasi untuk setiap anggota tim dengan konfigurasi yang konsisten.

**[IMAGE: usecase-dev.png - Screenshot showing multiple dev VMs]**

#### Pengujian & CI/CD
Buat lingkungan pengujian baru untuk setiap siklus pengujian, lalu hapus secara otomatis.

#### Beban Kerja Produksi
Jalankan microservice dengan isolasi kuat dan overhead minimal.

---

## Mulai Sekarang

Pilih topik untuk mempelajari lebih lanjut:

- **[Buat VM](create-vm/)** - Panduan pembuatan VM langkah demi langkah
- **[Akses VM](access-vm/)** - Hubungkan ke VM melalui console atau SSH
- **[Kelola VM](manage-vm/)** - Operasi start, stop, pause, resume
- **[Backup & Snapshot](backup-snapshot/)** - Lindungi data VM Anda
- **[Pemantauan](monitoring/)** - Lihat metrik dan log secara real-time
- **[Pemecahan Masalah](troubleshooting/)** - Diagnosa dan selesaikan masalah umum
