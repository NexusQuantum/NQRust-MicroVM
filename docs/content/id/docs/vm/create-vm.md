+++
title = "Buat VM"
description = "Panduan langkah demi langkah untuk membuat mesin virtual pertama Anda"
weight = 41
date = 2025-12-16
+++

Panduan ini memandu Anda membuat mesin virtual menggunakan antarmuka web.

---

## Prasyarat

Sebelum membuat VM, pastikan:

- ✅ Anda memiliki akses ke dashboard NQRust-MicroVM
- ✅ Setidaknya satu image kernel tersedia di Image Registry
- ✅ Setidaknya satu image rootfs tersedia di Image Registry
- ✅ Sumber daya yang cukup tersedia di host

---

## Langkah 1: Buka Wizard Pembuatan VM

1. Klik **Virtual Machines** di sidebar kiri
2. Klik tombol **Create VM** di pojok kanan atas
![Image: Screenshot highlighting Create VM button](/images/vm/vm-create-button.png)

Wizard pembuatan VM akan terbuka dengan **6 langkah**.

---

## Langkah 2: Informasi Dasar

Masukkan detail dasar tentang VM Anda:

![Image: Screenshot of Basic Info step](/images/vm/vm-step1-basic.png)

### Nama (Wajib)

Pilih nama yang unik dan deskriptif:

- Harus terdiri dari 1-50 karakter
- Contoh: `web-server-01`, `dev-ubuntu`, `test-environment`

**Tips**: Gunakan nama yang bermakna dan mencerminkan tujuan VM.

### Pemilik (Wajib)

Masukkan nama pemilik:

- Default: `developer`
- Maksimal 32 karakter
- Contoh: `developer`, `alice`, `backend-team`

Kolom ini mengidentifikasi siapa yang memiliki atau mengelola VM ini.

### Lingkungan (Wajib)

Pilih jenis lingkungan dari dropdown:

![Image: Environment dropdown selection](/images/vm/vm-environment-dropdown.png)

- **Development** - Untuk pengujian dan pekerjaan pengembangan
- **Staging** - Lingkungan pengujian pra-produksi
- **Production** - Beban kerja produksi langsung

**Default**: Development

### Deskripsi (Opsional)

Tambahkan catatan tentang VM ini (maksimal 200 karakter):

```
Ubuntu 22.04 development environment for backend API testing
```

**Tips**: Sertakan nama proyek, tim, atau catatan konfigurasi khusus.

Klik **Next** untuk melanjutkan.

---

## Langkah 3: Kredensial

Tetapkan kredensial login untuk VM Anda:

![Image: Screenshot of Credentials step](/images/vm/vm-step2-credentials.png)

### Nama Pengguna (Wajib)

Masukkan nama pengguna untuk akses VM:

- **Default**: `root`
- Maksimal 32 karakter
- Pilihan umum: `root`, `admin`, `ubuntu`

**Catatan**: Ini akan menjadi nama pengguna login Anda untuk akses console dan SSH.

### Kata Sandi (Wajib)

Tetapkan kata sandi yang aman:

- Minimal 1 karakter (maksimal 128 karakter)
- **Rekomendasi**: Gunakan kata sandi yang kuat dengan:
  - Campuran huruf besar dan huruf kecil
  - Angka dan karakter khusus
  - Minimal 12 karakter untuk VM produksi

**Penting**: Anda akan menggunakan kredensial ini untuk login melalui web console atau SSH.

**Tips Keamanan**: Untuk lingkungan produksi, nonaktifkan autentikasi kata sandi dan gunakan SSH key setelah pengaturan awal.

Klik **Next** untuk melanjutkan.

---

## Langkah 4: Konfigurasi Mesin

Konfigurasikan sumber daya CPU dan memori untuk VM Anda:

![Image: Screenshot showing CPU and memory sliders](/images/vm/vm-step3-machine.png)

### Jumlah vCPU

Gunakan slider untuk memilih jumlah CPU virtual (1-32):

![Image: vCPU slider control](/images/vm/vm-vcpu-slider.png)

| vCPU | Terbaik Untuk | RAM Tipikal |
|------|---------------|-------------|
| 1 | Pengujian, layanan ringan | 512 MiB - 1 GiB |
| 2 | Pengembangan, aplikasi kecil | 1 - 2 GiB |
| 4 | Web server, beban kerja menengah | 2 - 8 GiB |
| 8+ | Database, pemrosesan berat | 8+ GiB |

**Default**: 2 vCPU (atau dari preferensi Anda)

### Memori (MiB)

Gunakan slider untuk mengalokasikan memori (128-32768 MiB):

