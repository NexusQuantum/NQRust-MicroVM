+++
title = "Impor Image"
description = "Tambahkan kernel dan rootfs image baru ke registri dari berbagai sumber"
weight = 52
date = 2025-01-13
+++

Tambahkan image ke registri dari empat sumber: upload file lokal, path server, DockerHub, atau URL langsung.

---

## Ringkasan Metode Impor

| Metode | Terbaik untuk |
|---|---|
| **Upload File** | Image di mesin lokal Anda |
| **Import from Path** | Image yang sudah ada di server |
| **DockerHub** | Image rootfs OS resmi |
| **URL** | Image yang dihosting di web |

---

## Upload File

Upload image langsung dari browser Anda.

### Langkah-Langkah

1. Buka **Image Registry** dan klik **Upload**
2. Pilih **Type** image: Kernel atau Rootfs
3. Klik **Choose File** dan pilih file dari mesin Anda
4. Masukkan **Name** untuk image (mis. `ubuntu-22.04`, `vmlinux-6.1`)
5. Klik **Upload**

Bilah progres menampilkan status upload. File berukuran besar mungkin membutuhkan beberapa menit tergantung pada kecepatan koneksi Anda.

### Format yang Didukung

- Kernel: binary tidak terkompresi (`vmlinux`)
- Rootfs: image filesystem `.ext4`

### Tips Penamaan

```
Nama yang baik:
  ubuntu-22.04
  alpine-3.18-minimal
  vmlinux-6.1-lts

Nama yang buruk:
  image1
  test
  final_v2
```

---

## Impor dari Path

Impor image yang sudah ada di filesystem server. Membutuhkan `MANAGER_ALLOW_IMAGE_PATHS=true` untuk diaktifkan.

### Langkah-Langkah

1. Klik **Import from Path**
2. Pilih **Type** image
3. Masukkan **absolute path** ke file di server (mis. `/srv/images/custom/my-kernel`)
4. Masukkan **Name**
5. Klik **Import**

### Catatan

- Path harus dapat dibaca oleh proses manager
- Tidak ada data yang disalin — manager mereferensikan file di lokasi aslinya
- Berguna untuk image berukuran besar yang sudah ada di server (menghindari upload ulang)

---

## Impor dari DockerHub

Unduh image rootfs dari DockerHub. Manager mengekstrak filesystem dari layer image Docker.

### Langkah-Langkah

1. Klik **DockerHub**
2. Masukkan referensi image dalam format Docker:
   ```
   library/ubuntu:22.04
   library/alpine:3.18
   library/debian:12
   ```
3. Masukkan **Name**
4. Klik **Import**

Unduhan berjalan di latar belakang. Progres ditampilkan di halaman. Image berukuran besar (mis. Ubuntu) dapat membutuhkan beberapa menit.

### Image Populer

```
library/ubuntu:22.04
library/ubuntu:20.04
library/alpine:3.18
library/debian:12
library/fedora:39
```

---

## Impor dari URL

Unduh image langsung dari URL.

### Langkah-Langkah

1. Klik **Import from URL**
2. Tempel URL unduhan langsung (harus berupa tautan langsung ke file, bukan halaman web)
3. Pilih **Type**
4. Masukkan **Name**
5. Klik **Import**

Progres unduhan ditampilkan. Image disimpan ke registri setelah unduhan selesai.

### Contoh URL yang Valid

```
https://example.com/images/ubuntu-22.04.ext4
https://releases.example.org/kernels/vmlinux-6.1
```

---

## Setelah Mengimpor

Setelah image muncul di daftar registri, image siap digunakan dalam pembuatan VM. Buka **Virtual Machines** → **Create VM** dan pilih image dari dropdown Kernel atau Rootfs.

---

## Pemecahan Masalah

### Upload gagal atau habis waktu

- Periksa format file — kernel harus berupa binary tidak terkompresi, rootfs harus berupa `.ext4`
- Untuk file yang sangat besar (>1 GB), gunakan Import from Path jika file ada di server, atau impor URL jika dihosting secara remote

### Impor dari Path: "file not found"

- Pastikan path bersifat absolut (dimulai dengan `/`)
- Periksa apakah file ada dan proses manager memiliki izin baca
- Konfirmasi `MANAGER_ALLOW_IMAGE_PATHS=true` sudah diaktifkan

### Impor DockerHub habis waktu

- Image berukuran besar (Ubuntu, Debian) dapat membutuhkan 5–15 menit
- Muat ulang halaman registri — impor mungkin sudah selesai di latar belakang
- Periksa konektivitas internet server

### Impor URL gagal

- Konfirmasi URL adalah tautan file langsung (bukan pengalihan atau halaman HTML)
- Uji URL di browser — seharusnya memicu unduhan file

---

## Langkah Selanjutnya

- **[Telusuri Image](browse-images/)** — Temukan dan filter image yang telah diimpor
- **[Kelola Image](manage-images/)** — Ubah nama atau hapus image
