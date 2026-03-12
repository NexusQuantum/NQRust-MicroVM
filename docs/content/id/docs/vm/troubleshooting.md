+++
title = "Pemecahan Masalah"
description = "Diagnosa dan selesaikan masalah VM yang umum"
weight = 60
date = 2025-01-08
+++

Referensi untuk mendiagnosa dan menyelesaikan masalah umum dalam pembuatan VM, manajemen, akses, dan snapshots.

---

## Pembuatan VM

### Dropdown Kernel atau Rootfs Kosong

**Masalah**: Tidak ada opsi yang muncul di dropdown pemilihan kernel atau rootfs saat pembuatan VM.

**Solusi**:
1. Pergi ke **Image Registry** di sidebar
2. Unggah image kernel dan rootfs yang diperlukan (lihat [Upload Images](../registry/upload-images/))
3. Kembali ke pembuatan VM — dropdown akan terisi sekarang

---

### Sumber Daya Tidak Cukup

**Masalah**: Pesan error "Insufficient resources available" saat membuat VM.

**Solusi**:
- Kurangi alokasi vCPU atau memori
- Hentikan VM yang tidak digunakan untuk membebaskan sumber daya host
- Pergi ke halaman **Hosts** untuk memeriksa kapasitas yang tersedia
- Hubungi administrator Anda untuk menyediakan kapasitas tambahan

---

### VM Tersangkut di Status "Creating"

**Masalah**: VM menampilkan "Creating" lebih dari 30 detik.

**Solusi**:
1. Refresh halaman browser
2. Pergi ke **Hosts** dan verifikasi agent **Online** (indikator hijau)
3. Jika masih tersangkut, hapus VM dan buat ulang
4. Hubungi administrator Anda jika masalah berlanjut

---

## Manajemen VM

### Tidak Bisa Start VM

**Gejala**: Tombol Start tidak merespons, VM tersangkut di status transisi, atau notifikasi error muncul.

**Solusi**:

1. **Periksa sumber daya host** — pergi ke **Hosts** dan verifikasi host memiliki CPU dan memori yang tersedia; hentikan VM lain jika host kelebihan beban
2. **Verifikasi images ada** — pergi ke **Registry → Images** dan konfirmasi kernel dan rootfs ada; unggah ulang jika hilang
3. **Periksa status agent** — di halaman **Hosts**, konfirmasi agent **Online**; jika offline, hubungi administrator Anda
4. **Buat ulang VM** — hapus VM yang bermasalah dan buat yang baru dengan pengaturan yang sama

---

### Tombol Delete Dinonaktifkan

**Gejala**: Tombol Delete berwarna abu-abu; tooltip berbunyi "Cannot delete running VM".

![Disabled delete button with tooltip](/images/vm/vm-delete-disabled-running.png)

**Solusi**: Ini adalah perilaku yang diharapkan — VM yang sedang berjalan tidak dapat dihapus.

1. Klik **Stop** dan tunggu status berubah menjadi "Stopped"
2. Tombol **Delete** kini akan aktif

Pembatasan ini mencegah penghapusan tidak sengaja pada layanan aktif dan melindungi integritas data.

---

### VM Tidak Mau Berhenti

**Gejala**: Mengklik Stop tetapi VM masih menampilkan "Running" setelah 30 detik.

**Solusi**:

1. Refresh halaman (Ctrl+R / F5) dan periksa apakah statusnya sudah diperbarui
2. Tunggu 60 detik dan klik Stop lagi
3. Matikan secara graceful dari dalam VM terlebih dahulu:
   ```bash
   shutdown now
   ```
   Kemudian klik Stop setelah ~10 detik
4. Buka halaman detail VM dan periksa error yang ditampilkan

---

### VM yang Dijeda Tidak Bisa Resume

**Gejala**: Tombol Resume tidak berpengaruh, atau pesan error muncul.

**Solusi**:

1. Refresh browser dan coba Resume lagi
2. Hentikan VM yang dijeda, tunggu status "Stopped", lalu Start
3. Buka DevTools browser (F12 → Console) dan cari pesan error untuk dibagikan kepada administrator Anda
4. Jika snapshot ada, hapus VM dan pulihkan dari snapshot

