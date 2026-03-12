+++
title = "Buat Template"
description = "Simpan konfigurasi VM sebagai template yang dapat digunakan kembali"
weight = 36
date = 2025-01-08
+++

Pelajari cara membuat template VM yang dapat digunakan kembali untuk men-deploy VM baru dengan cepat menggunakan pengaturan yang telah dikonfigurasi sebelumnya.

---

## Sebelum Memulai

### Prasyarat

✅ **Diperlukan**:
- Akses ke halaman Templates
- Kernel image yang valid (di `/srv/images/` atau registri)
- Rootfs image yang valid (di `/srv/images/` atau registri)

✅ **Disarankan**:
- Mengetahui spesifikasi CPU dan RAM yang diinginkan
- Telah menguji kombinasi kernel + rootfs
- Telah menentukan nama template yang deskriptif

---

## Membuat Template

### Langkah 1: Buka Dialog Buat

Navigasikan ke halaman Templates:

![Image: Templates page](/images/templates/page.png)

Klik tombol **"Create Template"** di header:

![Image: Create template button](/images/templates/create-button.png)

Dialog buat template akan terbuka.

---

### Langkah 2: Masukkan Nama Template

![Image: Template name field](/images/templates/create-name.png)

Masukkan **nama yang deskriptif** untuk template Anda:

**Contoh yang baik**:
- `Ubuntu 22.04 Base`
- `Alpine Dev Environment`
- `Production Web Server`
- `Test VM - 1 vCPU`

**Tips**:
- Sertakan nama OS untuk kejelasan
- Sebutkan tujuan atau kasus penggunaan
- Jaga agar singkat tapi deskriptif
- Hindari nama generik seperti "Template 1"

---

### Langkah 3: Atur Alokasi CPU

![Image: vCPU field](/images/templates/create-vcpu.png)

**Rentang vCPU**: 1-32 CPU virtual

**Nilai yang disarankan**:
- **1 vCPU** - Tugas ringan, pengujian
- **2 vCPU** - Development, beban kerja kecil
- **4 vCPU** - Aplikasi production, database
- **8+ vCPU** - Beban kerja berat, pemrosesan data

**Contoh konfigurasi**:
```
Development:    2 vCPU
Staging:        2 vCPU
Production:     4 vCPU
High-Performance: 8 vCPU
```

**Catatan**: VM yang di-deploy dari template ini akan menggunakan alokasi CPU ini.

---

### Langkah 4: Atur Alokasi Memori

![Image: Memory field](/images/templates/create-memory.png)

**Rentang Memori**: 128-16384 MiB (0.125 - 16 GB)

**Nilai umum**:
- **512 MiB** - VM pengujian minimal
- **1024 MiB (1 GB)** - Development ringan
- **2048 MiB (2 GB)** - Development/staging standar
- **4096 MiB (4 GB)** - Aplikasi production
- **8192 MiB (8 GB)** - Database, beban kerja berat

**Contoh konfigurasi**:
```
Test VM:        512 MiB
Dev Environment: 2048 MiB
Web Server:     2048 MiB
Database:       8192 MiB
```

**Peringatan**: Pastikan host memiliki RAM bebas yang cukup untuk semua VM yang di-deploy.

---

### Langkah 5: Pilih Kernel Image

![Image: Kernel path field](/images/templates/create-kernel.png)

Pilih cara menentukan kernel:

**Opsi 1: File Path** (Disarankan untuk setup lokal)
```
/srv/images/vmlinux-5.10.fc.bin
```

**Opsi 2: Image Registry ID** (Lebih baik untuk production)
```
550e8400-e29b-41d4-a716-446655440000
```

**Path kernel umum**:
- `/srv/images/vmlinux-5.10.fc.bin` - Kernel Firecracker standar
- `/srv/images/vmlinux-5.10.bin` - Versi alternatif
- Path kustom jika Anda membangun kernel sendiri

**Tempat menemukan kernel image**:
1. Periksa direktori `/srv/images/` di host
2. Telusuri halaman Image Registry di UI
3. Gunakan kernel yang sudah dimuat dari skrip setup

---

### Langkah 6: Pilih Rootfs Image

![Image: Rootfs path field](/images/templates/create-rootfs.png)

Pilih image root filesystem:

**Opsi 1: File Path**
```
/srv/images/ubuntu-22.04.ext4
/srv/images/alpine-3.18.ext4
```

**Opsi 2: Image Registry ID**
```
660e9500-f39c-51e5-b827-557766551111
```

**Image rootfs umum**:

**Ubuntu**:
- `/srv/images/ubuntu-22.04.ext4` - Ubuntu 22.04 LTS
- `/srv/images/ubuntu-20.04.ext4` - Ubuntu 20.04 LTS

**Alpine**:
- `/srv/images/alpine-3.18.ext4` - Alpine Linux 3.18
- `/srv/images/alpine-3.19.ext4` - Alpine Linux 3.19

**Kustom**:
- Bangun sendiri dengan perangkat lunak yang sudah terpasang
- Simpan di direktori `/srv/images/`