![Image: Memory slider control](/images/vm/vm-memory-slider.png)

| Memori | Kasus Penggunaan | Contoh |
|--------|------------------|--------|
| 512 MiB | Alpine Linux, layanan minimal | Log forwarder, metrics agent |
| 1024 MiB (1 GiB) | Ubuntu minimal, aplikasi kecil | Layanan API, database kecil |
| 2048 MiB (2 GiB) | Lingkungan dev standar | Pengembangan full-stack |
| 4096 MiB (4 GiB) | Web server dengan caching | Nginx + Redis + App |
| 8192 MiB (8 GiB) | Database, build server | PostgreSQL, CI runner |

**Default**: 2048 MiB (atau dari preferensi Anda)

**Penting**: Memori harus merupakan kelipatan dari 128 MiB.

### Opsi Lanjutan

![Image: Advanced options checkboxes](/images/vm/vm-advanced-options.png)

#### Aktifkan SMT (Simultaneous Multithreading)

- ☐ **Enable SMT**
- Default: Dinonaktifkan
- Saat diaktifkan: Mengizinkan banyak thread per inti CPU
- **Gunakan saat**: Beban kerja komputasi berperforma tinggi

#### Lacak Dirty Pages

- ☐ **Track dirty pages**
- Default: Dinonaktifkan
- Saat diaktifkan: Melacak halaman memori yang dimodifikasi oleh VM
- **Gunakan saat**: Berencana menggunakan live migration atau snapshots

**Rekomendasi**:
- **Pengguna pertama kali**: Biarkan kedua opsi dinonaktifkan, gunakan 1 vCPU dan 1 GiB RAM
- **Pengembangan**: 2 vCPU dan 2 GiB RAM, opsi dinonaktifkan
- **Produksi**: Berdasarkan kebutuhan aplikasi

**Tips**: Mulai dengan yang kecil. Pantau penggunaan aktual dan tingkatkan jika diperlukan.

Klik **Next** untuk memilih sumber boot.

---

## Langkah 5: Sumber Boot

Pilih image kernel dan rootfs untuk VM Anda:

![Image: Screenshot of Boot Source selection](/images/vm/vm-step4-boot.png)

### Image Kernel (Wajib)

Pilih kernel Linux dari dropdown:

![Image: Kernel dropdown selection](/images/vm/vm-kernel-dropdown.png)

- **vmlinux-5.10.fc.bin** - Kernel standar yang dioptimalkan untuk Firecracker
- Kompatibel dengan sebagian besar distribusi (Ubuntu, Alpine, Debian)

**Jika dropdown kosong**: Anda perlu mengunggah kernel terlebih dahulu. Lihat [Image Registry](../registry/upload-images/).

Kernel pertama yang tersedia akan dipilih secara otomatis.

### Image Rootfs (Wajib)

Pilih sistem operasi dari dropdown:

![Image: Dropdown showing available rootfs images](/images/vm/vm-rootfs-selection.png)

**Pilihan populer**:

- **Alpine Linux 3.18** (Direkomendasikan untuk pemula)
  - Ukuran: ~100-200 MB
  - Waktu boot: <1 detik
  - Terbaik untuk: Pengujian, container, layanan ringan
  - Package manager: apk

- **Ubuntu 22.04**
  - Ukuran: ~2-5 GB
  - Waktu boot: ~2 detik
  - Terbaik untuk: Pengembangan, aplikasi produksi
  - Package manager: apt

- **Debian 12**
  - Ukuran: ~1-3 GB
  - Berfokus pada stabilitas
  - Terbaik untuk: Server, proyek jangka panjang

**Tips**: Jika Anda tidak yakin mana yang harus dipilih, mulailah dengan **Alpine Linux** untuk unduhan dan waktu boot yang lebih cepat.

Rootfs pertama yang tersedia akan dipilih secara otomatis.

### Jalur Initrd (Opsional)

![Image: Initrd path input field](/images/vm/vm-initrd-input.png)

Biarkan kosong kecuali Anda memerlukan initial ramdisk khusus.

- Digunakan untuk konfigurasi boot lanjutan
- Sebagian besar pengguna dapat melewati kolom ini

### Argumen Boot (Opsional)

![Image: Boot arguments input field](/images/vm/vm-bootargs-input.png)

Biarkan kosong untuk menggunakan parameter boot kernel default.

- Pengguna lanjutan dapat menambahkan parameter kernel kustom
- Contoh: `console=ttyS0 reboot=k panic=1`

Klik **Next** untuk konfigurasi jaringan.

---

## Langkah 6: Jaringan

Konfigurasikan pengaturan jaringan untuk VM Anda:

