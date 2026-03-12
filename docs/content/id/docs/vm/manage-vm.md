+++
title = "Kelola VM"
description = "Start, stop, pause, resume, dan hapus mesin virtual"
weight = 43
date = 2025-12-16
+++

Pelajari cara mengelola siklus hidup mesin virtual Anda melalui antarmuka web.

---

## Mengakses Manajemen VM

Navigasi ke halaman **Virtual Machines** dari sidebar untuk melihat semua VM Anda:

![Image: VMs List Page](/images/vm/manage-vms-page.png)

Halaman VMs menyediakan:
- **Search bar** - Temukan VM berdasarkan nama atau ID
- **State filter** - Filter berdasarkan All States, Running, Stopped, atau Paused
- **VM table** - Daftar semua VM beserta detail dan tindakan
- **Quick create** - Buat VM dari template (lihat di bawah)

---

## Status Siklus Hidup VM

VM dapat berada dalam salah satu dari beberapa status:

![Image: VM state badges](/images/vm/vm-state-badges.png)

| Status | Deskripsi | Tindakan yang Tersedia |
|--------|-----------|------------------------|
| **Stopped** | VM telah dibuat tetapi tidak berjalan | Start, Delete |
| **Running** | VM aktif dan menggunakan sumber daya | Stop, Pause |
| **Paused** | VM dibekukan di memori | Resume, Delete |

**Transisi status**:
- Stopped → Start → Running
- Running → Stop → Stopped
- Running → Pause → Paused
- Paused → Resume → Running
- Paused → Delete → (Dihapus)
- Stopped → Delete → (Dihapus)

**Catatan**: Anda **tidak dapat menghapus VM yang sedang berjalan** — Anda harus menghentikan atau menjeda terlebih dahulu.

---

## Menjalankan VM

### Start dari Halaman Daftar VM

Ketika VM berada dalam status **Stopped**, Anda dapat menjalankannya langsung dari tabel VM:

![Image: Start button on VM list](/images/vm/vm-action-start.png)

**Langkah**:
1. Pergi ke halaman **Virtual Machines**
2. Temukan VM yang terhenti (badge "Stopped" berwarna merah)
3. Di kolom **Actions**, klik tombol **Start** (ikon ▶ Play)


Status VM akan berubah dari "Stopped" menjadi "Running" dalam 1-2 detik.

### Yang Terjadi

Di balik layar, sistem akan:
1. ✓ Mengalokasikan sumber daya di host
2. ✓ Mem-boot kernel
3. ✓ Menginisialisasi sistem operasi
4. ✓ Mengonfigurasi antarmuka jaringan (menetapkan IP melalui DHCP)
5. ✓ Menjalankan semua layanan

**Waktu**: Biasanya selesai dalam **1-2 detik**

### Verifikasi VM Berjalan

Setelah dijalankan, periksa status VM:

![Image: VM showing Running status](/images/vm/vm-running-badge.png)

Anda seharusnya melihat:
- Badge status: **Running** (hijau)
- Kolom **Guest IP** menampilkan alamat IP yang ditetapkan
- Kolom **CPU** menampilkan persentase penggunaan
- Kolom **Memory** menampilkan persentase penggunaan

**Selanjutnya**: Klik nama VM untuk mengakses halaman detail yang berisi console, metrik, dan lainnya.

---

## Menghentikan VM

### Hentikan VM yang Sedang Berjalan

**⚠️ Peringatan**: Menghentikan VM sama seperti mencabut kabel daya — pekerjaan yang belum disimpan akan hilang!

Dari halaman daftar VM:

![Image: Stop button on running VM](/images/vm/vm-action-stop.png)

**Langkah**:
1. Temukan VM yang sedang berjalan (badge "Running" berwarna hijau)
2. Di kolom **Actions**, Anda akan melihat dua tombol:
   - Tombol **Pause** (ikon ❚❚) - membekukan VM
   - Tombol **Stop** (ikon ◼ Square) - menghentikan VM
3. Klik tombol **Stop**

VM akan segera berhenti dan berubah ke status "Stopped".

### Penghentian Aman (Direkomendasikan)

**Praktik terbaik**: Matikan OS secara graceful sebelum menghentikan:

1. Klik nama VM untuk membuka halaman detail VM
2. Pergi ke tab **Terminal**
3. Login ke console VM
4. Jalankan perintah shutdown:

