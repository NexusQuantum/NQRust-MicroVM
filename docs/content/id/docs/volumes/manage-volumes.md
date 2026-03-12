+++
title = "Kelola Volume"
description = "Lampirkan, lepas, hapus, dan organisir volume penyimpanan melalui antarmuka web"
weight = 73
date = 2025-01-13
+++

Lampirkan volume ke VM, lepas dengan aman, dan hapus penyimpanan yang tidak terpakai.

---

## Melampirkan Volume ke VM

Volume dilampirkan dari halaman detail VM, bukan dari daftar Volumes.

1. Buka **Virtual Machines** dan buka VM yang ingin Anda tambahkan penyimpanannya
2. Klik tab **Storage**
3. Klik **Add Drive**

![VM Storage tab showing attached drives](/images/volumes/vm-storage-tab.png)

Tab Storage menampilkan tabel **Attached Drives** dengan kolom:

| Kolom | Keterangan |
|---|---|
| **Drive ID** | Pengenal (mis. `rootfs`) dengan lencana `Default` untuk drive root |
| **Path** | Jalur lengkap ke file volume pada host |
| **Size** | Ukuran volume |
| **Root Device** | Lencana `Root` jika ini adalah drive booting |
| **Read Only** | Apakah drive dipasang sebagai hanya baca |
| **Actions** | Tombol Detach (tidak tersedia untuk drive root) |

### Mode pelampiran

- **Read-Write** (default) — VM dapat membaca dan menulis dengan bebas
- **Read-Only** — VM hanya dapat membaca; berguna untuk data referensi bersama

### Setelah melampirkan

Hentikan dan mulai ulang VM jika sedang berjalan, lalu pasang volume di dalamnya:

```bash
# Find the new block device
lsblk

# Mount it
sudo mkdir -p /mnt/data
sudo mount /dev/vdb /mnt/data

# Make permanent via /etc/fstab
sudo blkid /dev/vdb
# Add: UUID=... /mnt/data ext4 defaults 0 2
```

---

## Melepas Volume

**Sebelum melepas**, lepas pasang volume di dalam VM dan hentikan VM:

```bash
# Stop any apps using the volume
sudo systemctl stop myapp

# Unmount
sudo umount /mnt/data

# Confirm unmounted
mount | grep /mnt/data
```

Lalu klik ikon detach di kolom **Actions** pada tab Storage.

> Drive root (`Default`) tidak dapat dilepas.

Volume yang dilepas kembali ke status **Available** dan dapat dilampirkan ke VM yang berbeda.

---

## Menghapus Volume

Buka **Volumes** di sidebar, temukan volume tersebut, dan klik **Delete**.

**Persyaratan sebelum menghapus**:
- Volume tidak boleh terlampir ke VM mana pun (periksa kolom VMs menampilkan `0`)
- Cadangkan semua data yang ingin Anda simpan — penghapusan bersifat permanen

---

## Tugas Umum

### Memindahkan volume antar VM

1. Hentikan VM1
2. Lepas pasang volume di dalam VM1
3. Lepas dari VM1 melalui tab Storage
4. Buka tab Storage VM2 → Add Drive → pilih volume
5. Mulai VM2 dan pasang volume

### Berbagi data hanya baca antar VM

Lampirkan volume yang sama ke beberapa VM dalam mode **Read-Only**. Setiap VM dapat membaca data; tidak ada yang dapat mengubahnya.

### Membebaskan penyimpanan server

1. Buka **Volumes**, filter untuk menampilkan hanya volume yang tidak terlampir
2. Urutkan berdasarkan ukuran untuk menemukan kandidat terbesar
3. Konfirmasikan dengan tim Anda, lalu hapus

---

## Pemecahan Masalah

### Volume tidak terlihat di dalam VM setelah dilampirkan

Hentikan dan mulai VM — hotplug mungkin tidak didukung untuk semua konfigurasi.

### Pemasangan gagal

```bash
# Check filesystem type
sudo blkid /dev/vdb

# Check if already mounted
mount | grep vdb

# Try specifying the type explicitly
sudo mount -t ext4 /dev/vdb /mnt/data
```

### Tidak dapat melepas

- Anda tidak dapat melepas drive root (`Default`)
- Hentikan VM terlebih dahulu, lalu lepas
- Pastikan volume sudah dilepas pasang di dalam VM

### Tidak sengaja menghapus volume

Hubungi administrator Anda segera — file mungkin masih dapat dipulihkan dari disk sebelum ditimpa.

---

## Langkah Berikutnya

- **[Buat Volume](create-volumes/)** — Tambahkan volume penyimpanan baru
- **[Jelajahi Volume](browse-volumes/)** — Cari dan filter semua volume
