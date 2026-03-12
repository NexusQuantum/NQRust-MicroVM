+++
title = "Kelola Template"
description = "Deploy VM, edit, dan hapus template"
weight = 37
date = 2025-01-08
+++

Pelajari cara mengelola template VM Anda — deploy VM dari template, edit konfigurasi template, dan hapus template yang tidak digunakan.

---

## Mengakses Template

### Navigasi ke Halaman Templates

![Image: Templates navigation](/images/templates/nav-templates.png)

Klik **"Templates"** di sidebar untuk mengakses halaman Templates.

### Tata Letak Halaman Templates

![Image: Templates page](/images/templates/page-layout.png)

Halaman menampilkan:
- **Header** dengan tombol buat
- **Jumlah template** di header kartu
- **Kartu template** dalam tata letak grid
- **Detail template** (CPU, RAM, tanggal)
- **Tombol Deploy** di setiap kartu

---

## Daftar Template

### Informasi Kartu Template

Setiap kartu template menampilkan:

![Image: Template card](/images/templates/card-layout.png)

**Detail template**:
- **Nama template** - di bagian atas
- **Badge template** - menunjukkan ini adalah template
- **Jumlah vCPU** - jumlah CPU virtual
- **RAM (MiB)** - alokasi memori
- **Tanggal dibuat** - kapan template dibuat
- **Tombol Deploy** - untuk membuat VM

**Contoh kartu**:
```
Ubuntu 22.04 Base              [Template]
  CPU:  2 vCPU
  RAM:  2048 MiB
  Dibuat: Jan 8, 2025
                     [Deploy VM]
```

---

### Menelusuri Template

**Tata letak grid**:
- Template ditampilkan dalam grid responsif
- 1 kolom di mobile
- 2 kolom di tablet
- 3 kolom di desktop

**Pengurutan**:
- Saat ini diurutkan berdasarkan tanggal pembuatan (terbaru lebih dulu)
- Mendatang: Urutkan berdasarkan nama, ukuran sumber daya, penggunaan

**Pencarian**:
- Fitur mendatang: Cari template berdasarkan nama
- Fitur mendatang: Filter berdasarkan kebutuhan sumber daya

---

## Deploy VM dari Template

Men-deploy VM membuat instance VM baru dengan konfigurasi template.

### Langkah 1: Pilih Template

Temukan template yang ingin Anda deploy:

![Image: Template card with deploy button](/images/templates/select-template.png)

Klik tombol **"Deploy VM"** pada kartu template.

---

### Langkah 2: Masukkan Nama VM

Dialog deploy terbuka:

![Image: Deploy VM dialog](/images/templates/deploy-dialog.png)

**Dialog menampilkan**:
- Nama template yang sedang di-deploy
- Ringkasan konfigurasi template (vCPU, RAM)
- Field input nama VM
- Tombol Cancel dan Deploy

**Masukkan nama VM yang unik**:
- Nama yang baik: `web-server-01`, `dev-env-alice`, `test-vm-123`
- Hindari: nama generik seperti `vm1`, `test`

**Saran yang dibuat otomatis**:
- Dialog mengisi otomatis: `{nama-template}-{angka-acak}`
- Contoh: `Ubuntu 22.04 Base-1234`
- Anda dapat mengeditnya sesuai keinginan

---

### Langkah 3: Tinjau Konfigurasi

Sebelum men-deploy, periksa konfigurasi template:

![Image: Configuration summary](/images/templates/deploy-config.png)
![Image: Configuration summary](/images/templates/deploy-config-2.png)

**Verifikasi**:
- Jumlah vCPU sudah benar
- Alokasi RAM sesuai
- Anda memiliki sumber daya host yang cukup

**Konfigurasi template disalin ke VM baru**:
- Jumlah vCPU yang sama
- Alokasi memori yang sama
- Kernel image yang sama
- Rootfs image yang sama

**Yang berbeda**:
- VM mendapatkan ID unik
- VM mendapatkan nama yang Anda tentukan
- VM akan memiliki siklus hidupnya sendiri (independen dari template)

---

### Langkah 4: Deploy

Klik tombol **"Deploy VM"**:

![Image: Deploy button](/images/templates/deploy-submit.png)