```bash
# For Alpine/Debian/Ubuntu
shutdown now

# Or
poweroff
```

5. Tunggu 5-10 detik untuk shutdown graceful
6. Kembali ke daftar VM dan klik **Stop** jika diperlukan

Ini memastikan:
- Semua proses berhenti dengan bersih
- Filesystem dilepas dengan benar
- Data disimpan ke disk
- Tidak ada risiko korupsi

### Yang Terjadi

Ketika Anda mengklik Stop, sistem akan:
1. ✓ Memaksa menghentikan semua proses VM
2. ✓ Melepas antarmuka jaringan
3. ✓ Membebaskan sumber daya CPU
4. ✓ Menyimpan snapshot memori untuk restart cepat
5. ✓ Mengubah status menjadi **Stopped**

**Waktu**: Biasanya instan (< 1 detik)

![Image: VM in stopped state](/images/vm/vm-stopped-badge.png)

### Kapan Menghentikan VM

✅ **Hentikan ketika**:
- VM tidak diperlukan dalam waktu lama
- Melakukan pemeliharaan atau pembaruan
- Menghemat sumber daya CPU
- Memecahkan masalah
- Mempersiapkan penghapusan

⚠️ **Hindari menghentikan ketika**:
- Layanan produksi sedang berjalan
- Tugas yang berjalan lama sedang berlangsung (backup, build, dll.)
- Layanan/VM lain bergantung pada VM ini

---

## Menjeda VM

### Jeda VM yang Sedang Berjalan

Menjeda **membekukan** VM dalam statusnya saat ini — berguna untuk:
- Sementara membebaskan sumber daya CPU
- Debugging (jeda untuk memeriksa status)
- Resume cepat nanti tanpa reboot

Dari halaman daftar VM:

![Image: Pause button on running VM](/images/vm/vm-action-pause.png)

**Langkah**:
1. Temukan VM yang sedang berjalan (badge "Running" berwarna hijau)
2. Di kolom **Actions**, klik tombol **Pause** (ikon ❚❚)

VM akan langsung dibekukan dan berubah ke status "Paused".

### Yang Terjadi

Ketika Anda menjeda VM:
1. ✓ Semua proses dibekukan secara instan
2. ✓ Status saat ini disimpan ke memori
3. ✓ Sumber daya CPU dibebaskan (penggunaan CPU 0%)
4. ✓ Memori tetap dialokasikan
5. ✓ Koneksi jaringan ditangguhkan
6. ✓ Status berubah menjadi **Paused**

**Waktu**: Instan (< 100ms)

![Image: VM in paused state](/images/vm/vm-paused-badge.png)

**Penting**: VM yang dijeda masih menggunakan **memori** tetapi **tidak CPU**!

### Kapan Menjeda

✅ **Jeda ketika**:
- Perlu sementara membebaskan CPU untuk VM lain
- Debugging atau pemecahan masalah (periksa status yang dibekukan)
- Istirahat singkat (resume dalam beberapa jam)
- Menguji fungsionalitas pause/resume

⚠️ **Jangan jeda untuk**:
- Jangka waktu panjang (gunakan Stop untuk membebaskan memori)
- VM produksi (dapat menyebabkan masalah timeout)
- Aplikasi yang sensitif terhadap jaringan

---

## Melanjutkan VM

### Resume dari Status Paused

Dari halaman daftar VM:

![Image: Resume button on paused VM](/images/vm/vm-action-resume.png)

**Langkah**:
1. Temukan VM yang dijeda (badge "Paused" berwarna oranye)
2. Di kolom **Actions**, klik tombol **Resume** (ikon ▶ Play)

VM akan segera melanjutkan eksekusi.

### Yang Terjadi

Ketika Anda melanjutkan VM:
1. ✓ Eksekusi proses dipulihkan
2. ✓ VM melanjutkan dari titik jeda yang tepat
3. ✓ Penggunaan CPU dilanjutkan
4. ✓ Koneksi jaringan dibangun kembali
5. ✓ Status berubah menjadi **Running**

**Waktu**: Instan (< 100ms)

![Image: VM resumed to running state](/images/vm/vm-running-badge.png)

**Catatan**: VM melanjutkan **tepat dari tempat terakhir berhenti** — tidak ada reboot, tidak ada kehilangan data, aplikasi terus berjalan!

### Perbandingan Pause vs Stop

