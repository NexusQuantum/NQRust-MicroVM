+++
title = "Jelajahi Volume"
description = "Cari, filter, dan eksplorasi volume penyimpanan yang tersedia di registry"
weight = 71
date = 2025-01-13
+++

Pelajari cara menjelajahi dan mencari volume yang tersedia di registry untuk menemukan yang tepat bagi VM Anda.

---

## Mengakses Penjelajah Volume

### Dari Halaman Volumes

Navigasikan ke halaman Volumes:

![Image: Volumes page](/images/volumes/browse-main-volumes.png)

1. Klik **"Volumes"** di sidebar (di bawah Networks)
2. Lihat semua volume yang tersedia di tabel

---


## Tampilan Tabel Volume

Tabel volume menampilkan informasi penting:

![Image: Volume table](/images/volumes/volume-table.png)

**Kolom**:
- **Name** - Nama tampilan volume
- **Type** - Data, Rootfs, atau Scratch
- **Size** - Ukuran file dalam MB/GB
- **Format** - ext4, qcow2, atau raw
- **Attached VMs** - Jumlah VM yang menggunakan volume ini
- **Created** - Waktu volume ditambahkan
- **Actions** - Operasi yang tersedia

---

## Mencari Volume

### Pencarian Dasar

Gunakan bilah pencarian untuk menemukan volume:

![Image: Search bar](/images/volumes/search-bar-volumes.png)

**Cari berdasarkan**:
- Nama volume
- Nama VM (menemukan volume yang terlampir ke VM tersebut)
- Tipe format
- Kata kunci

**Contoh**:
```
Search: "postgres"
→ Finds: postgres-data-prod, postgres-backup

Search: "web-server"
→ Finds: Volumes attached to web-server VM

Search: "ext4"
→ Finds: All ext4 volumes

Search: "data"
→ Finds: postgres-data, webapp-data, logs-data
```

---

### Tips Pencarian

**Jadilah spesifik**:
```
❌ Too vague: "volume"
✅ Better: "postgres data"
✅ Best: "postgres-data-prod"
```

**Cari berdasarkan nama VM**:
```
"web-server" → shows volumes attached to web-server
"database-01" → shows volumes for database-01 VM
```

**Cari berdasarkan tujuan**:
```
"backup" → finds backup volumes
"logs" → finds log storage volumes
"temp" → finds temporary volumes
```

---

## Memfilter Volume

### Filter berdasarkan Tipe

Gunakan dropdown filter tipe:

![Image: Type filter](/images/volumes/filter-type-volumes.png)

**Pilihan filter**:
- **All** - Tampilkan semua tipe
- **ext4** - Hanya volume ext4
- **qcow2** - Hanya volume qcow2
- **raw** - Hanya volume raw

---

### Filter berdasarkan Status

Filter volume berdasarkan status pelampiran:

![Image: Status filter](/images/volumes/filter-status.png)

**Pilihan filter**:
- **All** - Tampilkan semua volume
- **Attached** - Hanya volume yang terlampir ke VM
- **Available** - Hanya volume yang tidak terlampir

**Kasus penggunaan**:
- Temukan volume "Available" untuk digunakan kembali
- Periksa "Attached" untuk melihat apa yang sedang digunakan
- Identifikasi volume yang tidak terpakai untuk pembersihan

---


### Pemfilteran Gabungan

Gabungkan pencarian dan filter:

![Image: Combined filtering](/images/volumes/combined-filter.png)

**Contoh 1**: Temukan volume data yang tersedia
```
1. Set type filter to "EXT4"
2. Set status filter to "Available"
Result: Only unattached data volumes
```

**Contoh 2**: Temukan volume postgres
```
1. Set type filter to "EXT4"
2. Search for "postgres"
Result: Only data volumes with "postgres" in name
```

---

## Melihat Detail Volume

### Informasi Volume

Setiap baris volume menampilkan detail penting:

![Image: Volume row details](/images/volumes/volume-row.png)