**Yang terjadi**:
1. ✅ Validasi nama VM
2. ✅ Konfigurasi template disalin
3. ✅ Panggilan API: `POST /v1/templates/{id}/instantiate`
4. ✅ VM dibuat di database
5. ✅ VM dimulai secara otomatis
6. ✅ Notifikasi sukses ditampilkan
7. ✅ Diarahkan ke halaman detail VM

**Waktu deployment**: ~30-60 detik total
- Pembuatan VM: ~5 detik
- Boot VM: ~25-55 detik

---

### Langkah 5: Verifikasi VM

Setelah deployment, Anda diarahkan ke halaman detail VM:

![Image: VM detail page](/images/templates/deployed-vm.png)

**Periksa status VM**:
- Status seharusnya maju: Creating → Booting → Running
- IP Guest seharusnya muncul setelah boot
- Metrik VM seharusnya menampilkan penggunaan CPU/memori

**Akses VM Anda**:
- Buka tab Shell untuk akses terminal
- Lihat Metrics untuk pemantauan performa
- Periksa Config untuk melihat pengaturan template yang diwarisi

---

## Tahapan Deployment

### Tahap 1: Creating (5-10 detik)

![Image: Creating state](/images/templates/deploy-creating.png)

**Yang sedang terjadi**:
- Record VM dibuat di database
- VM Firecracker sedang di-provision
- Host agent menerima permintaan buat
- Image kernel dan rootfs sedang disiapkan

**Badge status**: Kuning (Creating)

---

### Tahap 2: Booting (20-50 detik)

![Image: Booting state](/images/templates/deploy-booting.png)

**Yang sedang terjadi**:
- microVM Firecracker dimulai
- Kernel dimuat
- Rootfs di-mount
- Sistem operasi diinisialisasi

**Badge status**: Abu-abu (Booting)

---

### Tahap 3: Running (setelah ~30-60 detik)

![Image: Running state](/images/templates/deploy-running.png)

**Yang sedang terjadi**:
- VM sudah sepenuhnya boot
- Guest agent melaporkan metrik
- IP Guest ditetapkan
- VM siap digunakan

**Badge status**: Hijau (Running)

**Anda sekarang dapat**:
- Mengakses shell
- Melihat metrik
- Terhubung melalui SSH (jika dikonfigurasi)
- Men-deploy aplikasi

---

## Edit Template

Perbarui konfigurasi template untuk mengubah deployment VM mendatang.

### Kapan Harus Mengedit

**Edit template saat**:
- Kebutuhan sumber daya berubah (butuh lebih banyak CPU/RAM)
- Upgrade ke versi kernel yang lebih baru
- Beralih ke rootfs image yang berbeda
- Memperbaiki konfigurasi yang salah

**Catatan**: Mengedit template TIDAK memengaruhi VM yang sudah ada yang dibuat darinya. Hanya deployment mendatang yang menggunakan pengaturan baru.

---

### Cara Mengedit (Fitur Mendatang)

**Saat ini**: Tidak tersedia di UI (backend API sudah siap)

**Segera hadir**:
1. Klik kartu template untuk membuka detail
2. Klik tombol "Edit"
3. Ubah vCPU, memori, atau image
4. Simpan perubahan

**API sudah tersedia sekarang**:
```bash
curl -X PUT http://localhost:18080/v1/templates/{id} \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Ubuntu 22.04 Base Updated",
    "spec": {
      "vcpu": 4,
      "mem_mib": 4096,
      "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
      "rootfs_path": "/srv/images/ubuntu-22.04.ext4"
    }
  }'
```

---

## Hapus Template

Hapus template yang tidak lagi Anda butuhkan.

### Kapan Harus Menghapus

**Hapus template saat**:
- Template sudah usang
- Konfigurasi ini tidak lagi digunakan
- Ingin membersihkan daftar template
- Membuat kesalahan saat membuat template

**Aman untuk dihapus**:
- ✅ VM yang dibuat dari template tetap berfungsi
- ✅ Template dihapus dari database
- ✅ `template_id` di VM diatur ke NULL
- ✅ Tidak ada kehilangan data untuk VM yang di-deploy

---

### Cara Menghapus (Fitur Mendatang)

**Saat ini**: Tidak tersedia di UI (backend API sudah siap)