**Penting**: Pastikan kernel dan rootfs kompatibel!

---

### Langkah 7: Tinjau Konfigurasi

Sebelum membuat, tinjau pengaturan Anda:

![Image: Template configuration review](/images/templates/create-review.png)

**Periksa**:
- ✅ Nama template deskriptif
- ✅ Jumlah vCPU sesuai
- ✅ Alokasi memori mencukupi
- ✅ Path kernel ada dan valid
- ✅ Path rootfs ada dan valid
- ✅ Kernel dan rootfs kompatibel

---

### Langkah 8: Buat Template

Klik tombol **"Create Template"**:

![Image: Create button](/images/templates/create-submit.png)

**Yang terjadi**:
1. Validasi formulir dijalankan
2. Panggilan API ke backend (`POST /v1/templates`)
3. Template disimpan ke database
4. Notifikasi sukses muncul
5. Template muncul dalam daftar
6. Dialog ditutup secara otomatis

**Sukses**:
![Image: Success notification](/images/templates/create-success.png)

Template Anda siap digunakan!

---

## Contoh Template

### Contoh 1: Lingkungan Development

**Nama**: `Ubuntu Dev Environment`

**Konfigurasi**:
- vCPU: `2`
- Memori: `2048 MiB`
- Kernel: `/srv/images/vmlinux-5.10.fc.bin`
- Rootfs: `/srv/images/ubuntu-22.04.ext4`

**Kasus penggunaan**: VM development standar untuk anggota tim

---

### Contoh 2: VM Pengujian Ringan

**Nama**: `Alpine Test VM`

**Konfigurasi**:
- vCPU: `1`
- Memori: `512 MiB`
- Kernel: `/srv/images/vmlinux-5.10.fc.bin`
- Rootfs: `/srv/images/alpine-3.18.ext4`

**Kasus penggunaan**: Pengujian cepat, pipeline CI/CD

---

### Contoh 3: Web Server Production

**Nama**: `Production Web - Ubuntu`

**Konfigurasi**:
- vCPU: `4`
- Memori: `4096 MiB`
- Kernel: `/srv/images/vmlinux-5.10.fc.bin`
- Rootfs: `/srv/images/ubuntu-22.04.ext4`

**Kasus penggunaan**: Server aplikasi web production

---

### Contoh 4: Database Server

**Nama**: `PostgreSQL Server`

**Konfigurasi**:
- vCPU: `4`
- Memori: `8192 MiB`
- Kernel: `/srv/images/vmlinux-5.10.fc.bin`
- Rootfs: `/srv/images/ubuntu-22.04-postgres.ext4`

**Kasus penggunaan**: Instance database dengan PostgreSQL yang sudah terpasang

---

## Aturan Validasi

Formulir memvalidasi input sebelum membuat:

### Nama Template
- ❌ Tidak boleh kosong
- ✅ Harus unik
- ✅ Semua karakter diperbolehkan
- ✅ Disarankan: 3-50 karakter

### vCPU
- ❌ Harus berupa integer
- ❌ Minimum: 1
- ❌ Maksimum: 32
- ✅ Default: 1

### Memori (MiB)
- ❌ Harus berupa integer
- ❌ Minimum: 128 MiB
- ❌ Maksimum: 16384 MiB (16 GB)
- ✅ Default: 512 MiB

### Kernel
- ❌ Harus menyediakan path ATAU ID image
- ✅ Format path: `/srv/images/filename.bin`
- ✅ Format UUID untuk ID image

### Rootfs
- ❌ Harus menyediakan path ATAU ID image
- ✅ Format path: `/srv/images/filename.ext4`
- ✅ Format UUID untuk ID image

---

## Error Umum

### Error: "Template name cannot be empty"

**Penyebab**: Nama tidak dimasukkan

**Solusi**: Masukkan nama template yang deskriptif

---

### Error: "vCPU must be between 1 and 32"

**Penyebab**: Jumlah CPU tidak valid

**Solusi**:
- Masukkan angka antara 1-32
- Gunakan hanya nilai integer (tanpa desimal)

---

### Error: "Memory must be between 128 and 16384 MiB"

**Penyebab**: Alokasi memori tidak valid

**Solusi**:
- Masukkan memori dalam MiB (bukan MB atau GB)
- Gunakan nilai 128-16384
- Contoh: Untuk 2 GB, gunakan `2048` MiB

---

### Error: "Must provide kernel path or image ID"

**Penyebab**: Kedua field kernel kosong

**Solusi**:
- Masukkan path file kernel: `/srv/images/vmlinux-5.10.fc.bin`
- ATAU masukkan ID image kernel dari registri

---

### Error: "Must provide rootfs path or image ID"

**Penyebab**: Kedua field rootfs kosong

**Solusi**:
- Masukkan path file rootfs: `/srv/images/ubuntu-22.04.ext4`
- ATAU masukkan ID image rootfs dari registri

---

### Error: "Failed to create template"

