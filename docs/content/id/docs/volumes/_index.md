+++
title = "Volume"
description = "Panduan lengkap untuk mengelola volume penyimpanan bagi VM melalui antarmuka web"
weight = 90
date = 2025-01-13
+++

Volume adalah perangkat penyimpanan blok persisten yang dapat dilampirkan ke VM untuk penyimpanan tambahan di luar sistem berkas root.

---

## Apa itu Volume?

Volume adalah file `.ext4` yang dialokasikan pada host dan muncul sebagai perangkat blok di dalam VM. Berbeda dengan sistem berkas root, volume bersifat persisten secara independen — Anda dapat melepas volume dari satu VM dan melampirkannya ke VM lain tanpa kehilangan data.

**Kasus penggunaan umum**:

- **Penyimpanan basis data** — Simpan data PostgreSQL/MySQL di volume terpisah agar tetap ada meski VM dibangun ulang
- **Penyimpanan bersama** — Lampirkan volume yang sama (hanya baca) ke beberapa VM
- **Ruang kerja pengembangan** — Pisahkan volume kode dari volume OS

---

## Tipe Volume

| Tipe | Keterangan |
|---|---|
| **EXT4** | Sistem berkas Linux standar, direkomendasikan untuk sebagian besar beban kerja |
| **Rootfs** | Volume sistem berkas root, otomatis terdaftar saat VM dibuat |

---

## Mulai Cepat

1. Buka **Volumes** di sidebar
2. Klik **Create Volume**, isi formulir, dan klik **Create Volume**
3. Buka halaman detail VM → tab **Storage** → **Add Drive** untuk melampirkan volume

---

## Langkah Berikutnya

- **[Jelajahi Volume](browse-volumes/)** — Cari dan eksplorasi volume yang sudah ada
- **[Buat Volume](create-volumes/)** — Tambahkan volume penyimpanan baru
- **[Kelola Volume](manage-volumes/)** — Lampirkan, lepas, dan hapus volume