---

## Akses Console & SSH

### Tidak Bisa Terhubung ke Console

**Masalah**: Console menampilkan "Connection failed" atau layar kosong.

**Solusi**:
1. Verifikasi VM dalam status **Running**
2. Refresh halaman browser
3. Periksa browser console (F12) untuk error JavaScript
4. Coba browser yang berbeda (Chrome, Firefox, Edge)
5. Konfirmasi koneksi WebSocket tidak diblokir oleh firewall atau proxy
6. Nonaktifkan ekstensi browser sementara

---

### SSH Connection Refused

**Masalah**: `ssh: connect to host <ip> port 22: Connection refused`

**Solusi**:
1. Verifikasi VM sedang berjalan dan memiliki alamat IP
2. Ping IP: `ping <vm-ip>`
3. Periksa layanan SSH sedang berjalan di dalam VM (melalui console):
   ```bash
   # Alpine
   rc-service sshd status

   # Ubuntu/Debian
   systemctl status sshd
   ```
4. Periksa aturan firewall di dalam VM

---

### SSH Permission Denied

**Masalah**: `Permission denied (publickey)`

**Solusi**:
1. Konfirmasi SSH key dikonfigurasi saat pembuatan VM
2. Periksa Anda menggunakan key yang benar:
   ```bash
   ssh -v root@<vm-ip>
   ```
3. Coba autentikasi kata sandi (jika diaktifkan):
   ```bash
   ssh -o PreferredAuthentications=password root@<vm-ip>
   ```
4. Buat ulang VM dengan SSH key yang benar

---

### Console Lambat atau Lag

**Masalah**: Penundaan input yang terasa di web console.

**Solusi**:
- Gunakan SSH sebagai pengganti web console untuk performa yang lebih baik
- Periksa latensi jaringan Anda ke server
- Tutup tab browser lain untuk membebaskan sumber daya
- Coba console dalam mode private/incognito

---

### Tidak Bisa Tempel di Console

**Masalah**: Ctrl+V tidak berfungsi di web console.

**Solusi**: Gunakan `Ctrl+Shift+V`, atau klik kanan dan pilih **Paste**. Beberapa browser juga mendukung klik tengah untuk tempel.

---

## Snapshots

### Pembuatan Snapshot Gagal

**Masalah**: Error muncul saat mencoba membuat snapshot.

**Solusi**:
1. Periksa ruang disk yang tersedia di host
2. Pastikan VM dalam status yang stabil (tidak sedang boot atau transisi)
3. Coba hentikan VM sebelum mengambil snapshot
4. Hubungi administrator Anda jika ruang disk habis

---

### Pemulihan Membutuhkan Waktu Terlalu Lama

**Masalah**: Pemulihan terlihat tersangkut atau sangat lambat.

**Solusi**:
1. Tunggu — VM yang besar bisa membutuhkan beberapa menit untuk dipulihkan
2. Periksa koneksi jaringan Anda ke server
3. Refresh browser setelah 5 menit dan periksa status VM
4. Hubungi administrator Anda jika operasi melebihi 10 menit

---

### Tidak Bisa Menghapus Snapshot

**Masalah**: Tombol Delete berwarna abu-abu.

**Solusi**:
1. Periksa apakah snapshot sedang digunakan
2. Hentikan VM yang bergantung pada snapshot
3. Tunggu operasi lain selesai, lalu refresh halaman
4. Verifikasi Anda memiliki izin yang diperlukan

---

### Snapshot Hilang

**Masalah**: Snapshot yang diharapkan tidak terlihat di daftar.

**Solusi**:
1. Refresh halaman browser
2. Konfirmasi Anda melihat VM yang benar
3. Periksa halaman **All Snapshots**
4. Verifikasi snapshot tidak dihapus secara otomatis atau dihapus oleh anggota tim

---

## Mendapatkan Bantuan

Jika tidak ada solusi di atas yang menyelesaikan masalah Anda:

1. Periksa halaman **Hosts** untuk mengonfirmasi semua agent online
2. Tinjau DevTools browser (F12) untuk error
3. Hubungi administrator platform Anda dengan nama VM, pesan error, dan output browser console
