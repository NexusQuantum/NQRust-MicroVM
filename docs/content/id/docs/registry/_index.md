+++
title = "Registri Image"
description = "Panduan lengkap untuk mengelola image VM, kernel, dan root filesystem melalui antarmuka web"
weight = 70
date = 2025-01-13
+++

Registri Image adalah pusat pengelolaan semua image VM, termasuk kernel dan root filesystem.

---

## Apa Itu Registri Image?

Registri Image menyimpan dan mengelola semua image yang digunakan untuk membuat VM:

- **Kernel Images** — File kernel Linux untuk booting VM
- **Root Filesystems** — Root filesystem sistem operasi
- **Container Runtime** — Image khusus untuk menjalankan container

Image dapat digunakan ulang di banyak VM — impor sekali, gunakan di mana saja.

---

## Jenis Image

### Kernel Images

Diperlukan untuk setiap VM. Menyediakan fungsionalitas inti OS.

- Format: Binary kernel tidak terkompresi
- Ukuran umum: 5–20 MB
- Contoh: `vmlinux-5.10`, `vmlinux-6.1`, `vmlinux-6.6`

### Root Filesystem Images

Sistem operasi yang dijalankan oleh VM Anda.

- Format: Image filesystem `.ext4`
- Ukuran umum: 50 MB – 2 GB
- Contoh: `ubuntu-22.04.ext4`, `alpine-3.18.ext4`

### Container Runtime Images

Image khusus dengan Docker yang sudah terpasang untuk menjalankan container di dalam VM.

- Mencakup: Alpine Linux + Docker + OpenRC
- Digunakan secara otomatis saat men-deploy container

---

## Mengakses Registri

Navigasikan ke **Image Registry** di sidebar. Halaman ini menampilkan daftar semua image beserta nama, jenis, ukuran, jumlah VM yang menggunakannya, dan tanggal pembuatannya.

---

## Mulai Cepat

1. Buka **Image Registry** di sidebar
2. Klik **Upload** (atau gunakan metode impor lain) untuk menambahkan kernel dan rootfs
3. Buka **Virtual Machines** → **Create VM**
4. Pilih kernel dan rootfs dari menu dropdown

---

## Penyimpanan

Image disimpan di host manager pada path yang dikonfigurasi melalui `MANAGER_IMAGE_ROOT` (default: `/srv/images`).

---

## Langkah Selanjutnya

- **[Telusuri Image](browse-images/)** — Cari dan jelajahi image yang tersedia
- **[Impor Image](import-images/)** — Tambahkan image baru dari file, path, DockerHub, atau URL
- **[Kelola Image](manage-images/)** — Ubah nama, hapus, dan atur image