**Segera hadir**:
1. Klik kartu template
2. Klik tombol "Delete"
3. Konfirmasi penghapusan di dialog
4. Template dihapus dari daftar

**Dialog konfirmasi akan menanyakan**:
```
Delete Template?

Are you sure you want to delete "Ubuntu 22.04 Base"?

This will:
- Remove template from the database
- NOT affect VMs created from this template
- Cannot be undone

[Cancel]  [Delete Template]
```

**API sudah tersedia sekarang**:
```bash
curl -X DELETE http://localhost:18080/v1/templates/{template-id}
```

---

## Lihat Detail Template (Fitur Mendatang)

### Tampilan Detail Template

**Segera hadir**: Klik kartu template untuk melihat detail lengkap.

**Akan menampilkan**:
- ID Template (UUID)
- Nama lengkap
- Spesifikasi lengkap:
  - Jumlah vCPU
  - Memori (MiB)
  - Image kernel (path atau ID)
  - Image rootfs (path atau ID)
- Timestamp pembuatan
- Timestamp pembaruan
- Jumlah VM yang di-deploy dari template ini
- Daftar VM yang menggunakan template ini

---

## Pelacakan Penggunaan Template

### VM dari Template

Setiap VM melacak template mana yang digunakan untuk membuatnya:

**Dalam detail VM**:
- ID Template disimpan di field `vm.template_id`
- Dapat melihat template mana yang digunakan
- Mendatang: Tautan kembali ke template

**Penghapusan template**:
- Saat template dihapus, `vm.template_id` diatur ke NULL
- VM terus berfungsi secara normal
- Hanya kehilangan referensi ke template

---

## Tugas Umum

### Tugas: Deploy Beberapa VM dari Template yang Sama

**Kasus penggunaan**: Buat 3 web server dengan konfigurasi identik

**Langkah-Langkah**:
1. Temukan template "Web Server"
2. Klik "Deploy VM"
3. Nama: `web-server-01`
4. Deploy dan tunggu
5. Kembali ke halaman Templates
6. Klik "Deploy VM" lagi
7. Nama: `web-server-02`
8. Ulangi untuk `web-server-03`

**Hasil**: 3 VM identik siap untuk load balancing

---

### Tugas: Upgrade Semua Lingkungan Dev

**Skenario**: Versi kernel baru tersedia

**Langkah-Langkah**:
1. Uji kernel baru dengan VM manual terlebih dahulu
2. Verifikasi berfungsi dengan benar
3. Edit template "Dev Environment"
4. Perbarui path kernel ke versi baru
5. Deployment mendatang menggunakan kernel baru
6. Perbarui VM yang ada secara bertahap

**Catatan**: VM yang ada tidak diperbarui secara otomatis

---

### Tugas: Standardisasi Konfigurasi Production

**Skenario**: Pastikan semua VM production memiliki spesifikasi yang sama

**Langkah-Langkah**:
1. Buat template "Production Standard"
2. Atur: 4 vCPU, 8192 MiB RAM
3. Gunakan rootfs Ubuntu 22.04 LTS
4. Deploy semua VM production baru dari template ini
5. Nonaktifkan VM non-standar secara bertahap

**Manfaat**: Konsistensi terjamin di seluruh production

---

### Tugas: Bersihkan Template Lama

**Skenario**: Terlalu banyak template yang tidak digunakan

**Langkah-Langkah**:
1. Daftarkan semua template
2. Periksa tanggal pembuatan
3. Periksa berapa banyak VM yang di-deploy
4. Hapus template dengan:
   - Tanggal pembuatan lama (>6 bulan)
   - Nol VM yang di-deploy
   - Konfigurasi yang sudah usang

**Pertahankan**: Template yang aktif digunakan atau diperlukan di masa mendatang

---

## Template vs Pembuatan VM Manual

### Kapan Menggunakan Template

✅ **Gunakan template saat**:
- Men-deploy konfigurasi yang sama berkali-kali
- Menstandarisasi lingkungan development tim
- Deployment cepat adalah prioritas
- Ingin mendokumentasikan konfigurasi standar

**Manfaat**:
- Deployment lebih cepat (tidak perlu konfigurasi)
- Konsistensi terjamin
- Mudah direplikasi
- Infrastruktur yang mendokumentasikan dirinya sendiri