![Image: Screenshot of Network step](/images/vm/vm-step5-network.png)

### Aktifkan Jaringan

Pertama, tentukan apakah akan mengaktifkan jaringan:

![Image: Enable networking checkbox](/images/vm/vm-network-enable.png)

- ☑ **Enable networking**
- **Default**: Diaktifkan (dicentang)

**Kapan menonaktifkan**:
- VM yang sepenuhnya terisolasi untuk pengujian keamanan
- Tidak diperlukan akses jaringan

**Sebagian besar pengguna sebaiknya tetap mengaktifkan ini.**

### Nama Perangkat Host

![Image: Host device input field](/images/vm/vm-host-device.png)

Tentukan nama perangkat TAP di host:

- **Default**: `tap0`
- Sistem akan membuat perangkat TAP ini
- Beberapa VM dapat berbagi bridge yang sama tetapi memerlukan IP yang unik

**Tips**: Untuk sebagian besar kasus, gunakan default `tap0`.

### Alamat MAC Guest

![Image: Guest MAC address input with Generate button](/images/vm/vm-guest-mac.png)

Tetapkan alamat MAC untuk antarmuka jaringan VM:

- **Biarkan kosong** untuk pembuatan MAC otomatis (direkomendasikan)
- **ATAU klik "Generate"** untuk membuat alamat MAC acak
- **ATAU masukkan secara manual** (format: `AA:FC:00:00:00:01`)

**Contoh**:
- Auto-generated: Sistem menetapkan MAC yang unik
- Generated: `aa:bb:cc:dd:ee:ff` (klik tombol Generate)
- Manual: `AA:FC:00:00:00:01`

**Rekomendasi**: Biarkan kosong atau klik Generate untuk menghindari konflik MAC.

Klik **Next** untuk meninjau konfigurasi Anda.

---

## Langkah 7: Tinjau & Buat

Tinjau konfigurasi VM Anda sebelum membuat:

![Image: Screenshot of Review step](/images/vm/vm-step6-review.png)

Halaman tinjauan menampilkan ringkasan semua pengaturan Anda yang diorganisir ke dalam beberapa bagian:

### Informasi Dasar

![Image: Basic info summary card](/images/vm/vm-review-basic.png)

Tinjau:
- **Name**: Nama VM Anda
- **Owner**: Nama pemilik
- **Environment**: Development/Staging/Production
- **Description**: Deskripsi Anda (jika diberikan)

### Konfigurasi Mesin

![Image: Machine config summary card](/images/vm/vm-review-machine.png)

Verifikasi:
- **vCPU**: Jumlah CPU virtual
- **Memory**: RAM dalam MiB
- **SMT**: Enabled atau Disabled
- **Track Dirty Pages**: Yes atau No

### Sumber Boot

![Image: Boot source summary card](/images/vm/vm-review-boot.png)

Konfirmasi:
- **Kernel**: Jalur ke image kernel
- **Rootfs**: Jalur ke image rootfs

### Jaringan

![Image: Network summary card](/images/vm/vm-review-network.png)

Periksa:
- **Enabled**: Yes atau No
- **Host Device**: Nama perangkat TAP (mis., tap0)
- **Guest MAC**: Alamat MAC (atau "—" jika auto-generated)

### Lakukan Perubahan

Jika Anda perlu mengubah pengaturan:

1. Klik tombol **Previous** di bagian bawah
2. Navigasi ke langkah yang ingin Anda ubah
3. Perbarui nilainya
4. Klik **Next** untuk kembali ke Tinjauan

### Buat VM

Ketika semuanya sudah benar:

![Image: Create VM button highlighted](/images/vm/vm-create-button-review.png)

Klik tombol **Create VM** untuk melanjutkan.

---

## Proses Pembuatan VM

Sistem akan membuat VM Anda sekarang:

![Image: Loading spinner with "Creating VM..." message](/images/vm/vm-creating.png)

**Yang terjadi di balik layar:**

1. ✓ Sumber daya dialokasikan di host
2. ✓ Firecracker VMM dikonfigurasi
3. ✓ Kernel dan rootfs dipasang
4. ✓ Antarmuka jaringan dibuat
5. ✓ VM dijalankan

**Waktu**: Biasanya selesai dalam **1-2 detik**!

---

## Berhasil!

VM Anda sekarang berjalan:

![Image: Success notification and VM detail page](/images/vm/vm-created-success.png)

Anda akan melihat:

- ✅ **Status**: Running (indikator hijau)
- ✅ **IP Address**: Ditetapkan oleh DHCP (mis., 192.168.1.100)
- ✅ **Uptime**: Menghitung dari 00:00:01
- ✅ **Resource Usage**: Grafik CPU dan memori

