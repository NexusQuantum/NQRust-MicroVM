+++
title = "Quick Start"
description = "Masuk dan buat microVM pertama Anda"
weight = 3
date = 2025-12-01

[extra]
toc = true
+++

Panduan ini mengasumsikan Anda telah menyelesaikan [Instalasi](../installation/). Semua layanan sudah berjalan — tidak ada yang perlu dijalankan secara manual.

---

## Buka Web UI

Navigasi ke host Anda di browser. URL ditampilkan di akhir output installer, biasanya:

```
http://<your-host-ip>
```

Masuk dengan kredensial default:

- **Nama Pengguna:** `root`
- **Kata Sandi:** `root`

> Segera ganti kata sandi Anda melalui **Settings → Account**.

---

## Unggah Image VM

Sebelum membuat VM, Anda memerlukan kernel Linux dan image filesystem root.

1. Buka **Image Registry** di sidebar
2. Klik **Import Image**
3. Unggah kernel (`.bin`) dan rootfs (`.ext4`)

Lihat [Registri Image](../../registry/) untuk instruksi unggah yang lebih detail dan sumber image yang kompatibel.

---

## Buat VM Pertama Anda

1. Buka **Virtual Machines** di sidebar
2. Klik **Create VM**
3. Isi langkah-langkah wizard:

| Langkah | Yang perlu diisi |
|---|---|
| **Basic** | Nama (contoh: `my-first-vm`), deskripsi opsional |
| **Credentials** | Kata sandi root untuk VM |
| **Machine** | vCPU: `1`, Memori: `512` MB |
| **Boot** | Pilih kernel dan rootfs yang telah diunggah |
| **Network** | Biarkan default — bridge `fcbr0`, Allow MMDS diaktifkan |
| **Review** | Konfirmasi pengaturan dan klik **Create** |

4. Di halaman detail VM, klik **Start**
5. Tunggu hingga lencana status berubah menjadi **Running**

---

## Akses VM

Klik tab **Terminal** pada halaman detail VM. Konsol berbasis browser akan terbuka — masuk dengan `root` dan kata sandi yang telah Anda tetapkan.

Untuk akses SSH, periksa tab **Overview** untuk mendapatkan alamat IP VM, kemudian:
```bash
ssh root@<vm-ip>
```

---

## Langkah Selanjutnya

- **[Kelola VM](../../vm/manage-vm/)** — Mulai, hentikan, jeda, lanjutkan, hapus
- **[Snapshot](../../vm/backup-snapshot/)** — Simpan dan pulihkan status VM
- **[Jaringan](../../networks/)** — Buat jaringan virtual yang terisolasi
- **[Registri Image](../../registry/)** — Kelola kernel dan filesystem root
- **[Pengguna](../../users/)** — Tambahkan anggota tim dan tetapkan peran
