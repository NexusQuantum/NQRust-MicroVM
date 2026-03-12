+++
title = "Kelola Containers"
description = "Start, stop, restart, pause, resume, dan hapus container"
weight = 32
date = 2025-12-18
+++

Pelajari cara mengelola container Anda sepanjang siklus hidupnya — mulai dari menjalankan dan menghentikan hingga monitoring dan penghapusan.

---

## Mengakses Containers

### Dari Halaman Containers

Navigasi ke halaman containers utama:

![Image: Containers page navigation](/images/containers/manage-access-page.png)

1. Klik **"Containers"** di sidebar
2. Lihat semua container Anda dalam tabel

---

## Status Container

Container dapat berada dalam berbagai status yang ditandai dengan badge berwarna:

![Image: Container states badges](/images/containers/manage-states-badges.png)

| Status | Warna Badge | Deskripsi | Aksi Tersedia |
|--------|-------------|-----------|---------------|
| **Creating** | Kuning | VM sedang dibuat | Tunggu |
| **Booting** | Abu-abu | VM sedang booting | Tunggu |
| **Initializing** | Cyan | Docker daemon dimulai | Tunggu |
| **Running** | Hijau | Container aktif | Stop, Restart, Pause, Log, Shell |
| **Stopped** | Merah | Container dihentikan | Start, Delete |
| **Paused** | Kuning tua | Container dijeda | Resume, Stop |
| **Error** | Merah | Error deployment/runtime | Lihat log, Delete, Coba lagi |

**Transisi status**:
```
Creating → Booting → Initializing → Running
Running → Stopped (via Stop)
Running → Paused (via Pause)
Paused → Running (via Resume)
Status Apa Pun → Error (jika terjadi kegagalan)
```

---

## Filter dan Pencarian

### Cari Container

Gunakan kotak pencarian untuk menemukan container tertentu:

![Image: Search box](/images/containers/manage-search.png)

**Cari berdasarkan**:
- Nama container (mis., "nginx")
- Nama image (mis., "postgres")
- Pencocokan parsial (mis., "prod" menemukan semua container produksi)

**Contoh pencarian**:
```
"postgres"  → Menemukan semua container PostgreSQL
"prod"      → Menemukan prod-api, prod-db, nginx-prod
"alpine"    → Menemukan container yang menggunakan image Alpine
```

---

### Filter berdasarkan Status

Filter container berdasarkan status saat ini:

![Image: Status filter dropdown](/images/containers/manage-filter-status.png)

**Opsi filter**:
- **All Status** - Tampilkan semua container
- **Running** - Hanya container yang berjalan
- **Stopped** - Hanya container yang dihentikan
- **Creating** - Container yang sedang dibuat
- **Booting** - Container yang sedang booting
- **Initializing** - Container yang sedang diinisialisasi
- **Paused** - Container yang dijeda
- **Error** - Container dengan error

**Kasus penggunaan**:
- Temukan semua container yang berjalan untuk cek penggunaan resource
- Temukan container yang dihentikan untuk pembersihan
- Temukan container error untuk pemecahan masalah

---

## Tabel Container

Tabel container menampilkan informasi detail:

![Image: Container table with all columns](/images/containers/manage-table-full.png)

### Kolom Tabel

**1. Name**
- Nama container (tautan yang dapat diklik)
- Klik untuk membuka halaman detail container

![Image: Container name column](/images/containers/manage-column-name.png)

**2. Image**
- Docker image dan tag
- Ditampilkan dalam font monospace
- Contoh: `nginx:alpine`, `postgres:15`

![Image: Image column](/images/containers/manage-column-image.png)

**3. Status**
- Status saat ini dengan badge berwarna
- Warna: Hijau (Running), Merah (Stopped/Error), Kuning (Creating), Abu-abu (Booting), Cyan (Initializing), Kuning tua (Paused)

![Image: Status column](/images/containers/manage-column-status.png)

