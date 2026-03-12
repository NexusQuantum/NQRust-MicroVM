+++
title = "Telusuri Image"
description = "Cari, filter, dan jelajahi image yang tersedia di registri"
weight = 51
date = 2025-01-13
+++

Temukan dan jelajahi image yang tersedia di registri.

---

## Mengakses Registri

Navigasikan ke **Image Registry** di sidebar. Halaman ini menampilkan tabel semua image dengan kolom berikut:

| Kolom | Deskripsi |
|---|---|
| **Name** | Nama file image |
| **Type** | Kernel atau Rootfs |
| **Size** | Ukuran file di disk |
| **VMs** | Jumlah VM yang saat ini menggunakan image ini |
| **Created** | Tanggal upload/impor |
| **Actions** | Hapus, ubah nama, salin path |

---

## Mencari Image

Gunakan bilah pencarian di bagian atas halaman registri untuk memfilter berdasarkan nama. Pencarian tidak peka huruf besar/kecil dan mencocokkan nama sebagian.

**Contoh**:
```
ubuntu        → menemukan ubuntu-22.04.ext4, ubuntu-20.04.ext4
vmlinux       → menemukan semua kernel image
alpine        → menemukan alpine-3.18.ext4
```

---

## Memfilter Berdasarkan Jenis

Gunakan filter jenis untuk menampilkan hanya kategori image tertentu:

- **All** — Tampilkan semua image
- **Kernel** — Tampilkan hanya kernel image
- **Rootfs** — Tampilkan hanya root filesystem image

Ini berguna saat memilih image selama pembuatan VM untuk mempersempit daftar dengan cepat.

---

## Detail Image

Setiap baris menampilkan:

- **Name** — Nama file yang digunakan saat mereferensikan image
- **Size** — Rentang umum:
  - Kernel: 5–20 MB
  - Root filesystem: 50 MB – 2 GB
- **VMs** — Berapa banyak VM yang menggunakan image ini. Image yang sedang digunakan oleh VM aktif tidak dapat dihapus.

---

## Menyalin Path Image

Klik ikon **copy path** di kolom Actions untuk menyalin path lengkap sisi server ke clipboard Anda. Berguna saat menulis skrip atau mereferensikan image melalui API.

---

## Pemecahan Masalah

### Tidak ada image dalam daftar

Registri kosong — Anda perlu mengimpor image sebelum membuat VM. Lihat [Impor Image](import-images/).

### Tidak ada hasil setelah pencarian

Periksa ejaan, atau hapus filter jenis — image mungkin memiliki jenis yang berbeda dari yang saat ini dipilih.

### Dropdown Kernel/Rootfs kosong saat pembuatan VM

Dropdown hanya menampilkan image dari jenis yang sesuai. Jika kosong, impor jenis image yang diperlukan terlebih dahulu.

---

## Langkah Selanjutnya

- **[Impor Image](import-images/)** — Tambahkan image baru
- **[Kelola Image](manage-images/)** — Hapus, ubah nama, atur image
