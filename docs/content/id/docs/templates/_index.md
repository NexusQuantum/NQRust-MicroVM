+++
title = "Template VM"
description = "Simpan dan deploy konfigurasi VM sebagai template yang dapat digunakan kembali"
weight = 60
date = 2025-01-08
+++

Template VM memungkinkan Anda menyimpan konfigurasi VM sebagai blueprint yang dapat digunakan kembali. Buat template dengan pengaturan CPU, memori, kernel, dan rootfs yang telah dikonfigurasi sebelumnya, lalu deploy VM baru secara instan tanpa mengulang proses konfigurasi.

---

## Apa Itu Template?

Template adalah **konfigurasi VM yang tersimpan** yang dapat digunakan kembali untuk membuat VM baru dengan cepat. Alih-alih mengonfigurasi setiap VM dari awal, Anda mendefinisikan konfigurasi sekali sebagai template dan men-deploy beberapa VM darinya.

### Manfaat Utama

**1. Efisiensi Waktu**
- Konfigurasi sekali, deploy berkali-kali
- Tidak perlu mengulang langkah pembuatan VM
- Deploy VM dalam hitungan detik, bukan menit

**2. Konsistensi**
- Pastikan semua VM memiliki konfigurasi yang identik
- Kurangi kesalahan manusia dalam konfigurasi manual
- Standarisasi lingkungan development/staging/production

**3. Organisasi**
- Kelompokkan konfigurasi VM yang serupa
- Mudah ditemukan dan digunakan kembali untuk setup umum
- Dokumentasikan infrastruktur sebagai kode

---

## Kasus Penggunaan Umum

### Lingkungan Development

Buat template untuk lingkungan development Anda dengan CPU, RAM, dan base image tertentu:

```
Template: Dev Environment
- 2 vCPU
- 2048 MiB RAM
- Ubuntu 22.04 rootfs
- Kernel yang telah dikonfigurasi
```

Deploy VM development baru untuk setiap anggota tim atau proyek secara instan.

---

### Infrastruktur Pengujian

Siapkan template untuk skenario pengujian yang berbeda:

```
Template: Performance Test VM
- 4 vCPU
- 4096 MiB RAM
- Alpine Linux (ringan)

Template: Integration Test VM
- 1 vCPU
- 512 MiB RAM
- Ubuntu 22.04
```

Jalankan lingkungan pengujian sesuai kebutuhan, hapus saat selesai.

---

### Standardisasi Production

Pertahankan konfigurasi production yang konsisten:

```
Template: Web Server
- 2 vCPU
- 2048 MiB RAM
- Ubuntu 22.04 + NGINX

Template: Database Server
- 4 vCPU
- 8192 MiB RAM
- Ubuntu 22.04 + PostgreSQL
```

Deploy instance baru dengan konsistensi yang terjamin.

---

## Siklus Hidup Template

### 1. Buat Template

Definisikan konfigurasi VM Anda:
- Nama template (mis., "Ubuntu 22.04 Base")
- Jumlah vCPU (1-32)
- RAM dalam MiB (128-16384)
- Path atau ID image kernel
- Path atau ID image rootfs

### 2. Simpan Template

Template disimpan di database dan dapat:
- Dicantumkan dan dijelajahi
- Diperbarui sesuai kebutuhan
- Dihapus saat tidak lagi diperlukan

### 3. Deploy VM

Gunakan template untuk membuat VM:
- Klik "Deploy VM" pada template mana pun
- Masukkan nama VM
- VM dibuat dan dimulai secara otomatis
- VM yang di-deploy mewarisi semua pengaturan template

### 4. Kelola Template

Perbarui atau hapus template:
- Edit untuk mengubah CPU, RAM, atau image
- Hapus template yang tidak lagi Anda butuhkan
- VM yang dibuat dari template yang dihapus tetap berfungsi

---

## Komponen Template

### Konfigurasi Sumber Daya

**CPU (vCPU)**
- Jumlah CPU virtual
- Rentang: 1-32
- Menentukan daya pemrosesan VM

**Memori (MiB)**
- Alokasi RAM dalam mebibyte
- Rentang: 128-16384 MiB
- Memengaruhi performa dan kapasitas VM

---

### Image Boot

**Kernel Image**
- Binary kernel Linux untuk boot VM
- Dapat ditentukan berdasarkan path atau ID registri image
- Contoh: `/srv/images/vmlinux-5.10.fc.bin`

**Rootfs Image**
- Root filesystem yang berisi OS
- Dapat berupa Ubuntu, Alpine, atau image kustom
- Contoh: `/srv/images/ubuntu-22.04.ext4`

---

## Template vs VM

| Fitur | Template | VM |
|---------|----------|-----|
| Tujuan | Blueprint konfigurasi | Instance yang berjalan |
| Status | Spesifikasi statis | Dinamis (berjalan/berhenti) |
| Sumber Daya | Didefinisikan tapi tidak dialokasikan | Dialokasikan di host |
| Siklus Hidup | Buat, perbarui, hapus | Buat, mulai, hentikan, hapus |
| Biaya | Tidak menggunakan sumber daya | Menggunakan CPU/RAM host |
| Waktu Deploy | N/A | ~30 detik |

**Analoginya**: Template = Class, VM = Object Instance

---

## Mulai Cepat

### 1. Navigasi ke Templates

![Image: Templates page navigation](/images/templates/nav-templates.png)

Klik **"Templates"** di sidebar.

---

### 2. Buat Template Pertama Anda

![Image: Create template button](/images/templates/create-button.png)

1. Klik **"Create Template"**
2. Masukkan nama template
3. Atur CPU dan memori
4. Pilih image kernel dan rootfs
5. Klik **"Create Template"**