**4. Uptime**
- Sudah berapa lama container berjalan
- Format: "2h 30m", "5d 12h", "Never" (jika belum pernah dimulai)
- Hanya ditampilkan untuk container yang berjalan

![Image: Uptime column](/images/containers/manage-column-uptime.png)

**5. CPU**
- Alokasi vCPU
- Contoh: "0.5 vCPU", "2 vCPU", "4 vCPU"

![Image: CPU column](/images/containers/manage-column-cpu.png)

**6. Memory**
- Alokasi memory dalam MB
- Contoh: "512 MB", "2048 MB", "4096 MB"

![Image: Memory column](/images/containers/manage-column-memory.png)

**7. Ports**
- Pemetaan port (Host:Container)
- Menampilkan protokol (TCP/UDP)
- Beberapa baris jika ada beberapa port
- "No ports" jika tidak ada pemetaan

![Image: Ports column](/images/containers/manage-column-ports.png)

**Contoh tampilan port**:
```
8080:80 (TCP)
5432:5432 (TCP)

Beberapa port:
  8080:80 (TCP)
  8443:443 (TCP)
```

**8. Owner**
- Siapa yang membuat container
- Menampilkan: "You" (container Anda), "Other User", atau "System"

![Image: Owner column](/images/containers/manage-column-owner.png)

**9. Actions**
- Tombol aksi untuk operasi container
- Lihat bagian "Aksi Container" di bawah

![Image: Actions column](/images/containers/manage-column-actions.png)

---

## Aksi Container

Aksi yang tersedia di tabel dan halaman detail:

### Lihat Log

Klik ikon **Logs** (📄) untuk melihat log container:

![Image: Logs button in table](/images/containers/manage-action-logs.png)

Membuka halaman detail container pada tab Logs.

**Yang akan Anda lihat**:
- Streaming log real-time
- Output stdout dan stderr
- Timestamp untuk setiap entri log

Lihat [Lihat Log](logs/) untuk panduan lengkap.

---

### Lihat Shell

Klik ikon **Shell** (⌨️) untuk membuka shell container:

![Image: Shell button in table](/images/containers/manage-action-shell.png)

Membuka halaman detail container pada tab Shell.

**Yang akan Anda lihat**:
- Terminal interaktif di dalam container
- Jalankan perintah secara langsung
- Akses filesystem container

---

### Start Container

Tersedia saat: **Container Stopped**

![Image: Start button](/images/containers/manage-action-start.png)

**Cara start**:
1. Temukan container yang dihentikan di tabel
2. Klik tombol **ikon Play** (▶️)
3. Status container berubah menjadi "Booting" lalu "Running"
4. Tunggu 5-10 detik untuk startup

**Yang terjadi**:
- Firecracker VM dilanjutkan atau di-restart
- Docker daemon dimulai
- Container dimulai dengan konfigurasi yang tersimpan

---

### Stop Container

Tersedia saat: **Container Running**

![Image: Stop button](/images/containers/manage-action-stop.png)

**Cara stop**:
1. Temukan container yang berjalan di tabel
2. Klik tombol **ikon Stop** (⏹️)
3. Container mati dengan baik (graceful shutdown)
4. Status berubah menjadi "Stopped"

**Yang terjadi**:
- Container menerima sinyal SIGTERM
- Grace period 10 detik untuk pembersihan
- Kemudian SIGKILL jika belum berhenti
- Firecracker VM dihentikan

**Keamanan data**:
- ✅ Data di volume tersimpan
- ✅ Konfigurasi tersimpan
- ⚠️ Data yang tidak di volume mungkin hilang

---

### Restart Container

Tersedia saat: **Container Running**

![Image: Restart button](/images/containers/manage-action-restart.png)

**Cara restart**:
1. Temukan container yang berjalan di tabel
2. Klik tombol **ikon Restart** (🔄)
3. Container dihentikan lalu dimulai kembali
4. Total waktu: 10-20 detik

**Yang terjadi**:
1. Container dihentikan (graceful shutdown)
2. Container dimulai ulang
3. Konfigurasi yang sama digunakan
4. Counter uptime baru