| Operasi | Waktu Resume | Status Tersimpan | Memori Digunakan | CPU Digunakan | Kasus Penggunaan |
|---------|--------------|------------------|------------------|---------------|------------------|
| **Pause** | ~100ms | ✅ Ya (tepat) | ✅ Ya | ❌ Tidak | Istirahat singkat, debugging |
| **Stop** | ~2 detik | ❌ Tidak (reboot) | ❌ Tidak | ❌ Tidak | Istirahat lama, bebaskan semua sumber daya |

**Panduan Keputusan**:
- **Butuh CPU sekarang, mungkin resume segera** → Gunakan **Pause**
- **Tidak akan menggunakan VM selama berjam-jam/berhari-hari** → Gunakan **Stop**
- **Ingin membebaskan semua sumber daya** → Gunakan **Stop**
- **Debugging/inspeksi** → Gunakan **Pause**

---

## Menghapus VM

**⚠️ Peringatan**: Penghapusan bersifat **permanen** dan **tidak dapat dibatalkan**!

### Penting: VM yang Sedang Berjalan Tidak Dapat Dihapus

Anda **tidak dapat menghapus VM yang sedang berjalan**. Tombol hapus akan **dinonaktifkan** dengan tooltip:

![Image: Delete button disabled on running VM](/images/vm/vm-delete-disabled-running.png)

**"Cannot delete running VM. Stop the VM first."**

**Anda harus**:
- **Hentikan** VM terlebih dahulu, ATAU
- **Jeda** VM terlebih dahulu

Kemudian tombol hapus akan tersedia.

### Sebelum Menghapus

**⚠️ Direkomendasikan: Buat snapshot terlebih dahulu**:
1. Pergi ke halaman detail VM → tab **Snapshots**
2. Klik **Create Snapshot**
3. Tunggu snapshot selesai
4. Sekarang Anda dapat menghapus dengan aman (snapshot dapat memulihkan VM nanti)

Lihat panduan [Backup & Snapshot](backup-snapshot/).

**Periksa hal-hal ini sebelum menghapus**:
- ✅ Data sudah di-backup atau tidak diperlukan
- ✅ Tidak ada layanan lain yang bergantung pada VM ini
- ✅ Tidak ada koneksi aktif
- ✅ Snapshot telah dibuat (jika Anda mungkin perlu memulihkan)

### Hapus VM

Dari halaman daftar VM:

![Image: Delete button on stopped/paused VM](/images/vm/vm-action-delete.png)

**Langkah**:
1. **Hentikan atau jeda VM** (tombol hapus tidak akan berfungsi pada VM yang sedang berjalan)
2. Di kolom **Actions**, klik tombol **Delete** (ikon 🗑️ Trash)
3. Dialog konfirmasi akan muncul:

![Image: Delete confirmation dialog](/images/vm/vm-delete-confirm.png)

4. Klik **Delete** untuk mengonfirmasi

VM akan dihapus secara permanen.

### Yang Dihapus

Ketika Anda menghapus VM:

- ✅ **Konfigurasi VM** - Semua pengaturan dihapus
- ✅ **Status runtime** - Status proses dibersihkan
- ✅ **Konfigurasi jaringan** - Perangkat TAP dilepas
- ⚠️ **Volume rootfs** - Dihapus jika tidak dibagikan dengan VM lain
- ❌ **Snapshots** - Dipertahankan (masih dapat dipulihkan dari sana)
- ❌ **Images di registry** - Dipertahankan (kernel/rootfs masih tersedia)

**Waktu**: Biasanya instan (< 1 detik)

### Notifikasi Berhasil

Setelah penghapusan, Anda akan melihat pesan berhasil:

![Image: VM deleted success notification](/images/vm/vm-deleted-success.png)

**"VM Deleted - [Nama VM] has been deleted"**

VM akan dihapus dari daftar VM.

### Pemulihan Setelah Penghapusan

Jika Anda membuat snapshot sebelum menghapus:

1. Pergi ke halaman **Snapshots** (sidebar)
2. Temukan snapshot VM Anda
3. Klik **Restore** untuk membuat VM baru dari snapshot
4. VM akan dibuat ulang dengan status yang sama seperti saat snapshot diambil

**Catatan**: Anda tidak dapat memulihkan VM yang dihapus kecuali Anda membuat snapshot terlebih dahulu!

---

## Pembuatan Cepat dari Template

