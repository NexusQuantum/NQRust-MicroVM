+++
title = "Buat Volume"
description = "Tambahkan volume penyimpanan baru ke registry dan siapkan untuk dilampirkan ke VM"
weight = 72
date = 2025-01-13
+++

Buat volume penyimpanan blok persisten baru dan lampirkan ke VM.

---

## Membuat Volume

1. Buka **Volumes** di sidebar
2. Klik **Create Volume**
3. Isi formulir dan klik **Create Volume**

![Create Volume dialog](/images/volumes/volume-create-dialog.png)

### Kolom Formulir

**Name** *(wajib)*
Pengenal unik untuk volume. Gunakan nama deskriptif yang mencerminkan tujuannya.
```
postgres-data
web-uploads
dev-workspace-alice
```

**Description** *(opsional)*
Catatan singkat tentang kegunaan volume ini.

**Size (GB)** *(wajib)*
Ukuran yang akan dialokasikan dalam gigabyte. Ruang langsung dicadangkan di host.

- Minimum: 1 GB
- Rencanakan untuk pertumbuhan — volume tidak dapat diubah ukurannya setelah dibuat
- Ukuran umum: 10 GB (kecil), 50 GB (sedang), 100–500 GB (besar)

**Type** *(wajib)*
Saat ini `EXT4` — sistem berkas Linux standar, cocok untuk semua beban kerja.

**Host** *(wajib)*
Mesin host tempat file volume akan disimpan. Pilih dari dropdown host yang terdaftar. Volume hanya dapat dilampirkan ke VM yang berjalan pada host yang sama.

---

## Setelah Membuat

Volume baru muncul dalam daftar Volumes. Untuk menggunakannya, lampirkan ke VM dari tab **Storage** milik VM — lihat [Kelola Volume](manage-volumes/).

---

## Pemasangan di Dalam VM

Setelah dilampirkan, pasang volume di dalam VM:

```bash
# List block devices to find the new drive
lsblk

# Create a mount point
sudo mkdir -p /mnt/data

# Mount (usually /dev/vdb for the second drive)
sudo mount /dev/vdb /mnt/data

# Verify
df -h /mnt/data
```

**Jadikan permanen** — tambahkan ke `/etc/fstab`:
```bash
# Get the UUID
sudo blkid /dev/vdb

# Add to /etc/fstab:
UUID=your-uuid-here /mnt/data ext4 defaults 0 2
```

---

## Tips Penamaan

```
Bagus:
  postgres-data-prod
  web-uploads-staging
  dev-alice-workspace
  logs-archive-2025-01

Buruk:
  volume1
  test
  data
```

---

## Pemecahan Masalah

### Pembuatan gagal

- Periksa apakah host memiliki ruang disk yang tersedia
- Coba ukuran yang lebih kecil
- Pastikan nama bersifat unik
- Hubungi administrator Anda jika masalah berlanjut

---

## Langkah Berikutnya

- **[Kelola Volume](manage-volumes/)** — Lampirkan volume ke VM
- **[Jelajahi Volume](browse-volumes/)** — Temukan volume yang sudah ada