Lihat panduan [Buat Template](create-template/) untuk detail selengkapnya.

---

### 3. Deploy VM

![Image: Deploy VM button](/images/templates/deploy-button.png)

1. Temukan template Anda dalam daftar
2. Klik **"Deploy VM"**
3. Masukkan nama VM
4. Klik **"Deploy VM"**

VM dibuat dan dimulai secara otomatis!

Lihat panduan [Kelola Template](manage-templates/) untuk detail selengkapnya.

---

## Ikhtisar Halaman Templates

Halaman Templates menampilkan semua template yang tersimpan:

![Image: Templates page layout](/images/templates/page-layout.png)

**Bagian halaman**:
- **Header** - Judul halaman dan deskripsi
- **Create Button** - Membuka dialog buat template
- **Template Cards** - Grid semua template
- **Template Info** - CPU, RAM, tanggal pembuatan untuk setiap template
- **Deploy Button** - Deploy VM cepat dari template

---

## Properti Template

Setiap template menyimpan:

**Informasi Dasar**
- ID Template (UUID)
- Nama template
- Timestamp pembuatan
- Timestamp pembaruan

**Spesifikasi Sumber Daya**
- Jumlah vCPU
- Memori (MiB)
- Path atau ID image kernel
- Path atau ID image rootfs

**Metadata**
- VM yang dibuat dari template ini (dilacak)
- Timestamp deployment terakhir

---

## Keterbatasan Template

### Keterbatasan Saat Ini

❌ **Konfigurasi jaringan** - Tidak disimpan dalam template (segera hadir)
❌ **Drive tambahan** - Hanya rootfs yang disertakan (segera hadir)
❌ **Variabel lingkungan** - Bukan bagian dari spesifikasi template
❌ **Tag/kategori** - Belum ada sistem organisasi
❌ **Berbagi template** - Belum ada fungsionalitas ekspor/impor

✅ **Yang berfungsi**:
- Konfigurasi CPU dan memori
- Pemilihan image kernel dan rootfs
- Template tidak terbatas
- Deploy beberapa VM dari satu template
- Perbarui dan hapus template

---

## Praktik Terbaik

### Penamaan Template

✅ **Penamaan yang baik**:
- "Ubuntu 22.04 Base"
- "Alpine Dev Environment"
- "Production Web Server"
- "Test VM - 1 vCPU"

❌ **Penamaan yang buruk**:
- "Template 1"
- "test"
- "asdf"
- "new template copy 2"

**Tips**: Sertakan nama OS dan tujuan dalam nama template.

---

### Ukuran Sumber Daya

**Template development**:
- 1-2 vCPU
- 512-2048 MiB RAM
- Jaga agar ringan untuk deployment lebih cepat

**Template production**:
- 2-4 vCPU
- 2048-8192 MiB RAM
- Ukuran berdasarkan kebutuhan beban kerja aktual

**Template pengujian**:
- 1 vCPU
- 512 MiB RAM
- Sumber daya minimal untuk spin-up cepat

---

### Manajemen Image

✅ **Gunakan image yang konsisten**:
- Jaga kompatibilitas versi kernel dan rootfs
- Gunakan kombinasi image yang telah diuji
- Dokumentasikan image mana yang bekerja bersama

✅ **Gunakan registri image**:
- Referensikan image berdasarkan ID registri jika memungkinkan
- Lebih mudah memperbarui semua template sekaligus
- Pelacakan penggunaan image lebih baik

---

## Pemecahan Masalah

### Template tidak dapat dibuat

**Periksa**:
- Nama template tidak kosong
- vCPU antara 1-32
- Memori antara 128-16384 MiB
- Path kernel dan rootfs ada

---

### VM gagal setelah deployment

**Kemungkinan penyebab**:
- File kernel atau rootfs tidak ditemukan di host
- Sumber daya host tidak mencukupi
- File image rusak

**Solusi**:
1. Verifikasi path image dalam template
2. Periksa apakah host memiliki CPU/RAM yang cukup
3. Uji image dengan pembuatan VM manual terlebih dahulu

---

### Template tidak muncul dalam daftar

**Periksa**:
- Muat ulang halaman
- Periksa konsol browser untuk error
- Verifikasi manager API sedang berjalan
- Periksa apakah template tidak terhapus

---

## Langkah Selanjutnya

- **[Buat Template](create-template/)** - Panduan pembuatan template langkah demi langkah
- **[Kelola Template](manage-templates/)** - Deploy, perbarui, dan hapus template
- **[Buat VM](/docs/vm/create-vm/)** - Pelajari tentang pembuatan VM manual
- **[Manajemen VM](/docs/vm/manage-vm/)** - Mengelola VM yang di-deploy

---

## FAQ

**T: Bisakah saya membuat template dari VM yang sudah ada?**
J: Belum. Fitur ini direncanakan untuk rilis mendatang. Untuk saat ini, catat konfigurasi VM dan buat template baru secara manual.

**T: Apa yang terjadi pada VM saat saya menghapus template?**
J: VM terus berjalan secara normal. VM bersifat independen setelah dibuat.

**T: Bisakah saya memperbarui template setelah membuatnya?**
J: Ya! Edit template dan perubahan akan diterapkan pada deployment mendatang (bukan VM yang sudah ada).

**T: Berapa banyak VM yang dapat saya deploy dari satu template?**
J: Tidak terbatas, selama host Anda memiliki sumber daya yang cukup.

**T: Apakah template dapat mencakup konfigurasi jaringan atau penyimpanan?**
J: Belum. Saat ini template hanya menyimpan CPU, RAM, kernel, dan rootfs. Template jaringan dan penyimpanan adalah fitur yang direncanakan.