Alih-alih menggunakan wizard lengkap, Anda dapat dengan cepat membuat VM dari template:

![Image: Quick create button](/images/vm/quick-create-button.png)

**Langkah**:
1. Di halaman **Virtual Machines**, klik tombol **Quick create**
2. Dialog akan terbuka menampilkan template yang tersedia:

![Image: Quick create dialog with template selection](/images/vm/quick-create-dialog.png)

3. **Pilih template** dengan mengkliknya (menampilkan tanda centang saat dipilih)
4. **Masukkan nama VM** di kolom input
5. Klik **Create VM**

![Image: Template selected with VM name](/images/vm/quick-create-selected.png)

VM akan dibuat secara instan dengan semua pengaturan dari template!

**Keuntungan**:
- ⚡ Jauh lebih cepat dari wizard lengkap
- ✅ Pengaturan yang telah dikonfigurasi sebelumnya (CPU, memori, images)
- ✅ Konfigurasi yang konsisten antar VM
- ✅ Sempurna untuk membuat beberapa VM serupa

**Kasus penggunaan**:
- Buat lingkungan dev untuk anggota tim
- Spin up VM pengujian dengan cepat
- Deploy konfigurasi yang terstandarisasi
- Prototyping cepat

Lihat [VM Templates](../templates/) untuk membuat dan mengelola template.

---

## Memfilter dan Mencari VM

### Cari berdasarkan Nama atau ID

Gunakan search bar untuk menemukan VM dengan cepat:

![Image: Search bar in VMs page](/images/vm/vm-search-bar.png)

- Ketik nama VM (mis., "web-server")
- Atau ketik ID VM
- Hasil difilter secara instan saat Anda mengetik
- Pencarian tidak peka huruf besar/kecil

### Filter berdasarkan Status

Gunakan dropdown filter status untuk menampilkan hanya status VM tertentu:

![Image: State filter dropdown](/images/vm/vm-state-filter.png)

**Opsi**:
- **All States** - Tampilkan semua VM (default)
- **Running** - Hanya VM yang berjalan
- **Stopped** - Hanya VM yang berhenti
- **Paused** - Hanya VM yang dijeda

**Tips**: Kombinasikan pencarian dan filter untuk hasil yang tepat (mis., cari "prod" + filter "Running")

---

## Informasi Tabel VM

Tabel VM menampilkan informasi detail untuk setiap VM:

### Penjelasan Kolom

1. **Name** - Nama VM (klik untuk membuka halaman detail)
2. **State** - Status saat ini dengan badge berwarna
3. **CPU** - Jumlah vCPU dan persentase penggunaan saat ini
4. **Memory** - MiB yang dialokasikan dan persentase penggunaan saat ini
5. **Guest IP** - Alamat IP yang ditetapkan ke VM (melalui DHCP)
6. **Host** - Host/agent mana yang menjalankan VM ini
7. **Owner** - Siapa yang membuat VM:
   - **"You"** (hijau) - VM yang Anda buat
   - **"Other User"** - VM pengguna lain
   - **"System"** - VM yang dibuat sistem
8. **Created** - Waktu relatif (mis., "2 hours ago")
9. **Actions** - Tombol tindakan (Start, Stop, Pause, Resume, Delete)

### Paginasi

Jika Anda memiliki lebih dari 10 VM, gunakan paginasi di bagian bawah:

![Image: Pagination controls](/images/vm/vm-pagination.png)

- **10 VM per halaman**
- Klik nomor halaman untuk navigasi
- Gunakan panah Previous/Next

---

## Pemecahan Masalah

### Masalah: Tidak Bisa Start VM

**Gejala**:
- Tombol Start tidak merespons
- VM tersangkut di status transisi
- Notifikasi error muncul

**Solusi**:

1. **Periksa sumber daya host**:
   - Pergi ke halaman **Hosts** (sidebar)
   - Verifikasi host memiliki CPU dan memori yang tersedia
   - Jika host kelebihan beban, hentikan VM lain terlebih dahulu

2. **Verifikasi images ada**:
   - Pergi ke **Registry** → **Images**
   - Periksa image kernel dan rootfs ada
   - Unggah ulang jika hilang

3. **Periksa status agent**:
   - Pergi ke halaman **Hosts**
   - Verifikasi agent **Online** (hijau)
   - Jika offline, hubungi administrator

4. **Coba hapus dan buat ulang**:
   - Hapus VM yang bermasalah
   - Buat yang baru dengan pengaturan yang sama