**Kasus penggunaan**:
- Terapkan perubahan konfigurasi
- Bersihkan state memory
- Pulihkan dari soft error
- Reload kode aplikasi (jika di-mount ke volume)

---


### Hapus Container

Tersedia saat: **Container Stopped atau dalam status Error**

![Image: Delete button](/images/containers/manage-action-delete.png)

**Cara hapus**:

**Dari tabel**:
1. Temukan container yang dihentikan/error
2. Klik tombol **ikon Tempat Sampah** (🗑️)
3. Konfirmasi penghapusan dalam dialog

**Dari halaman detail**:
1. Hentikan container terlebih dahulu (jika berjalan)
2. Klik tombol **"Delete"** di header
3. Konfirmasi penghapusan

**Dialog konfirmasi**:

![Image: Delete confirmation dialog](/images/containers/manage-delete-confirm.png)

**Peringatan**: Tindakan ini tidak dapat dibatalkan!

**Yang dihapus**:
- ❌ Konfigurasi container
- ❌ Instans container
- ❌ Firecracker microVM
- ❌ Data container sementara (yang tidak di volume)
- ✅ **Volume tetap tersimpan** (dapat digunakan kembali)

**Penting**: Jika Anda ingin menghapus volume juga, hapus secara manual dari halaman Volumes.

---

## Halaman Detail Container

Klik nama container untuk membuka halaman detail:

![Image: Container detail page header](/images/containers/manage-detail-header.png)

### Header Halaman

Header menampilkan:

![Image: Detail page header components](/images/containers/manage-detail-header-parts.png)

1. **Tombol Kembali** - Kembali ke daftar container
2. **Nama container** - Besar, tebal
3. **Badge status** - Status saat ini dengan warna
4. **Nama image** - Di bawah nama container
5. **Container ID** - Identifikasi unik
6. **Pesan error** - Jika dalam status error (teks merah)
7. **Tombol aksi** - Sisi kanan

---

### Tombol Aksi (Halaman Detail)

![Image: Detail page action buttons](/images/containers/manage-detail-actions.png)

**Tombol yang tersedia** (bergantung pada status):
- **Refresh** - Muat ulang data container
- **Edit** - Ubah konfigurasi (saat dihentikan)
- **Start** - Jalankan container yang dihentikan
- **Pause** - Jeda container yang berjalan
- **Resume** - Lanjutkan container yang dijeda
- **Stop** - Hentikan container yang berjalan
- **Restart** - Restart container yang berjalan
- **View Container VM** - Buka microVM yang mendasarinya
- **Delete** - Hapus container

---

### Tab Container

Halaman detail memiliki 5 tab:

![Image: Container detail tabs](/images/containers/manage-detail-tabs.png)

#### 1. Tab Overview

Menampilkan ringkasan dan informasi container:

![Image: Overview tab](/images/containers/manage-tab-overview.png)

**Informasi yang ditampilkan**:
- Container ID
- Status saat ini
- Nama dan tag image
- Uptime
- Tanggal dibuat
- Batas CPU dan memory
- Pemetaan port
- Variabel lingkungan
- Volume mount
- Tautan VM container

**Kasus penggunaan**:
- Cek status cepat
- Lihat konfigurasi sekilas
- Akses VM container

---

#### 2. Tab Logs

Log container secara real-time:

![Image: Logs tab](/images/containers/manage-tab-logs.png)

**Fitur**:
- Start/stop streaming log
- Auto-scroll ke log terbaru
- Unduh log sebagai file teks
- Pisah stdout/stderr
- Timestamp untuk setiap entri

Lihat [Lihat Log](logs/) untuk panduan lengkap.

---

#### 3. Tab Stats

Monitor penggunaan resource:

![Image: Stats tab](/images/containers/manage-tab-stats.png)

**Metrik yang ditampilkan**:
- Penggunaan CPU (%)
- Penggunaan Memory (MB dan %)
- I/O Jaringan
- I/O Disk
- Uptime