---

### Kapan Membuat VM Secara Manual

✅ **Gunakan pembuatan manual saat**:
- VM sekali pakai dengan konfigurasi unik
- Bereksperimen dengan pengaturan yang berbeda
- Mempelajari platform
- Memerlukan setup jaringan/penyimpanan kustom

**Manfaat**:
- Kontrol penuh atas setiap pengaturan
- Dapat mengonfigurasi opsi jaringan lanjutan
- Menambahkan beberapa drive
- Mengatur variabel lingkungan kustom

---

## Praktik Terbaik

### Manajemen Template

**1. Konvensi Penamaan**
Gunakan penamaan yang konsisten:
```
{OS} - {Tujuan} - {Ukuran}

Contoh:
- Ubuntu 22.04 - Dev - 2vCPU
- Alpine - Test - 1vCPU
- Ubuntu 22.04 - Prod - 4vCPU
```

**2. Pembaruan Berkala**
- Tinjau template setiap kuartal
- Perbarui ke versi kernel terbaru
- Segarkan rootfs dengan patch keamanan
- Hapus template yang usang

**3. Dokumentasi**
Catat:
- Perangkat lunak apa yang sudah terpasang di rootfs
- Kasus penggunaan yang dimaksudkan untuk setiap template
- Jumlah deployment dan VM aktif
- Tanggal pembaruan terakhir

---

### Deployment VM

**1. Nama VM yang Deskriptif**
Saat men-deploy, gunakan nama yang menunjukkan:
- Tujuan: `web-server`, `db-primary`, `cache`
- Lingkungan: `dev`, `staging`, `prod`
- Nomor instance: `01`, `02`, `03`

**Contoh**: `prod-web-server-01`

**2. Perencanaan Sumber Daya**
Sebelum deployment massal:
- Periksa CPU/RAM host yang tersedia
- Hitung total sumber daya yang diperlukan
- Sisakan buffer untuk OS host (10-20%)

**Contoh**:
- Host memiliki 32 GB RAM
- Setiap VM membutuhkan 2 GB
- Deploy maksimal 12-14 VM (sisakan buffer)

**3. Rollout Bertahap**
Untuk perubahan production:
1. Deploy satu VM dari template baru
2. Uji secara menyeluruh
3. Jika berfungsi, deploy lebih banyak VM
4. Migrasi secara bertahap dari VM lama

---

### Organisasi Template

**Buat Kategori Template**:

**Template Development**:
- Sumber daya ringan
- Prioritas deployment cepat
- Image terbaru/eksperimental diperbolehkan

**Template Staging**:
- Cocokkan sumber daya production
- Image yang sama dengan production
- Untuk pengujian sebelum prod

**Template Production**:
- Sumber daya tinggi
- Image yang stabil dan telah diuji
- Terdokumentasi dengan baik
- Dikontrol versinya

---

## Pemecahan Masalah

### Masalah: VM Gagal Di-Deploy

**Gejala**:
- Klik "Deploy VM"
- VM menampilkan status "Error"
- Tidak pernah mencapai "Running"

**Kemungkinan penyebab**:
1. File kernel tidak ditemukan
2. File rootfs tidak ditemukan
3. Sumber daya host tidak mencukupi
4. File image rusak

**Solusi**:
1. Periksa path kernel template ada:
   ```bash
   ls -lh /srv/images/vmlinux-5.10.fc.bin
   ```

2. Periksa path rootfs template ada:
   ```bash
   ls -lh /srv/images/ubuntu-22.04.ext4
   ```

3. Periksa sumber daya host:
   ```bash
   free -h  # Periksa RAM
   nproc    # Periksa CPU
   ```

4. Periksa log manager untuk detail:
   ```bash
   journalctl -u manager -f
   ```

---

### Masalah: Tidak Dapat Menemukan Template

**Gejala**:
- Daftar template kosong
- Template yang Anda buat hilang

**Kemungkinan penyebab**:
1. Halaman Templates tidak dimuat
2. Masalah koneksi API
3. Template telah dihapus
4. Lingkungan/database yang salah