---

### Masalah: Tombol Delete Dinonaktifkan

**Gejala**:
- Tombol Delete berwarna abu-abu
- Tooltip berkata "Cannot delete running VM"

![Image: Disabled delete button with tooltip](/images/vm/vm-delete-disabled-running.png)

**Solusi**:

Ini adalah **perilaku yang diharapkan** — Anda tidak dapat menghapus VM yang sedang berjalan.

1. Klik tombol **Stop** terlebih dahulu
2. Tunggu status berubah menjadi "Stopped"
3. Sekarang tombol **Delete** akan aktif
4. Klik Delete untuk menghapus VM

**Mengapa ada pembatasan ini?**
- Mencegah penghapusan tidak sengaja pada layanan aktif
- Memastikan shutdown graceful
- Melindungi integritas data

---

### Masalah: VM Tidak Mau Berhenti

**Gejala**:
- Mengklik Stop tetapi VM masih menampilkan "Running"
- Tidak ada perubahan status setelah 30 detik

**Solusi**:

1. **Refresh halaman** (Ctrl+R atau F5)
   - Terkadang UI perlu memperbarui status

2. **Tunggu dan coba lagi**:
   - Tunggu 60 detik
   - Klik Stop lagi

3. **Graceful shutdown terlebih dahulu**:
   - Klik nama VM → tab **Terminal**
   - Login ke console
   - Jalankan: `shutdown now`
   - Tunggu 10 detik, lalu klik Stop

4. **Periksa halaman detail VM**:
   - Buka halaman detail VM
   - Periksa apakah ada error yang ditampilkan
   - Coba hentikan dari sana

---

### Masalah: VM yang Dijeda Tidak Bisa Resume

**Gejala**:
- Tombol Resume tidak berfungsi
- Pesan error muncul

**Solusi**:

1. **Refresh browser**:
   - Tekan F5 atau Ctrl+R
   - Coba Resume lagi

2. **Stop dan start sebagai gantinya**:
   - Klik tombol **Stop** (ya, Anda bisa menghentikan VM yang dijeda)
   - Tunggu status "Stopped"
   - Klik **Start**

3. **Periksa browser console**:
   - Tekan F12 untuk membuka DevTools
   - Pergi ke tab Console
   - Cari pesan error
   - Bagikan dengan administrator jika ditemukan error

4. **Upaya terakhir - Hapus dan pulihkan**:
   - Jika Anda memiliki snapshot, hapus VM
   - Pulihkan dari snapshot

---

## Praktik Terbaik

### Menjalankan VM

✅ **Lakukan**:
- Jalankan VM hanya saat benar-benar diperlukan (hemat sumber daya)
- Gunakan **Quick create** dari template untuk konsistensi
- Verifikasi host memiliki sumber daya yang cukup sebelum menjalankan beberapa VM
- Periksa agent **Online** di halaman Hosts terlebih dahulu
- Gunakan pencarian/filter untuk menemukan VM yang Anda butuhkan dengan cepat

⚠️ **Hindari**:
- Menjalankan semua VM sekaligus (dapat membebani host)
- Menjalankan VM di host yang offline/gagal

---

### Menghentikan VM

✅ **Lakukan**:
- **Graceful shutdown terlebih dahulu** (SSH masuk dan jalankan `shutdown now`)
- Tunggu 5-10 detik setelah graceful shutdown sebelum mengklik Stop
- Hentikan VM saat tidak digunakan untuk membebaskan sumber daya
- Buat snapshot sebelum menghentikan VM penting
- Hentikan VM dev/test setelah jam kerja

⚠️ **Hindari**:
- Force stop tanpa graceful shutdown (risiko korupsi data)
- Menghentikan VM produksi selama jam kerja
- Menghentikan VM dengan koneksi aktif
- Menghentikan VM yang menjalankan tugas panjang (backup, build)

---

### Menjeda VM

✅ **Lakukan**:
- Gunakan pause untuk membebaskan sumber daya CPU secara **sementara** (jam, bukan hari)
- Jeda untuk debugging atau pemecahan masalah
- Resume dalam jangka waktu yang wajar
- Gunakan saat Anda membutuhkan CPU tetapi tidak memori