**Informasi yang ditampilkan**:
- **Name**: Nama tampilan volume
- **Type badge**: Indikator tipe dengan kode warna
  - Biru untuk Data
  - Hijau untuk Rootfs
  - Ungu untuk Scratch
- **Size**: Ukuran file yang mudah dibaca
- **Format badge**: ext4, qcow2, atau raw
- **VM count**: Jumlah VM yang menggunakan volume ini
- **Date**: Waktu volume dibuat

---

### Tampilan Ukuran Volume

Ukuran diformat agar mudah dibaca:

```
< 1 GB:     "512 MB"
< 10 GB:    "2.5 GB"
< 100 GB:   "45 GB"
>= 100 GB:  "250 GB"
```

**Ukuran umum**:
- Rootfs: 2-20 GB
- Volume data: 10-500 GB
- Scratch: 5-100 GB
- Volume basis data: 50-1000 GB

---



## Kategori Volume

### Volume Data Aplikasi

Volume untuk penyimpanan aplikasi:

```
Database:
- postgres-data-prod
- mysql-data-staging
- redis-cache

Web Applications:
- webapp-uploads
- static-assets
- user-content

Logs:
- app-logs-2025
- nginx-logs
- system-logs
```

---

### Volume Sistem

Penyimpanan Rootfs dan sistem:

```
Operating Systems:
- ubuntu-22.04-base
- alpine-3.18-minimal
- debian-12-server

Specialized:
- container-runtime
- development-env
- production-base
```

---

### Volume Sementara

Penyimpanan Scratch dan sementara:

```
Computation:
- temp-processing
- scratch-space
- build-cache

Testing:
- test-data-temp
- dev-workspace
- experiment-storage
```

---

## Mengurutkan Volume

Volume dapat diurutkan dengan mengklik header kolom:

**Urutkan berdasarkan Name** (alfabetis):
```
app-logs
database-backup
postgres-data
webapp-uploads
```

**Urutkan berdasarkan Size** (terbesar dulu):
```
postgres-data (500 GB)
webapp-uploads (100 GB)
logs-archive (50 GB)
scratch-temp (10 GB)
```

**Urutkan berdasarkan Usage** (paling banyak digunakan dulu):
```
shared-assets (5 VMs)
postgres-data (3 VMs)
logs-archive (1 VM)
test-volume (0 VMs)
```

**Urutkan berdasarkan Date** (terbaru dulu):
```
new-data-volume (Today)
postgres-backup (Yesterday)
old-logs (Last month)
```

---

## Kondisi Kosong

### Tidak Ada Volume Ditemukan

Saat tidak ada volume yang cocok dengan pencarian Anda:

![Image: No results](/images/volumes/no-results-volumes.png)

**Pesan**: "No volumes found"

**Tindakan**:
1. Hapus kueri pencarian
2. Sesuaikan filter
3. Coba kata kunci yang berbeda
4. Buat volume baru jika diperlukan

---


## Tips Performa

### Navigasi Cepat

**Pintasan keyboard**:
- `Tab` - Pindah antara pencarian dan filter
- `Enter` - Pilih volume yang disorot
- `Escape` - Tutup modal browser
- `Arrow keys` - Navigasi daftar volume

**Pintasan mouse**:
- Klik nama volume untuk pilihan cepat
- Klik dua kali untuk pemilihan instan
- Arahkan untuk tooltip info cepat

---

### Pencarian Efisien

**Mulai luas, lalu persempit**:
```
Step 1: Filter to type (e.g., "Data")
Step 2: Filter to status (e.g., "Available")
Step 3: Search for name (e.g., "postgres")
```

**Gunakan awalan**:
```
"postgres" → finds PostgreSQL volumes
"web" → finds web application volumes
"log" → finds log storage volumes
```

**Simpan pencarian umum**:
Simpan catatan volume yang sering digunakan:
```
Production database: postgres-data-prod
Upload storage: webapp-uploads-prod
Log archive: logs-archive-2025
```

---

## Praktik Terbaik