**Fitur**:
- Pembaruan real-time (setiap 5 detik)
- Chart dan grafik
- Data historis

Lihat [Monitor Statistik](stats/) untuk panduan lengkap.

---

#### 4. Tab Config

Detail konfigurasi lengkap:

![Image: Config tab](/images/containers/manage-tab-config.png)

**Menampilkan**:
- Semua pemetaan port
- Semua variabel lingkungan
- Semua volume mount
- Batas resource
- Informasi image
- Pengaturan container

**Kasus penggunaan**:
- Verifikasi konfigurasi
- Dokumentasikan pengaturan
- Referensi untuk deployment baru

---

#### 5. Tab Events

Event siklus hidup container:

![Image: Events tab](/images/containers/manage-tab-events.png)

**Jenis event**:
- Container dibuat
- Container dijalankan
- Container dihentikan
- Container dihapus
- Perubahan status
- Error dan peringatan

**Detail event**:
- Timestamp
- Jenis event
- Pesan event
- Aktor (siapa yang memicu)

---

## Edit Container

Ubah konfigurasi container saat dihentikan.

### Kapan Tersedia

Tombol Edit hanya aktif saat:
- ✅ Container **Stopped**
- ✅ Container dalam status **Error**
- ❌ Dinonaktifkan saat Running, Paused, atau Creating

![Image: Edit button enabled when stopped](/images/containers/manage-edit-enabled.png)

---

### Buka Dialog Edit

Klik tombol **"Edit"** untuk membuka dialog edit:

![Image: Edit container dialog](/images/containers/manage-edit-dialog.png)

---

### Kolom yang Dapat Diedit

**Yang dapat diubah**:

**1. Batas Resource**:
- Batas CPU (0.1 hingga 16 vCPU)
- Batas Memory (64 MB hingga 32 GB)

![Image: Edit resources](/images/containers/manage-edit-resources.png)

<!-- **2. Pemetaan Port**:
- Tambah pemetaan port baru
- Hapus pemetaan yang ada
- Ubah port host (bukan port container)

![Image: Edit ports](/images/containers/manage-edit-ports.png) -->

**2. Variabel Lingkungan**:
- Tambah variabel baru
- Ubah variabel yang ada
- Hapus variabel

![Image: Edit environment variables](/images/containers/manage-edit-env.png)

**Tidak dapat diedit setelah dibuat**:
- ❌ Nama container
- ❌ Nama image
- ❌ Volume mount (buat container baru sebagai gantinya)

---

### Simpan Perubahan

Setelah mengedit:

![Image: Save edit button](/images/containers/manage-edit-save.png)

1. Tinjau perubahan Anda
2. Klik **"Save Changes"**
3. Konfigurasi container diperbarui
4. Start container untuk menerapkan perubahan

**Penting**: Perubahan baru berlaku setelah container dijalankan.

---

## Lihat VM Container

Setiap container berjalan dalam Firecracker microVM-nya sendiri. Anda dapat melihat VM yang mendasarinya:

![Image: View Container VM button](/images/containers/manage-view-vm-button.png)

### Akses VM Container

Klik tombol **"View Container VM"**:
- Membuka halaman detail VM dalam tampilan baru
- Menampilkan VM yang menjalankan container ini
- Lihat log VM, metrik, konfigurasi

![Image: Container VM detail page](/images/containers/manage-container-vm-page.png)

### Detail VM Container

VM container menampilkan:
- **VM ID** - Identifikasi unik (dimulai dengan "vm-")
- **Status** - Status VM (sesuai status container)
- **Resource** - CPU dan memory yang dialokasikan
- **Jaringan** - Konfigurasi jaringan VM
- **Log** - Log boot dan sistem VM

**Kasus penggunaan**:
- Debug masalah tingkat VM
- Cek log boot VM
- Verifikasi konfigurasi jaringan
- Monitor penggunaan resource VM

---

## Refresh Daftar Container