⚠️ **Hindari**:
- Menjeda untuk jangka waktu panjang (gunakan Stop untuk membebaskan memori)
- Menjeda VM produksi (dapat menyebabkan masalah timeout/koneksi)
- Menjeda aplikasi yang sensitif terhadap jaringan
- Lupa untuk resume (memori terbuang)

---

### Menghapus VM

✅ **Lakukan**:
- **SELALU buat snapshot sebelum menghapus** (dapat dipulihkan nanti)
- Hentikan atau jeda VM terlebih dahulu (tidak bisa menghapus VM yang berjalan)
- Verifikasi data sudah di-backup di tempat lain
- Periksa tidak ada layanan/VM lain yang bergantung padanya
- Dokumentasikan alasan penghapusan (di catatan tim)
- Periksa ulang Anda menghapus VM yang benar

⚠️ **Hindari**:
- Menghapus tanpa snapshot (permanen!)
- Menghapus VM layanan yang dibagikan
- Menghapus VM produksi tanpa persetujuan tim
- Mengklik Delete pada VM yang salah (periksa nama dengan teliti!)

---

### Manajemen Sumber Daya

✅ **Praktik terbaik**:
- **Pantau secara teratur**: Periksa halaman Hosts untuk penggunaan sumber daya
- **Hentikan VM yang tidak digunakan**: Jangan biarkan VM test berjalan semalaman
- **Gunakan filter**: Filter berdasarkan "Running" untuk melihat apa yang menggunakan sumber daya
- **Bersihkan secara teratur**: Hapus VM test/temp yang lama
- **Gunakan template**: Quick create untuk alokasi sumber daya yang terstandarisasi
- **Right-size VM**: Jangan over-allocate CPU/memori

**Daftar periksa pembersihan mingguan**:
1. Filter VM berdasarkan "Running"
2. Hentikan VM dev/test yang tidak digunakan
3. Hapus VM sementara yang lama (setelah membuat snapshot)
4. Periksa halaman Hosts untuk tren penggunaan sumber daya

---

### Pencarian dan Organisasi

✅ **Tips**:
- **Konvensi penamaan**: Gunakan `<env>-<purpose>-<number>` (mis., `dev-web-01`, `prod-api-02`)
- **Gunakan pencarian**: Temukan VM dengan cepat dengan mengetik nama
- **Filter berdasarkan status**: Lihat hanya VM Running/Stopped/Paused
- **Kolom Owner**: Mudah melihat VM mana yang milik Anda
- **Paginasi**: Gunakan nomor halaman untuk daftar VM yang besar

**Contoh skema penamaan**:
```
dev-alice-ubuntu      (VM pribadi developer)
test-backend-api      (lingkungan pengujian)
staging-database      (server DB staging)
prod-web-01          (web server produksi #1)
prod-web-02          (web server produksi #2)
```

---

## Referensi Cepat

### Ringkasan Tindakan Status VM

| Status Saat Ini | Tindakan yang Tersedia | Klik untuk... |
|-----------------|------------------------|----------------|
| **Stopped** | ▶ Start, 🗑️ Delete | Start: Jalankan VM<br>Delete: Hapus permanen |
| **Running** | ❚❚ Pause, ◼ Stop | Pause: Bekukan instan<br>Stop: Matikan VM |
| **Paused** | ▶ Resume, 🗑️ Delete | Resume: Lanjutkan eksekusi<br>Delete: Hapus |

### Alur Kerja Umum

**Pekerjaan harian**:
1. Pagi: Start VM dev Anda
2. Bekerja: Gunakan tab Terminal untuk akses
3. Sore: Stop VM dev Anda

**Pengujian**:
1. Quick create VM dari template pengujian
2. Jalankan pengujian
3. Stop VM
4. Hapus VM setelah mengonfirmasi hasil

**Deployment produksi**:
1. Buat snapshot dari VM prod saat ini
2. Stop VM prod
3. Buat VM baru dengan konfigurasi yang diperbarui
4. Uji VM baru
5. Alihkan traffic ke VM baru
6. Simpan snapshot VM lama untuk rollback

---

## Langkah Berikutnya

Setelah mengetahui cara mengelola VM, jelajahi:

- **[Akses VM](access-vm/)** - Terhubung melalui terminal dan SSH
- **[Pemantauan](monitoring/)** - Lihat metrik performa dan log
- **[Backup & Snapshot](backup-snapshot/)** - Lindungi VM Anda dengan snapshots
- **[Buat VM](create-vm/)** - Buat VM menggunakan wizard
