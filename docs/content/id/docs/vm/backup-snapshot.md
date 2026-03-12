+++
title = "Backup & Snapshot"
description = "Buat backup dan pulihkan VM menggunakan snapshots"
weight = 44
date = 2025-12-16
+++

Pelajari cara melindungi VM Anda dengan snapshots untuk backup dan pemulihan yang cepat.

---

## Apa itu Snapshot?

Snapshot menangkap status lengkap VM pada suatu titik waktu tertentu:

- **Status VM Penuh**: Memori, disk, dan konfigurasi
- **Pembuatan Instan**: Hanya membutuhkan beberapa detik
- **Pemulihan Cepat**: Pulihkan VM dalam hitungan detik
- **Beberapa Snapshot**: Simpan beberapa titik backup

### Kasus Penggunaan

**Sebelum Perubahan Berisiko**:
```
Create Snapshot → Make Changes → Success? Keep | Failure? Restore
```

**Backup Rutin**:
- Snapshot harian untuk VM produksi
- Sebelum pembaruan sistem
- Sebelum deployment aplikasi

**Pengujian & Pengembangan**:
- Simpan status bersih sebelum pengujian
- Pulihkan ke status bersih di antara pengujian
- Bereksperimen dengan aman

**Pemulihan Bencana**:
- Pemulihan cepat dari kegagalan
- Rollback dari pembaruan yang buruk
- Pemulihan dari penghapusan tidak sengaja

---

## Membuat Snapshot

### Langkah 1: Navigasi ke VM

1. Pergi ke halaman **Virtual Machines**
2. Klik VM yang ingin Anda snapshot
3. Klik tab **Snapshots**

![Snapshots tab](/images/vm/vm-snapshots-tab.png)

### Langkah 2: Buat Snapshot

Klik tombol **Create Snapshot**:

![Create Snapshot button](/images/vm/create-snapshot-button.png)

Dialog akan muncul:

![Create snapshot dialog](/images/vm/create-snapshot-dialog.png)

### Langkah 3: Masukkan Detail Snapshot

**Nama Snapshot**:
- Gunakan nama yang deskriptif
- Sertakan tanggal/waktu atau tujuan
- Contoh:
  - `before-upgrade-2025-12-16`
  - `clean-install`
  - `before-database-migration`
  - `daily-backup-20251216`

**Deskripsi** (Opsional):
```
Before upgrading to PostgreSQL 15
Installed packages: postgresql-14, nginx, nodejs
```

### Langkah 4: Buat

Klik **Create** untuk memulai proses snapshot:

![Snapshot creating progress](/images/vm/snapshot-creating.png)

**Yang terjadi**:
1. Status VM dijeda sebentar
2. Isi memori disimpan
3. Status disk ditangkap
4. VM dilanjutkan secara otomatis

**Waktu**: Biasanya 5-15 detik tergantung ukuran VM

### Langkah 5: Snapshot Dibuat

Snapshot baru muncul di daftar:

![Snapshot list](/images/vm/snapshot-list.png)

Anda akan melihat:
- Nama snapshot
- Tanggal/waktu pembuatan
- Ukuran (disk + memori)
- Tindakan (Restore, Delete)

## Memulihkan dari Snapshot

**Peringatan**: Memulihkan menggantikan status VM saat ini dengan snapshot!

### Sebelum Memulihkan

**Pertimbangan penting**:
- Data VM saat ini akan hilang
- VM akan kembali ke waktu snapshot
- Buat snapshot baru dari status saat ini jika diperlukan
- Hentikan VM sebelum memulihkan (direkomendasikan)

### Proses Pemulihan

1. Pergi ke tab **Snapshots** VM
2. Temukan snapshot yang ingin Anda pulihkan
3. Klik tombol **Restore**

![Restore button on snapshot entry](/images/vm/snapshot-restore-button.png)

4. Konfirmasi pemulihan:

![Restore confirmation dialog](/images/vm/restore-confirm-dialog.png)

**Pesan konfirmasi**:
```
⚠️  Warning: This will restore VM to snapshot state.
Current data will be lost. This cannot be undone.

Snapshot: before-upgrade-2025-12-16
Created: 2025-12-16 10:30:00

Type VM name to confirm: my-vm
```

5. Ketik nama VM dan klik **Confirm Restore**