**Solusi**:
1. Muat ulang halaman
2. Periksa konsol browser untuk error
3. Verifikasi manager API sedang berjalan:
   ```bash
   curl http://localhost:18080/v1/templates
   ```
4. Periksa database untuk template:
   ```bash
   psql $DATABASE_URL -c "SELECT id, name FROM template;"
   ```

---

### Masalah: VM yang Di-Deploy Memiliki Konfigurasi yang Salah

**Gejala**:
- VM memiliki CPU/RAM yang berbeda dari yang diharapkan
- VM menggunakan image yang salah

**Kemungkinan penyebab**:
1. Template diedit setelah deployment dimulai
2. Melihat VM yang salah
3. Desync database

**Solusi**:
1. Periksa halaman detail VM untuk konfigurasi aktual
2. Bandingkan dengan konfigurasi template
3. Jika salah, hapus VM dan deploy ulang
4. Verifikasi pengaturan template sebelum men-deploy

---

### Masalah: Tombol Deploy Dinonaktifkan

**Gejala**:
- Tidak dapat mengklik tombol "Deploy VM"
- Tombol berwarna abu-abu

**Kemungkinan penyebab**:
1. Host offline
2. Koneksi API terputus
3. Template memiliki konfigurasi yang tidak valid
4. Bug UI

**Solusi**:
1. Periksa konsol browser untuk error
2. Verifikasi manager sedang berjalan
3. Muat ulang halaman
4. Coba deploy template yang berbeda
5. Periksa status host di halaman Hosts

---

## Tips Performa

### Deployment Cepat

**Untuk deployment VM tercepat**:
1. Gunakan rootfs Alpine Linux (lebih kecil, boot lebih cepat)
2. Gunakan sumber daya minimum yang diperlukan (1 vCPU, 512 MiB)
3. Pra-muat image di host
4. Deploy saat beban host rendah

**Deployment Alpine**: ~15-20 detik
**Deployment Ubuntu**: ~30-60 detik

---

### Optimasi Sumber Daya

**Optimalkan penggunaan host**:
1. Ukuran template yang tepat (jangan over-alokasi)
2. Gunakan batas memori yang sesuai dengan beban kerja
3. Monitor penggunaan aktual dan sesuaikan template
4. Hentikan VM yang tidak digunakan

**Audit template**:
- Periksa penggunaan CPU/RAM aktual VM yang di-deploy
- Perbarui template agar sesuai dengan kebutuhan nyata
- Hapus template yang over-provisioned

---

## Referensi Cepat

### Aksi Template

| Aksi | Langkah | Status |
|--------|-------|--------|
| Deploy VM | Klik "Deploy VM" → Masukkan nama → Deploy | ✅ Tersedia |
| Lihat detail | Klik kartu template | 🚧 Segera hadir |
| Edit template | Detail template → Edit | 🚧 UI segera hadir (API siap) |
| Hapus template | Detail template → Delete | 🚧 UI segera hadir (API siap) |
| Periksa penggunaan | Detail template → Tab VMs | 🚧 Segera hadir |

---

### Pintasan Keyboard

| Aksi | Pintasan |
|--------|----------|
| Deploy VM | Klik tombol Deploy |
| Tutup dialog | Esc |
| Kirim formulir | Enter |
| Navigasi template | Tombol panah (mendatang) |

---

## Langkah Selanjutnya

- **[Buat Template](create-template/)** - Buat lebih banyak template
- **[Ikhtisar Template](./)** - Pelajari tentang template
- **[Manajemen VM](/docs/vm/manage-vm/)** - Kelola VM yang di-deploy
- **[Pemantauan VM](/docs/vm/monitoring/)** - Pantau performa VM

---

## Referensi API

Untuk pengguna mahir dan otomatisasi:

**Daftarkan template**:
```bash
GET /v1/templates
```

**Dapatkan detail template**:
```bash
GET /v1/templates/{id}
```

**Deploy VM dari template**:
```bash
POST /v1/templates/{id}/instantiate
Body: {"name": "my-vm-name"}
```

**Perbarui template**:
```bash
PUT /v1/templates/{id}
Body: {"name": "...", "spec": {...}}
```

**Hapus template**:
```bash
DELETE /v1/templates/{id}
```

Lihat dokumentasi API lengkap di `/api-docs/openapi.yaml`