Perbarui daftar container agar tetap terkini:

![Image: Refresh button](/images/containers/manage-refresh-button.png)

### Cara Refresh

Klik tombol **"Refresh"** di header halaman.

**Yang terjadi**:
- Tombol menampilkan "Refreshing..." dengan spinner
- Mengambil data container terbaru
- Tabel diperbarui dengan informasi baru

**Kapan perlu di-refresh**:
- Setelah men-deploy container baru
- Cek perubahan status
- Verifikasi aksi selesai
- Monitor beberapa container

**Auto-refresh**: Belum tersedia. Refresh manual diperlukan.

---

## Tugas Manajemen Umum

### Tugas: Hentikan Semua Container yang Berjalan

**Langkah**:
1. Filter berdasarkan status: **"Running"**
2. Untuk setiap container yang berjalan, klik tombol **Stop**
3. Tunggu masing-masing mencapai status **"Stopped"**
4. Refresh daftar untuk verifikasi

**Kasus penggunaan**: Pemeliharaan, pembersihan resource, shutdown

---

### Tugas: Restart Container yang Gagal

**Langkah**:
1. Filter berdasarkan status: **"Error"**
2. Untuk setiap container error:
   - Klik nama container untuk melihat detail
   - Cek log untuk alasan error
   - Perbaiki masalah (jika masalah konfigurasi)
   - Hapus dan buat ulang container

**Kasus penggunaan**: Pulihkan dari error, perbaiki konfigurasi

---

### Tugas: Temukan Container dengan Resource Tinggi

**Langkah**:
1. Filter berdasarkan status: **"Running"**
2. Perhatikan kolom CPU dan Memory
3. Klik container dengan resource tinggi
4. Buka tab **Stats**
5. Analisis penggunaan resource aktual
6. Sesuaikan batas jika diperlukan (stop, edit, start)

**Kasus penggunaan**: Optimalkan alokasi resource, temukan bottleneck

---

### Tugas: Bersihkan Container Lama

**Langkah**:
1. Filter berdasarkan status: **"Stopped"**
2. Cek kolom Uptime (menampilkan kapan terakhir berjalan)
3. Hapus container yang tidak lagi dibutuhkan
4. Konfirmasi penghapusan untuk masing-masing

**Kasus penggunaan**: Bebaskan resource, bersihkan deployment lama

---

### Tugas: Monitor Container Produksi

**Alur kerja**:
1. Cari "prod" untuk menemukan container produksi
2. Verifikasi semua dalam status **"Running"**
3. Klik masing-masing untuk cek:
   - Tab Logs: Tidak ada error
   - Tab Stats: Penggunaan resource normal
   - Uptime: Stabil, tidak ada restart

**Kasus penggunaan**: Health check harian, monitoring

---

## Izin dan Kontrol Akses

### Akses Berbasis Pemilik

Container dimiliki oleh pengguna yang membuatnya:

![Image: Owner column showing ownership](/images/containers/manage-owner-column.png)

**Indikator pemilik**:
- **"You"** - Container Anda (kontrol penuh)
- **"Other User"** - Container pengguna lain (akses terbatas)
- **"System"** - Container sistem (akses terbatas)

---

### Tingkat Izin

**Container Anda** (Owner: "You"):
- ✅ Lihat detail
- ✅ Lihat log, statistik, konfigurasi
- ✅ Start, stop, restart, pause, resume
- ✅ Edit konfigurasi
- ✅ Delete

**Container pengguna lain**:
- ✅ Lihat di daftar (jika peran admin/viewer)
- ❌ Tidak dapat mengubah
- ❌ Tidak dapat menghapus
- Menampilkan "Not permitted" di kolom Actions

![Image: Not permitted action](/images/containers/manage-not-permitted.png)

---

## Pemecahan Masalah

### Masalah: Container Tidak Mau Start

**Gejala**:
- Klik tombol Start
- Container masuk ke "Booting" lalu kembali ke "Stopped"
- Atau masuk ke status "Error"