### Menemukan Volume yang Tepat

- **Periksa jumlah VM**:
  - Jumlah nol = Tersedia untuk digunakan
  - Jumlah tinggi = Banyak digunakan (bersama atau rootfs)

- **Verifikasi ukuran**:
  - Sesuaikan dengan kebutuhan penyimpanan Anda
  - Pertimbangkan pertumbuhan
  - Periksa ruang yang tersedia

- **Tinjau format**:
  - ext4 untuk performa
  - qcow2 untuk penghematan ruang
  - raw untuk kesederhanaan

---

### Sebelum Memilih

- **Konfirmasi ketersediaan**:
  - Periksa apakah sudah terlampir
  - Verifikasi tidak digunakan oleh VM kritis
  - Pertimbangkan mode pelampiran

- **Periksa kapasitas**:
  - Ukuran yang cukup untuk kebutuhan
  - Ruang untuk pertumbuhan
  - Persyaratan performa

- **Verifikasi tujuan**:
  - Cocokkan volume dengan kasus penggunaan
  - Produksi vs. pengembangan
  - Sementara vs. persisten

---

## Pemecahan Masalah

### Masalah: Tidak Dapat Menemukan Volume yang Diharapkan

**Gejala**:
- Volume tidak ada di daftar
- Pencarian tidak mengembalikan hasil

**Kemungkinan penyebab**:
1. Volume belum dibuat
2. Filter menyembunyikan volume
3. Salah ketik dalam kueri pencarian

**Solusi**:
1. Hapus semua filter (atur ke "All")
2. Hapus kueri pencarian
3. Periksa ejaan
4. Verifikasi volume telah dibuat
5. Tanya administrator apakah volume ada

---

### Masalah: Terlalu Banyak Hasil

**Gejala**:
- Daftar volume yang panjang
- Sulit menemukan volume tertentu

**Solusi**:
1. Gunakan istilah pencarian yang spesifik
2. Terapkan filter tipe dan status
3. Urutkan berdasarkan kolom yang relevan
4. Gunakan nama VM dalam pencarian

---

### Masalah: Tujuan Volume Tidak Jelas

**Gejala**:
- Beberapa volume yang serupa
- Tidak tahu mana yang harus dipilih

**Solusi**:
1. Periksa jumlah VM (volume yang sedang digunakan)
2. Cari nama yang deskriptif
3. Tanya tim tentang volume standar
4. Periksa tanggal pembuatan volume
5. Tinjau ukuran volume (petunjuk tentang tujuan)

---

## Referensi Cepat

### Operator Pencarian

| Istilah Pencarian | Cocok dengan |
|-------------|---------|
| postgres | Volume apa pun dengan "postgres" di nama |
| web-server | Volume yang terlampir ke VM web-server |
| ext4 | Semua volume berformat ext4 |
| data | Volume apa pun dengan "data" di nama |

### Pilihan Filter

| Filter | Menampilkan |
|--------|-------|
| All Types | Semua tipe volume |
| Data | Hanya volume data |
| Rootfs | Hanya volume sistem berkas root |
| Scratch | Hanya volume sementara |
| Attached | Hanya volume yang sedang digunakan |
| Available | Hanya volume yang tidak terlampir |

### Pengurutan Kolom

| Kolom | Urutan Sortir |
|--------|------------|
| Name | Alfabetis (A-Z) |
| Size | Terbesar ke terkecil |
| VMs | Paling banyak digunakan ke paling sedikit |
| Created | Terbaru ke terlama |

---

## Langkah Berikutnya

- **[Buat Volume](create-volumes/)** - Tambahkan volume baru ke registry
- **[Kelola Volume](manage-volumes/)** - Lampirkan, lepas, dan organisir volume
- **[Users](/docs/users/)** - Kelola akun pengguna dan kontrol akses
- **[Ikhtisar Volume](../)** - Pelajari tipe volume
- **[Buat VM](/docs/vm/create-vm/)** - Gunakan volume saat membuat VM
