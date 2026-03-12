+++
title = "Kelola Image"
description = "Hapus, ubah nama, dan atur image di registri"
weight = 53
date = 2025-01-13
+++

Kelola registri image Anda dengan menghapus image yang tidak digunakan, mengubah nama untuk kejelasan, dan menjaga penyimpanan tetap terorganisir.

---

## Aksi yang Tersedia

Setiap baris image memiliki menu **Actions** dengan:

| Aksi | Deskripsi |
|---|---|
| **Rename** | Ubah nama image |
| **Copy Path** | Salin path file sisi server |
| **Delete** | Hapus image secara permanen |

---

## Menghapus Image

### Sebelum menghapus

Periksa kolom **VMs** — kolom ini menunjukkan berapa banyak VM yang saat ini menggunakan image tersebut. Image yang sedang digunakan oleh satu atau lebih VM tidak dapat dihapus. Hentikan atau hapus VM tersebut terlebih dahulu.

### Langkah-Langkah

1. Klik **Delete** di kolom Actions
2. Konfirmasi penghapusan di dialog
3. Image dihapus secara permanen dari registri dan disk

**Tindakan ini tidak dapat dibatalkan.** Pastikan tidak ada VM yang bergantung pada image sebelum menghapus.

### Pembersihan massal

Untuk membebaskan ruang secara efisien:
1. Filter berdasarkan **Type** untuk fokus pada satu kategori sekaligus
2. Urutkan berdasarkan kolom **VMs** untuk menemukan image dengan 0 VM
3. Hapus image yang tidak digunakan satu per satu

---

## Mengubah Nama Image

Ubah nama image agar mengikuti konvensi penamaan yang konsisten atau untuk memperjelas isi image tersebut.

### Langkah-Langkah

1. Klik **Rename** di kolom Actions
2. Masukkan nama baru di dialog
3. Klik **Save**

### Konvensi penamaan

```
<os>-<version>[-variant]
  ubuntu-22.04
  alpine-3.18-minimal
  debian-12

vmlinux-<version>[-variant]
  vmlinux-6.1
  vmlinux-5.10-lts
```

**Catatan**: Mengubah nama tidak memengaruhi VM yang sudah menggunakan image — VM mereferensikan image berdasarkan ID secara internal, bukan berdasarkan nama.

---

## Menyalin Path Image

Klik **Copy Path** untuk menyalin path file lengkap sisi server ke clipboard Anda.

**Kasus penggunaan**:
- Mereferensikan image dalam panggilan API
- Menulis skrip provisioning
- Dokumentasi dan runbook

**Contoh path**:
```
/srv/images/ubuntu-22.04.ext4
/srv/images/vmlinux-6.1
```

---

## Manajemen Penyimpanan

### Menemukan image berukuran besar

Urutkan tabel berdasarkan **Size** untuk mengidentifikasi image terbesar. Image root filesystem biasanya paling besar — kernel berukuran kecil.

### Memeriksa yang sedang digunakan

Kolom **VMs** menunjukkan penggunaan aktif. Image dengan `0` VM aman untuk dihapus jika tidak lagi diperlukan.

### Jadwal pembersihan yang disarankan

| Frekuensi | Tugas |
|---|---|
| Mingguan | Hapus image dengan 0 VM yang sudah tidak diperlukan |
| Bulanan | Tinjau dan ubah nama image dengan nama yang tidak jelas |
| Setelah upgrade | Hapus versi kernel/rootfs lama setelah VM dimigrasikan |

---

## Pemecahan Masalah

### Tidak dapat menghapus — image sedang digunakan

Kolom **VMs** menampilkan angka lebih dari 0. Navigasikan ke VM tersebut, lalu hentikan dan hapus VM tersebut, atau perbarui VM untuk menggunakan image yang berbeda sebelum mencoba menghapus lagi.

### Ubah nama tidak langsung terlihat

Muat ulang halaman browser — tabel registri mungkin masih dalam cache.

### Image yang terhapus secara tidak sengaja

Tidak ada recycle bin — image yang dihapus tidak dapat dipulihkan. Anda perlu mengimpor ulang image menggunakan salah satu [metode impor](import-images/).

---

## Langkah Selanjutnya

- **[Telusuri Image](browse-images/)** — Cari dan filter image
- **[Impor Image](import-images/)** — Tambahkan image baru untuk menggantikan yang dihapus