![Image: Container failed to start](/images/containers/troubleshoot-wont-start.png)

**Solusi**:
1. **Cek log**:
   - Buka tab Logs
   - Cari pesan error
   - Umum: Konflik port, variabel lingkungan yang hilang

2. **Verifikasi konfigurasi**:
   - Buka tab Config
   - Periksa variabel lingkungan
   - Verifikasi pemetaan port

3. **Cek resource host**:
   - Buka Dashboard → Hosts
   - Pastikan host memiliki cukup CPU/memory
   - Periksa host online

4. **Cek VM container**:
   - Klik "View Container VM"
   - Cek status VM
   - Lihat log VM

---

### Masalah: Container Terus Restart

**Gejala**:
- Container mencapai "Running"
- Lalu kembali ke "Stopped"
- Berulang terus

![Image: Container restart loop](/images/containers/troubleshoot-restart-loop.png)

**Solusi**:
1. **Cek log aplikasi**:
   - Lihat tab Logs dengan cepat setelah restart
   - Cari pesan crash
   - Umum: Error aplikasi, dependensi yang hilang

2. **Verifikasi variabel lingkungan**:
   - Periksa variabel yang diperlukan sudah diset
   - Contoh: `POSTGRES_PASSWORD`, `API_KEY`

3. **Cek kebutuhan image**:
   - Baca dokumentasi Docker Hub
   - Pastikan semua persyaratan terpenuhi
   - Cek kebutuhan memory minimal

4. **Tes secara lokal**:
   ```bash
   docker run -it --rm \
     -e VAR=value \
     -p 8080:80 \
     nginx:alpine
   ```

---

### Masalah: Tidak Dapat Menghapus Container

**Gejala**:
- Tombol Delete dinonaktifkan
- Status container "Running" atau "Paused"

![Image: Delete button disabled](/images/containers/troubleshoot-cant-delete-running.png)

**Solusi**:
1. **Hentikan container terlebih dahulu**:
   - Klik tombol Stop (jika running)
   - Atau Resume lalu Stop (jika paused)
2. **Tunggu status "Stopped"**
3. **Kemudian klik Delete**

**Alasan**: Container yang berjalan tidak dapat dihapus untuk mencegah kehilangan data.

---

### Masalah: Tombol Edit Dinonaktifkan

**Gejala**:
- Tombol Edit berwarna abu-abu
- Container sedang berjalan

![Image: Edit button disabled](/images/containers/troubleshoot-cant-edit-running.png)

**Solusi**:
1. **Hentikan container terlebih dahulu**:
   - Klik tombol Stop
   - Tunggu status "Stopped"
2. **Kemudian klik Edit**

**Alasan**: Konfigurasi tidak dapat diedit saat container berjalan.

---

### Masalah: Aksi Menampilkan "Not Permitted"

**Gejala**:
- Dapat melihat container di daftar
- Kolom Actions menampilkan "Not permitted"
- Tidak dapat start, stop, atau delete

![Image: Not permitted message](/images/containers/troubleshoot-not-permitted.png)

**Alasan**:
- Container dibuat oleh pengguna lain
- Anda tidak memiliki izin untuk mengubah

**Solusi**:
- Minta pemilik container untuk memberikan akses
- Atau minta admin untuk transfer kepemilikan
- Atau buat container Anda sendiri

---

### Masalah: Container Hilang dari Daftar

**Gejala**:
- Container ada sebelumnya
- Sekarang tidak ada di tabel
- Tidak dihapus dengan sengaja

![Image: Empty container list](/images/containers/troubleshoot-missing.png)

**Solusi**:
1. **Cek filter**:
   - Pastikan filter status "All Status"
   - Kosongkan kotak pencarian
   - Klik "Refresh"

2. **Cek filter kepemilikan**:
   - Pengguna non-admin hanya melihat container mereka sendiri
   - Peran admin/viewer melihat semua container

3. **Verifikasi tidak dihapus**:
   - Tanya anggota tim lain
   - Cek tab Events pada container lain