**Kemungkinan penyebab**:
- Backend API tidak berjalan
- Masalah koneksi database
- Path image tidak valid
- Masalah konektivitas jaringan

**Solusi**:
1. Periksa manager sedang berjalan: `ps aux | grep manager`
2. Verifikasi path ada: `ls /srv/images/`
3. Periksa konsol browser untuk error detail
4. Coba lagi setelah beberapa detik

---

## Setelah Membuat

### Verifikasi Template

Setelah dibuat, verifikasi template Anda muncul dalam daftar:

![Image: Template in list](/images/templates/template-in-list.png)

**Periksa**:
- Nama template sudah benar
- vCPU dan RAM ditampilkan dengan benar
- Tanggal pembuatan adalah hari ini
- Tombol Deploy tersedia

---

### Deploy VM Pertama Anda

Uji template Anda dengan men-deploy VM:

1. Klik **"Deploy VM"** pada kartu template
2. Masukkan nama VM (mis., `test-from-template`)
3. Klik **"Deploy VM"**
4. Tunggu ~30 detik hingga VM mulai
5. Verifikasi VM sedang berjalan

Lihat [Kelola Template](manage-templates/) untuk detail deployment.

---

### Edit Jika Diperlukan

Jika Anda perlu mengubah template:

1. Klik kartu template (fitur mendatang)
2. Klik tombol **"Edit"**
3. Ubah pengaturan
4. Simpan perubahan

**Catatan**: Perubahan hanya memengaruhi deployment mendatang, bukan VM yang sudah ada.

---

## Praktik Terbaik

### 1. Uji Sebelum Menyimpan

**Sebelum membuat template**:
1. Buat VM uji secara manual dengan konfigurasi yang sama
2. Verifikasi kombinasi kernel + rootfs berfungsi
3. Periksa VM dapat boot dan berjalan dengan benar
4. Kemudian buat template dengan pengaturan tersebut

Ini mencegah deployment VM yang rusak dari template.

---

### 2. Gunakan Nama yang Deskriptif

**Sertakan dalam nama**:
- Sistem operasi (Ubuntu, Alpine, dll.)
- Tujuan (Dev, Prod, Test)
- Fitur khusus (with Docker, with PostgreSQL)
- Tingkatan sumber daya (1 vCPU, 4 vCPU)

**Contoh**: `Ubuntu 22.04 - Dev - 2vCPU` lebih baik dari `template1`

---

### 3. Dokumentasikan Template Anda

Catat tentang:
- Perangkat lunak apa yang sudah terpasang di rootfs
- Versi kernel yang digunakan
- Kasus penggunaan yang diharapkan
- Konfigurasi khusus yang diperlukan setelah deployment

---

### 4. Atur Berdasarkan Lingkungan

Buat set template untuk lingkungan yang berbeda:

**Development**:
- Sumber daya lebih rendah (1-2 vCPU, 512-2048 MiB)
- OS yang sama dengan production
- Prioritas deployment cepat

**Staging**:
- Cocokkan sumber daya production
- Image yang sama dengan production
- Untuk pengujian pra-production

**Production**:
- Sumber daya lebih tinggi (4+ vCPU, 4096+ MiB)
- Image yang stabil dan telah diuji
- Terdokumentasi dan berversi

---

### 5. Perbarui Template Secara Berkala

Tinjau dan perbarui template secara berkala:
- Perbarui ke versi kernel terbaru
- Segarkan image rootfs dengan patch keamanan
- Sesuaikan alokasi sumber daya berdasarkan penggunaan
- Hapus template yang tidak digunakan

---

## Referensi Cepat

### Daftar Periksa Pembuatan Template

Sebelum mengklik "Create Template":

- [ ] Nama template deskriptif dan unik
- [ ] Jumlah vCPU sudah diatur (1-32)
- [ ] Memori diatur dalam MiB (128-16384)
- [ ] Path atau ID kernel disediakan
- [ ] Path atau ID rootfs disediakan
- [ ] Image ada dan dapat diakses
- [ ] Alokasi sumber daya sesuai untuk kasus penggunaan
- [ ] Konfigurasi telah diuji dengan VM manual

---

### Pintasan Keyboard

| Aksi | Pintasan |
|--------|----------|
| Buka dialog buat | Klik tombol "Create Template" |
| Pindah antar field | Tab |
| Kirim formulir | Enter (saat tombol terfokus) |
| Batal | Esc |

---

## Langkah Selanjutnya

- **[Kelola Template](manage-templates/)** - Deploy VM, edit, dan hapus template
- **[Ikhtisar Template](./)** - Pelajari lebih lanjut tentang template
- **[Buat VM](/docs/vm/create-vm/)** - Panduan pembuatan VM manual
- **[Image Registry](/docs/operations/image-registry/)** - Kelola image kernel dan rootfs

---

## Topik Terkait

- **Pembuatan VM** - Template vs pembuatan VM manual
- **Manajemen Image** - Menggunakan registri image dengan template
- **Perencanaan Sumber Daya** - Menentukan ukuran CPU dan memori yang tepat