### Progres Pemulihan

Sistem akan:
1. Menghentikan VM (jika berjalan)
2. Menggantikan disk dengan snapshot
3. Memulihkan status memori
4. Memulai ulang VM

**Waktu**: Biasanya 10-30 detik

### Verifikasi Pemulihan

Setelah pemulihan:

1. Periksa VM dalam status "Running"
2. Akses console dan verifikasi data
3. Uji bahwa semuanya berfungsi seperti yang diharapkan
4. Periksa timestamp — harus sesuai dengan waktu snapshot

**Contoh verifikasi**:
```bash
# Check system uptime (should show recent boot)
uptime

# Check file timestamps
ls -la /var/log/

# Verify applications are running
ps aux | grep nginx
```

---

## Mengelola Snapshot

### Mengganti Nama Snapshot

1. Klik menu **⋮** di sebelah snapshot
2. Pilih **Rename**
3. Masukkan nama baru
4. Klik **Save**

### Menghapus Snapshot

**Perhatian**: Snapshot yang dihapus tidak dapat dipulihkan!

1. Klik menu **⋮** di sebelah snapshot
2. Pilih **Delete**
3. Konfirmasi penghapusan

**Yang terjadi**:
- Snapshot dihapus secara permanen
- Ruang disk dibebaskan
- Tidak dapat dipulihkan setelah dihapus
- VM tidak terpengaruh

---

## Jenis Snapshot

### Snapshot Penuh

Menangkap status VM secara lengkap:
- ✅ Semua data disk
- ✅ Isi memori
- ✅ Konfigurasi
- ✅ Titik pemulihan independen

**Ukuran**: Sesuai ukuran disk + memori VM

**Gunakan saat**: Membuat titik backup utama

### Snapshot Inkremental

Menangkap hanya perubahan sejak snapshot terakhir:
- ✅ Perubahan sejak snapshot induk
- ✅ Ukuran lebih kecil
- ✅ Pembuatan lebih cepat
- ⚠️ Memerlukan snapshot induk

**Ukuran**: Hanya data yang berubah

**Gunakan saat**: Backup rutin untuk VM yang sama

---

## Praktik Terbaik

### Penamaan Snapshot

**Nama yang baik**:
```
before-update-2025-12-16
after-install-postgres
clean-os-install
production-daily-20251216-0300
pre-migration-backup
```

**Nama yang buruk**:
```
snapshot1
backup
test
20251216
```

### Frekuensi Snapshot

**VM Produksi**:
- Snapshot harian pada jam sepi
- Sebelum perubahan apapun
- Simpan 7 snapshot harian terakhir
- Snapshot jangka panjang bulanan

**VM Pengembangan**:
- Sebelum perubahan besar
- Setelah konfigurasi yang berhasil
- Snapshot status bersih
- Simpan 2-3 snapshot terbaru

**VM Pengujian**:
- Sebelum setiap siklus pengujian
- Status baseline yang bersih
- Hapus setelah pengujian selesai

### Retensi Snapshot

**Kebijakan retensi yang direkomendasikan**:

| Jenis Snapshot | Simpan Selama | Contoh |
|----------------|---------------|--------|
| Harian | 7 hari | Backup minggu lalu |
| Mingguan | 4 minggu | Bulan lalu |
| Bulanan | 3-12 bulan | Arsip triwulanan |
| Sebelum Perubahan | Sampai diverifikasi | 1-2 minggu |

**Hapus snapshot lama**:
- Bebaskan ruang disk
- Kurangi kekacauan
- Fokus pada backup penting
- Otomatiskan pembersihan jika memungkinkan

### Manajemen Penyimpanan

**Pantau penyimpanan snapshot**:
1. Pergi ke halaman **Snapshots**
2. Periksa total ukuran
3. Tinjau penggunaan penyimpanan

**Optimalkan penyimpanan**:
- Hapus snapshot yang tidak diperlukan
- Gunakan snapshot inkremental
- Kompres snapshot lama
- Arsipkan ke penyimpanan eksternal

---

## Pemulihan Bencana

### Rencana Pemulihan

**Skenario**: VM crash dan tidak bisa boot

**Langkah pemulihan**:

1. **Kaji kerusakan**:
   - Coba restart VM
   - Periksa pesan error
   - Identifikasi status baik yang terakhir diketahui