---

## Praktik Terbaik

### Manajemen Siklus Hidup

✅ **Hentikan container saat tidak digunakan**:
- Bebaskan resource host
- Kurangi biaya
- Mudah di-restart saat dibutuhkan

✅ **Monitor kesehatan container**:
- Cek log secara berkala
- Monitor penggunaan resource di Stats
- Perhatikan error atau peringatan

✅ **Restart container secara berkala**:
- Bersihkan memory leak
- Terapkan perubahan konfigurasi
- Segarkan koneksi

❌ **Jangan hapus container dengan data**:
- Stop alih-alih delete jika data penting
- Gunakan volume untuk data persisten
- Volume bertahan saat container dihapus

---

### Optimasi Resource

✅ **Sesuaikan ukuran container**:
- Monitor penggunaan aktual di tab Stats
- Sesuaikan resource berdasarkan data
- Stop, edit, start untuk menerapkan batas baru

✅ **Gunakan pause untuk idle sementara**:
- Pause alih-alih stop untuk periode singkat
- Resume lebih cepat dari restart
- Mempertahankan state

❌ **Jangan over-alokasi resource**:
- Cek penggunaan aktual sebelum meningkatkan
- Membuang resource host
- Membatasi jumlah container yang dapat di-deploy

---

### Keamanan

✅ **Pembaruan berkala**:
- Buat ulang container secara berkala dengan image terbaru
- Cek pembaruan keamanan
- Tes sebelum deploy ke produksi

✅ **Tinjau izin**:
- Cek siapa yang memiliki akses ke container
- Hapus akses yang tidak diperlukan
- Gunakan model kepemilikan dengan benar

✅ **Monitor log**:
- Perhatikan aktivitas yang tidak biasa
- Cek akses yang tidak sah
- Siapkan alert (fitur mendatang)

---

### Pemecahan Masalah

✅ **Cek log terlebih dahulu**:
- Sebagian besar masalah muncul di log
- Mulai dari tab Logs
- Cari pesan error dan stack trace

✅ **Gunakan Stats untuk masalah resource**:
- Jika container lambat, cek Stats
- Cari penggunaan CPU atau memory yang tinggi
- Sesuaikan batas jika diperlukan

✅ **Tes perubahan konfigurasi**:
- Edit container saat dihentikan
- Tes terlebih dahulu di luar produksi
- Verifikasi sebelum deploy

---

## Referensi Cepat

### Aksi Berdasarkan Status Container

| Status Saat Ini | Aksi Tersedia |
|----------------|---------------|
| Creating | Tunggu |
| Booting | Tunggu |
| Initializing | Tunggu |
| Running | Stop, Restart, Pause, Logs, Shell, Stats |
| Stopped | Start, Edit, Delete |
| Paused | Resume, Stop |
| Error | Logs, Edit, Delete |

### Pintasan Aksi Umum

| Ingin... | Langkah |
|----------|---------|
| Start container | Klik tombol ▶️ Play |
| Stop container | Klik tombol ⏹️ Stop |
| Restart container | Klik tombol 🔄 Restart |
| Lihat log | Klik tombol 📄 Logs |
| Buka shell | Klik tombol ⌨️ Shell |
| Hapus container | Stop → Klik 🗑️ Delete → Konfirmasi |
| Edit konfigurasi | Stop → Edit → Simpan → Start |

### Pintasan Keyboard

| Pintasan | Aksi |
|----------|------|
| Klik nama container | Buka halaman detail |
| Tombol Back | Kembali ke daftar container |
| Tab di tabel | Navigasi ke sel berikutnya |

---

## Langkah Selanjutnya

- **[Lihat Log](logs/)** - Streaming log dan debugging real-time
- **[Monitor Statistik](stats/)** - Metrik penggunaan resource dan performa
- **[Deploy Container](deploy-container/)** - Buat container baru
- **[Ikhtisar Container](./)** - Pelajari lebih lanjut tentang container