---

## Verifikasi VM Anda

### Periksa Status

Di halaman detail VM, verifikasi:

![Image: VM detail page showing running state](/images/vm/vm-detail-running.png)

- Status menampilkan **Running** dengan indikator hijau
- Alamat IP ditampilkan
- Grafik penggunaan CPU menunjukkan aktivitas
- Penggunaan memori dalam batas yang ditetapkan

### Uji Akses Console

Klik tab **Terminal**:

![Image: Terminal tab highlighted](/images/vm/vm-console-tab.png)

Anda seharusnya melihat prompt terminal:

```
Welcome to Alpine Linux 3.18
alpine login: root
Password:
```

**Login**:
- Jika menggunakan SSH key: Login sebagai `root` (mungkin otomatis)
- Jika mengatur kata sandi: Masukkan kata sandi root Anda

![Image: Successful console login](/images/vm/vm-console-logged-in.png)

### Uji Jaringan

Dari console, verifikasi konektivitas jaringan:

```bash
# Check IP address
ip addr show eth0

# Test internet connectivity
ping -c 3 google.com

# Check DNS resolution
nslookup github.com
```

![Image: Successful ping command](/images/vm/vm-network-test.png)

Jika semua pengujian berhasil, **VM Anda telah beroperasi sepenuhnya**!

---

## Mulai Cepat dari Template

Untuk pembuatan VM yang lebih cepat, gunakan template:

1. Pergi ke halaman **Templates**
2. Temukan template (mis., "Ubuntu 22.04 Base")
3. Klik **Deploy**
4. Masukkan nama VM
5. Klik **Deploy**

![Image: Deploy from template dialog](/images/vm/template-deploy.png)

VM dibuat secara instan dengan pengaturan yang telah dikonfigurasi sebelumnya!

---

## Pemecahan Masalah

### Masalah: Tidak Ada Image yang Tersedia

**Masalah**: Dropdown Kernel atau Rootfs kosong

**Solusi**:
1. Pergi ke halaman **Image Registry**
2. Unggah image yang diperlukan (lihat [Upload Images](../registry/upload-images/))
3. Kembali ke pembuatan VM

---

### Masalah: Sumber Daya Tidak Cukup

**Masalah**: Pesan kesalahan "Insufficient resources available"

**Solusi**:
- Kurangi alokasi CPU atau memori
- Hentikan VM yang tidak digunakan untuk membebaskan sumber daya
- Hubungi administrator untuk menambah kapasitas

---

### Masalah: VM Tersangkut di Status "Creating"

**Masalah**: VM menampilkan "Creating" lebih dari 30 detik

**Solusi**:
1. Refresh halaman
2. Periksa halaman **Hosts** untuk memverifikasi agent online
3. Jika masih tersangkut, hapus VM dan coba lagi
4. Hubungi administrator jika masalah berlanjut

---

### Masalah: Tidak Bisa Mengakses VM Console

**Masalah**: Console menampilkan "Connection failed"

**Solusi**:
- Verifikasi status VM adalah "Running"
- Periksa browser console untuk kesalahan
- Coba browser yang berbeda
- Pastikan koneksi WebSocket tidak diblokir oleh firewall

---

## Langkah Berikutnya

Setelah VM Anda dibuat:

- **[Akses VM](access-vm/)** - Pelajari cara terhubung melalui SSH
- **[Kelola VM](manage-vm/)** - Operasi start, stop, pause
- **[Pemantauan](monitoring/)** - Lihat metrik performa
- **[Backup & Snapshot](backup-snapshot/)** - Lindungi data Anda

---

## Praktik Terbaik

**Konvensi Penamaan**:
```
<environment>-<purpose>-<number>
Contoh:
  prod-web-01, prod-web-02
  dev-alice-ubuntu
  test-backend-api
  staging-database
```

**Ukuran Sumber Daya**:
- Mulai dengan sumber daya minimal
- Pantau penggunaan aktual (lihat [Pemantauan](monitoring/))
- Tingkatkan hanya saat diperlukan
- Jangan over-allocate — membuang sumber daya

**Keamanan**:
- ✅ Gunakan SSH key sebagai pengganti kata sandi
- ✅ Gunakan kata sandi yang kuat dan unik jika diperlukan
- ✅ Nonaktifkan login dengan kata sandi di produksi
- ✅ Perbarui VM secara berkala dengan patch keamanan

**Organisasi**:
- Gunakan template untuk konfigurasi yang berulang
- Tambahkan deskripsi yang informatif
- Ikuti konvensi penamaan yang konsisten
- Dokumentasikan tujuan dan pemilik VM