2. **Temukan snapshot terbaru**:
   - Pergi ke tab Snapshots VM
   - Identifikasi snapshot yang berfungsi paling baru
   - Catat data apa yang akan hilang

3. **Pulihkan snapshot**:
   - Hentikan VM yang gagal
   - Klik Restore pada snapshot yang dipilih
   - Konfirmasi pemulihan
   - Tunggu hingga selesai

4. **Verifikasi pemulihan**:
   - Periksa VM berhasil dimulai
   - Uji layanan penting
   - Verifikasi integritas data
   - Dokumentasikan apa yang hilang

5. **Cegah pengulangan**:
   - Identifikasi penyebab kegagalan
   - Implementasikan perbaikan
   - Buat snapshot baru dari status yang diperbaiki

### Uji Pemulihan

**Latihan bulanan**:
1. Pilih VM yang tidak kritis
2. Buat snapshot pengujian
3. Lakukan beberapa perubahan
4. Pulihkan dari snapshot
5. Verifikasi pemulihan berhasil
6. Hapus snapshot pengujian

**Mengapa perlu diuji?**:
- Verifikasi backup valid
- Latih proses pemulihan
- Bangun kepercayaan diri
- Temukan masalah sebelum darurat

---

## Pemecahan Masalah

### Masalah: Pembuatan Snapshot Gagal

**Masalah**: Pesan error saat membuat snapshot

**Solusi**:
1. Periksa ruang disk yang tersedia di host
2. Pastikan VM dalam status yang stabil
3. Coba hentikan VM terlebih dahulu, lalu buat snapshot
4. Kurangi frekuensi snapshot
5. Hubungi administrator jika disk penuh

---

### Masalah: Pemulihan Membutuhkan Waktu Terlalu Lama

**Masalah**: Pemulihan tersangkut atau sangat lambat

**Solusi**:
1. Tunggu — VM yang besar membutuhkan waktu lebih lama (bisa beberapa menit)
2. Periksa koneksi jaringan ke server
3. Refresh browser setelah 5 menit
4. Periksa status VM secara langsung
5. Hubungi administrator jika > 10 menit

---

### Masalah: Tidak Bisa Menghapus Snapshot

**Masalah**: Tombol Delete berwarna abu-abu

**Solusi**:
1. Periksa apakah snapshot sedang digunakan
2. Hentikan VM yang bergantung pada snapshot
3. Tunggu operasi lain selesai
4. Refresh halaman
5. Periksa izin

---

### Masalah: Snapshot Hilang

**Masalah**: Snapshot yang diharapkan tidak ada di daftar

**Solusi**:
1. Refresh halaman browser
2. Periksa Anda melihat VM yang benar
3. Periksa halaman All Snapshots
4. Verifikasi snapshot tidak dihapus secara otomatis
5. Tanyakan kepada tim apakah ada yang menghapusnya

---

## Tips Lanjutan

### Daftar Periksa Pra-Snapshot

Sebelum membuat snapshot penting:

```bash
# In VM console/SSH

# 1. Stop services gracefully
systemctl stop nginx
systemctl stop postgresql

# 2. Sync filesystem
sync

# 3. Clear cache (optional)
sync; echo 3 > /proc/sys/vm/drop_caches

# 4. Create marker file
echo "Snapshot created at $(date)" > /root/snapshot-$(date +%Y%m%d).txt
```

Kemudian buat snapshot.

**Mengapa?**:
- Memastikan status yang konsisten
- Mencegah korupsi
- Membuat pemulihan lebih bersih

### Metadata Snapshot

Tambahkan metadata yang berguna dalam deskripsi:

```
Created: 2025-12-16 15:30:00
Purpose: Before PostgreSQL 15 upgrade
Installed: PostgreSQL 14.5, Nginx 1.24, Node.js 20
Services Running: web-api, background-worker
IP Address: 192.168.1.100
Last Updated: 2025-12-15
```

Membantu mengidentifikasi titik pemulihan yang benar di kemudian hari!

---

## Langkah Berikutnya

- **[Pemantauan](monitoring/)** - Pantau performa VM
- **[Kelola VM](manage-vm/)** - Operasi siklus hidup VM
- **[Buat VM](create-vm/)** - Buat VM baru
